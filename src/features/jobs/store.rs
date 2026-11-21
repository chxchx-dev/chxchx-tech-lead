//! Jobs on disk: a snapshot (`jobs.json`) plus a journal of changes (`journal.jsonl`), and one
//! event file per job (`events/<id>.jsonl`).
//!
//! Why a journal: a change is one appended line, so a crash loses at most that line (it is
//! skipped when read back). The journal is folded into the snapshot, written atomically, when the
//! store opens and every `COMPACT_AFTER` changes.
//!
//! Only one process owns a store (`owner.lock`). Opening it marks every job that was still queued
//! or running as `interrupted`: its owner stopped and nobody is going to finish it. Queued jobs are
//! not resumed on their own, because an agent must not start acting while no one is watching.

use super::lock::OwnerLock;
use crate::features::error_cache::redact;
use chxchx_protocol::{
    ErrorCode, IdempotencyKey, Job, JobError, JobEvent, JobEventBody, JobId, JobSpec, JobState,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Finished jobs kept (with their events); older ones are forgotten.
pub const MAX_KEPT_FINISHED: usize = 200;
/// Events kept per job. Past it, progress and agent events are dropped; states and errors are not.
pub const MAX_EVENTS_PER_JOB: u64 = 20_000;
const MAX_TEXT_CHARS: usize = 4_000;
const COMPACT_AFTER: usize = 1_000;

#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    jobs: Vec<Job>,
}

pub struct JobStore {
    dir: PathBuf,
    jobs: BTreeMap<JobId, Job>,
    next_seq: BTreeMap<JobId, u64>,
    journal: File,
    journal_lines: usize,
    counter: u64,
    /// What could not be read back, for the owner to report.
    pub warnings: Vec<String>,
    _lock: OwnerLock,
}

