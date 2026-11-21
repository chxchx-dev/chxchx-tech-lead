//! Runs queued jobs on their own threads, a few at a time, outside any window.
//!
//! The runner owns the store. An `Executor` does the work of a job: it reports through its
//! `Context` and checks `cancelled()` at safe points (cancellation is cooperative). Whatever the
//! executor does, including panicking, the job ends in a final state and the runner keeps going.

use super::store::JobStore;
use chxchx_protocol::{
    ErrorCode, IdempotencyKey, Job, JobAnswer, JobError, JobEvent, JobEventBody, JobId, JobSpec,
    JobState,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub trait Executor: Send + Sync + 'static {
    /// Does the job. `Ok` carries a short summary of the outcome.
    fn run(&self, job: &Job, context: &Context) -> Result<Option<String>, JobError>;
}

/// What an executor can do while it runs a job.
pub struct Context {
    id: JobId,
    cancel: Arc<AtomicBool>,
    shared: Arc<Shared>,
    answers: Receiver<JobAnswer>,
    pending: Arc<Mutex<BTreeSet<String>>>,
}

impl Context {
    pub fn emit(&self, body: JobEventBody) {
        let event = self.shared.store().append_event(&self.id, body);
        self.shared.broadcast(event);
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Whether someone may answer this job's questions (the node), as opposed to nobody (`jobs
    /// run` in a terminal).
    pub fn attended(&self) -> bool {
        self.shared.attended
    }

    /// From now on, `JobRunner::answer` accepts an answer to `request_id`.
    pub fn expect_answer(&self, request_id: &str) {
        lock(&self.pending).insert(request_id.to_owned());
    }

    /// No answer to `request_id` is wanted any more (answered, refused or the turn moved on).
    pub fn settle(&self, request_id: &str) {
        lock(&self.pending).remove(request_id);
    }

    /// Answers that arrived since the last call.
    pub fn answers(&self) -> Vec<JobAnswer> {
        self.answers.try_iter().collect()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

struct Running {
    cancel: Arc<AtomicBool>,
    inbox: Sender<JobAnswer>,
    pending: Arc<Mutex<BTreeSet<String>>>,
}

#[derive(Default)]
struct Schedule {
    queue: VecDeque<JobId>,
    running: BTreeMap<JobId, Running>,
}

struct Shared {
    store: Mutex<JobStore>,
    executor: Box<dyn Executor>,
    limit: usize,
    attended: bool,
    schedule: Mutex<Schedule>,
    idle: Condvar,
    listeners: Mutex<Vec<Sender<JobEvent>>>,
}

impl Shared {
    fn store(&self) -> MutexGuard<'_, JobStore> {
        self.store
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn schedule(&self) -> MutexGuard<'_, Schedule> {
        self.schedule
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn broadcast(&self, events: impl IntoIterator<Item = JobEvent>) {
        let mut listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for event in events {
            listeners.retain(|listener| listener.send(event.clone()).is_ok());
        }
    }
}

#[derive(Clone)]
pub struct JobRunner {
    shared: Arc<Shared>,
}

impl JobRunner {
    pub fn new(store: JobStore, executor: impl Executor, limit: usize) -> Self {
        Self::build(store, executor, limit, false)
    }

    /// A runner whose jobs can be answered through `answer` (the node).
    pub fn attended(store: JobStore, executor: impl Executor, limit: usize) -> Self {
        Self::build(store, executor, limit, true)
    }

    fn build(store: JobStore, executor: impl Executor, limit: usize, attended: bool) -> Self {
        Self {
            shared: Arc::new(Shared {
                store: Mutex::new(store),
                executor: Box::new(executor),
                limit: limit.max(1),
                attended,
                schedule: Mutex::new(Schedule::default()),
                idle: Condvar::new(),
                listeners: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Queues a job, or returns the one a previous request with the same key created.
    pub fn submit(&self, spec: JobSpec, key: Option<IdempotencyKey>) -> Result<Job, JobError> {
        self.submit_tracked(spec, key).map(|(job, _)| job)
    }

    /// Like `submit`, and says whether the job is new (`false`: an earlier request with the same
    /// key already made it). Whoever records "a job was created" needs to know.
    pub fn submit_tracked(
        &self,
        spec: JobSpec,
        key: Option<IdempotencyKey>,
    ) -> Result<(Job, bool), JobError> {
        let (job, events) = self.shared.store().create(spec, key)?;
        let created = !events.is_empty();
        self.shared.broadcast(events);
        if created {
            self.shared.schedule().queue.push_back(job.id.clone());
            pump(&self.shared);
        }
        Ok((job, created))
    }

    /// Cancels a queued job at once; a running one is asked to stop and ends as `cancelled` when
    /// its executor returns. A finished job is left as it is.
    pub fn cancel(&self, id: &JobId) -> Result<Job, JobError> {
        let mut schedule = self.shared.schedule();
        if let Some(running) = schedule.running.get(id) {
            running.cancel.store(true, Ordering::SeqCst);
            drop(schedule);
            return self.get(id).ok_or_else(|| not_found(id));
        }
        if let Some(position) = schedule.queue.iter().position(|queued| queued == id) {
            schedule.queue.remove(position);
            drop(schedule);
            let result = self
                .shared
                .store()
                .transition(id, JobState::Cancelled, None, None);
            let (job, events) =
                result.map_err(|message| JobError::new(ErrorCode::Internal, message))?;
            self.shared.broadcast(events);
            self.shared.idle.notify_all();
            return Ok(job);
        }
        drop(schedule);
        self.get(id).ok_or_else(|| not_found(id))
    }

    /// Hands an answer to the running job that is waiting for it.
    pub fn answer(&self, id: &JobId, answer: JobAnswer) -> Result<(), JobError> {
        answer.validate()?;
        let schedule = self.shared.schedule();
        let Some(running) = schedule.running.get(id) else {
            return Err(if self.get(id).is_some() {
                JobError::new(ErrorCode::Conflict, "El trabajo no está en curso")
            } else {
                not_found(id)
            });
        };
        if !lock(&running.pending).remove(&answer.request_id) {
            return Err(JobError::new(
                ErrorCode::Conflict,
                "El trabajo no espera una respuesta a esa petición (ya se respondió o caducó)",
            ));
        }
        running
            .inbox
            .send(answer)
            .map_err(|_| JobError::new(ErrorCode::Conflict, "El trabajo ya terminó"))
    }

    pub fn get(&self, id: &JobId) -> Option<Job> {
        self.shared.store().get(id).cloned()
    }

    pub fn list(&self) -> Vec<Job> {
        self.shared.store().list()
    }

    pub fn events_since(&self, id: &JobId, after_seq: u64) -> Vec<JobEvent> {
        self.shared.store().events_since(id, after_seq)
    }

    /// Every event from now on, until the receiver is dropped.
    pub fn subscribe(&self) -> Receiver<JobEvent> {
        let (sender, receiver) = channel();
        self.shared
            .listeners
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(sender);
        receiver
    }

    /// Waits until nothing is queued or running; `false` if `timeout` passed first.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut schedule = self.shared.schedule();
        while !schedule.queue.is_empty() || !schedule.running.is_empty() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            schedule = self
                .shared
                .idle
                .wait_timeout(schedule, left)
                .unwrap_or_else(|poison| poison.into_inner())
                .0;
        }
        true
    }
}

/// Starts queued jobs while there is room.
fn pump(shared: &Arc<Shared>) {
    loop {
        let mut schedule = shared.schedule();
        if schedule.running.len() >= shared.limit {
            return;
        }
        let Some(id) = schedule.queue.pop_front() else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(Mutex::new(BTreeSet::new()));
        let (inbox, answers) = channel();
        schedule.running.insert(
            id.clone(),
            Running {
                cancel: cancel.clone(),
                inbox,
                pending: pending.clone(),
            },
        );
        drop(schedule);

        let started = shared
            .store()
            .transition(&id, JobState::Running, None, None);
        let job = match started {
            Ok((job, events)) => {
                shared.broadcast(events);
                job
            }
            Err(_) => {
                // It vanished or was already moved on: nothing to run.
                shared.schedule().running.remove(&id);
                continue;
            }
        };
        let shared = shared.clone();
        std::thread::spawn(move || run(&shared, job, cancel, answers, pending));
    }
}

fn run(
    shared: &Arc<Shared>,
    job: Job,
    cancel: Arc<AtomicBool>,
    answers: Receiver<JobAnswer>,
    pending: Arc<Mutex<BTreeSet<String>>>,
) {
    let context = Context {
        id: job.id.clone(),
        cancel: cancel.clone(),
        shared: shared.clone(),
        answers,
        pending,
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| shared.executor.run(&job, &context)))
        .unwrap_or_else(|_| {
            Err(JobError::new(
                ErrorCode::Internal,
                "El trabajo falló de forma inesperada",
            ))
        });
    let (state, summary, error) = if cancel.load(Ordering::SeqCst) {
        (JobState::Cancelled, None, None)
    } else {
        match outcome {
            Ok(summary) => (JobState::Succeeded, summary, None),
            Err(error) => (JobState::Failed, None, Some(error)),
        }
    };
    let finished = shared.store().transition(&job.id, state, summary, error);
    if let Ok((_, events)) = finished {
        shared.broadcast(events);
    }
    shared.schedule().running.remove(&job.id);
    shared.idle.notify_all();
    pump(shared);
}

fn not_found(id: &JobId) -> JobError {
    JobError::new(ErrorCode::NotFound, format!("No existe el trabajo {id}"))
}
