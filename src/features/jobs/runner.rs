//! Runs queued jobs on their own threads, a few at a time, outside any window.
//!
//! The runner owns the store. An `Executor` does the work of a job: it reports through its
//! `Context` and checks `cancelled()` at safe points (cancellation is cooperative). Whatever the
//! executor does, including panicking, the job ends in a final state and the runner keeps going.

use super::store::JobStore;
use chxchx_protocol::{
    ErrorCode, IdempotencyKey, Job, JobError, JobEvent, JobEventBody, JobId, JobSpec, JobState,
};
use std::collections::{BTreeMap, VecDeque};
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
}

impl Context {
    pub fn emit(&self, body: JobEventBody) {
        let event = self.shared.store().append_event(&self.id, body);
        self.shared.broadcast(event);
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct Schedule {
    queue: VecDeque<JobId>,
    running: BTreeMap<JobId, Arc<AtomicBool>>,
}

struct Shared {
    store: Mutex<JobStore>,
    executor: Box<dyn Executor>,
    limit: usize,
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
        Self {
            shared: Arc::new(Shared {
                store: Mutex::new(store),
                executor: Box::new(executor),
                limit: limit.max(1),
                schedule: Mutex::new(Schedule::default()),
                idle: Condvar::new(),
                listeners: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Queues a job, or returns the one a previous request with the same key created.
    pub fn submit(&self, spec: JobSpec, key: Option<IdempotencyKey>) -> Result<Job, JobError> {
        let (job, events) = self.shared.store().create(spec, key)?;
        let created = !events.is_empty();
        self.shared.broadcast(events);
        if created {
            self.shared.schedule().queue.push_back(job.id.clone());
            pump(&self.shared);
        }
        Ok(job)
    }

    /// Cancels a queued job at once; a running one is asked to stop and ends as `cancelled` when
    /// its executor returns. A finished job is left as it is.
    pub fn cancel(&self, id: &JobId) -> Result<Job, JobError> {
        let mut schedule = self.shared.schedule();
        if let Some(flag) = schedule.running.get(id) {
            flag.store(true, Ordering::SeqCst);
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
        schedule.running.insert(id.clone(), cancel.clone());
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
        std::thread::spawn(move || run(&shared, job, cancel));
    }
}

fn run(shared: &Arc<Shared>, job: Job, cancel: Arc<AtomicBool>) {
    let context = Context {
        id: job.id.clone(),
        cancel: cancel.clone(),
        shared: shared.clone(),
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