/// Where the jobs of this user live.
pub fn default_dir() -> Option<PathBuf> {
    crate::features::project_profiles::application_home().map(|home| home.join("jobs"))
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

/// The jobs as they are on disk, without owning the store (for `jobs list` while a node runs).
pub fn read(dir: &Path) -> Result<Vec<Job>, String> {
    let (jobs, _) = load(dir)?;
    Ok(newest_first(jobs.into_values()))
}

/// The events of a job after `after_seq`, without owning the store.
pub fn read_events(dir: &Path, id: &JobId, after_seq: u64) -> Vec<JobEvent> {
    let text = fs::read_to_string(events_path(dir, id)).unwrap_or_default();
    text.lines()
        .filter_map(|line| serde_json::from_str::<JobEvent>(line).ok())
        .filter(|event| event.seq > after_seq)
        .collect()
}

impl JobStore {
    pub fn open(dir: &Path) -> Result<Self, String> {
        create_private_dir(dir)?;
        create_private_dir(&dir.join("events"))?;
        let lock = OwnerLock::acquire(&dir.join("owner.lock"))?;
        let (jobs, warnings) = load(dir)?;
        let journal = open_journal(dir, false)?;
        let mut store = Self {
            dir: dir.to_owned(),
            next_seq: BTreeMap::new(),
            jobs,
            journal,
            journal_lines: 0,
            counter: 0,
            warnings,
            _lock: lock,
        };
        for id in store.jobs.keys().cloned().collect::<Vec<_>>() {
            let last = read_events(dir, &id, 0).last().map_or(0, |event| event.seq);
            store.next_seq.insert(id, last + 1);
        }
        let unfinished: Vec<JobId> = store
            .jobs
            .values()
            .filter(|job| !job.state.is_terminal())
            .map(|job| job.id.clone())
            .collect();
        for id in unfinished {
            let error = JobError::new(
                ErrorCode::Interrupted,
                "El proceso que lo ejecutaba se detuvo antes de terminar",
            );
            store.transition(&id, JobState::Interrupted, None, Some(error))?;
        }
        store.compact()?;
        Ok(store)
    }

    pub fn get(&self, id: &JobId) -> Option<&Job> {
        self.jobs.get(id)
    }

    pub fn list(&self) -> Vec<Job> {
        newest_first(self.jobs.values().cloned())
    }

    pub fn events_since(&self, id: &JobId, after_seq: u64) -> Vec<JobEvent> {
        read_events(&self.dir, id, after_seq)
    }

    /// A new queued job, or the one already created with the same key and the same request.
    pub fn create(
        &mut self,
        spec: JobSpec,
        key: Option<IdempotencyKey>,
    ) -> Result<(Job, Vec<JobEvent>), JobError> {
        spec.validate()?;
        if let Some(key) = &key
            && let Some(existing) = self
                .jobs
                .values()
                .find(|job| job.idempotency_key.as_ref() == Some(key))
        {
            return if existing.spec == spec {
                Ok((existing.clone(), Vec::new()))
            } else {
                Err(JobError::new(
                    ErrorCode::Conflict,
                    "Esa clave de idempotencia ya se usó para otra petición",
                ))
            };
        }
        let id = self.new_id();
        let now = now_ms();
        let job = Job {
            id: id.clone(),
            spec,
            state: JobState::Queued,
            created_ms: now,
            updated_ms: now,
            idempotency_key: key,
            summary: None,
            error: None,
        };
        self.write_job(&job).map_err(internal)?;
        self.jobs.insert(id.clone(), job.clone());
        self.next_seq.insert(id.clone(), 1);
        let event = self.append_event(
            &id,
            JobEventBody::State {
                state: JobState::Queued,
            },
        );
        Ok((job, event.into_iter().collect()))
    }

    /// Moves a job to its next state; an illegal move is refused and changes nothing.
    pub fn transition(
        &mut self,
        id: &JobId,
        next: JobState,
        summary: Option<String>,
        error: Option<JobError>,
    ) -> Result<(Job, Vec<JobEvent>), String> {
        let mut job = self
            .jobs
            .get(id)
            .cloned()
            .ok_or_else(|| format!("No existe el trabajo {id}"))?;
        if !job.state.can_become(next) {
            return Err(format!(
                "El trabajo {id} no puede pasar de {:?} a {:?}",
                job.state, next
            ));
        }
        job.state = next;
        job.updated_ms = now_ms().max(job.updated_ms);
        job.summary = summary.map(|text| clean(&text)).or(job.summary);
        let error = error.map(|error| JobError::new(error.code, clean(&error.message)));
        job.error = error.clone().or(job.error);
        self.write_job(&job)?;
        self.jobs.insert(id.clone(), job.clone());
        let mut events = Vec::new();
        if let Some(error) = error {
            events.extend(self.append_event(id, JobEventBody::Error { error }));
        }
        events.extend(self.append_event(id, JobEventBody::State { state: next }));
        if self.journal_lines >= COMPACT_AFTER {
            self.compact()?;
        }
        Ok((job, events))
    }

    /// Records an event; `None` when it was dropped (past the cap, or the job is unknown).
    pub fn append_event(&mut self, id: &JobId, body: JobEventBody) -> Option<JobEvent> {
        let seq = self.next_seq.get(id).copied()?;
        let essential = matches!(
            body,
            JobEventBody::State { .. } | JobEventBody::Error { .. }
        );
        if seq > MAX_EVENTS_PER_JOB && !essential {
            return None;
        }
        let event = JobEvent {
            job: id.clone(),
            seq,
            at_ms: now_ms(),
            body: redact_body(body),
        };
        let line = serde_json::to_string(&event).ok()?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(events_path(&self.dir, id))
            .ok()?;
        file.write_all(format!("{line}\n").as_bytes()).ok()?;
        self.next_seq.insert(id.clone(), seq + 1);
        Some(event)
    }

    /// Lets a test reach the event cap without writing twenty thousand lines.
    #[cfg(test)]
    pub(super) fn jump_to_seq(&mut self, id: &JobId, seq: u64) {
        self.next_seq.insert(id.clone(), seq);
    }

    fn new_id(&mut self) -> JobId {
        loop {
            self.counter += 1;
            // Milliseconds first, zero-padded: ids sort by creation time.
            let raw = format!(
                "{:012x}-{:x}-{:x}",
                now_ms(),
                std::process::id(),
                self.counter
            );
            let id = JobId::parse(raw).expect("generated ids are valid");
            if !self.jobs.contains_key(&id) {
                return id;
            }
        }
    }

    fn write_job(&mut self, job: &Job) -> Result<(), String> {
        let line = serde_json::to_string(job).map_err(|error| error.to_string())?;
        self.journal
            .write_all(format!("{line}\n").as_bytes())
            .and_then(|()| self.journal.sync_data())
            .map_err(|error| format!("No se pudo guardar el trabajo {}: {error}", job.id))?;
        self.journal_lines += 1;
        Ok(())
    }

    /// Folds the journal into the snapshot and forgets the oldest finished jobs.
    fn compact(&mut self) -> Result<(), String> {
        let finished: Vec<JobId> = self
            .jobs
            .values()
            .rev()
            .filter(|job| job.state.is_terminal())
            .skip(MAX_KEPT_FINISHED)
            .map(|job| job.id.clone())
            .collect();
        for id in finished {
            self.jobs.remove(&id);
            self.next_seq.remove(&id);
            let _ = fs::remove_file(events_path(&self.dir, &id));
        }
        let snapshot = Snapshot {
            version: 1,
            jobs: self.jobs.values().cloned().collect(),
        };
        let bytes = serde_json::to_vec_pretty(&snapshot).map_err(|error| error.to_string())?;
        write_atomic(&self.dir.join("jobs.json"), &bytes)?;
        self.journal = open_journal(&self.dir, true)?;
        self.journal_lines = 0;
        Ok(())
    }
}

fn load(dir: &Path) -> Result<(BTreeMap<JobId, Job>, Vec<String>), String> {
    let mut warnings = Vec::new();
    let mut jobs = BTreeMap::new();
    let snapshot_path = dir.join("jobs.json");
    match fs::read(&snapshot_path) {
        Ok(bytes) => match serde_json::from_slice::<Snapshot>(&bytes) {
            Ok(snapshot) => jobs.extend(snapshot.jobs.into_iter().map(|job| (job.id.clone(), job))),
            Err(error) => {
                // Kept aside for diagnosis; the journal may still recover recent jobs.
                let aside = dir.join(format!("jobs.json.corrupt-{}", now_ms()));
                let _ = fs::rename(&snapshot_path, &aside);
                warnings.push(format!(
                    "jobs.json no se pudo leer ({error}); se apartó en {}",
                    aside.display()
                ));
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "No se pudo leer {}: {error}",
                snapshot_path.display()
            ));
        }
    }
    let journal = fs::read_to_string(dir.join("journal.jsonl")).unwrap_or_default();
    let mut skipped = 0;
    for line in journal.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str::<Job>(line) {
            Ok(job) => {
                jobs.insert(job.id.clone(), job);
            }
            Err(_) => skipped += 1,
        }
    }
    if skipped > 0 {
        warnings.push(format!(
            "Se ignoraron {skipped} líneas incompletas del diario de trabajos"
        ));
    }
    Ok((jobs, warnings))
}

fn newest_first(jobs: impl Iterator<Item = Job>) -> Vec<Job> {
    let mut jobs: Vec<Job> = jobs.collect();
    jobs.sort_by(|a, b| b.id.cmp(&a.id));
    jobs
}

fn events_path(dir: &Path, id: &JobId) -> PathBuf {
    // A JobId only holds letters, digits, `-` and `_`: safe as a file name.
    dir.join("events").join(format!("{id}.jsonl"))
}

fn open_journal(dir: &Path, truncate: bool) -> Result<File, String> {
    let path = dir.join("journal.jsonl");
    let mut options = OpenOptions::new();
    options.create(true);
    if truncate {
        options.write(true).truncate(true);
    } else {
        options.append(true);
    }
    options
        .open(&path)
        .map_err(|error| format!("No se pudo abrir {}: {error}", path.display()))
}

fn create_private_dir(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|error| format!("No se pudo crear {}: {error}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let result = File::create(&temp)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|error| format!("No se pudo escribir {}: {error}", path.display()))
}

fn clean(text: &str) -> String {
    redact(&text.chars().take(MAX_TEXT_CHARS).collect::<String>())
}

/// Secrets never reach the event files, wherever they appear in the event.
fn redact_body(body: JobEventBody) -> JobEventBody {
    fn walk(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(text) => *text = redact(text),
            serde_json::Value::Array(items) => items.iter_mut().for_each(walk),
            serde_json::Value::Object(map) => map.values_mut().for_each(walk),
            _ => {}
        }
    }
    match body {
        JobEventBody::Progress { message, fraction } => JobEventBody::Progress {
            message: clean(&message),
            fraction,
        },
        JobEventBody::Agent { mut event } => {
            walk(&mut event);
            JobEventBody::Agent { event }
        }
        JobEventBody::Error { error } => JobEventBody::Error {
            error: JobError::new(error.code, clean(&error.message)),
        },
        state @ JobEventBody::State { .. } => state,
    }
}

fn internal(message: String) -> JobError {
    JobError::new(ErrorCode::Internal, message)
}
