use super::MAX_QUEUED;
use super::store::{MAX_EVENTS_PER_JOB, MAX_KEPT_FINISHED};
use super::*;
use chxchx_protocol::{
    AccessMode, AgentTurnSpec, ErrorCode, IdempotencyKey, Job, JobError, JobEventBody, JobId,
    JobSpec, JobState, ProjectId, Provider,
};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

fn dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-jobs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn turn(prompt: &str) -> JobSpec {
    JobSpec::AgentTurn(AgentTurnSpec {
        project: ProjectId::parse("demo").unwrap(),
        provider: Provider::Codex,
        prompt: prompt.into(),
        session: None,
        model: None,
        effort: None,
        access: AccessMode::ReadOnly,
        network: false,
    })
}

fn key(value: &str) -> Option<IdempotencyKey> {
    Some(IdempotencyKey::parse(value).unwrap())
}

fn states(events: &[chxchx_protocol::JobEvent]) -> Vec<JobState> {
    events
        .iter()
        .filter_map(|event| match event.body {
            JobEventBody::State { state } => Some(state),
            _ => None,
        })
        .collect()
}

#[test]
fn a_job_survives_its_owner_and_comes_back_interrupted_with_its_history() {
    let dir = dir("restart");
    let (queued, running) = {
        let mut store = JobStore::open(&dir).unwrap();
        let (queued, _) = store.create(turn("uno"), None).unwrap();
        let (running, _) = store.create(turn("dos"), None).unwrap();
        store
            .transition(&running.id, JobState::Running, None, None)
            .unwrap();
        store.append_event(
            &running.id,
            JobEventBody::Progress {
                message: "a medias".into(),
                fraction: Some(0.5),
            },
        );
        (queued, running)
    }; // the owner goes away without finishing either

    // Before anyone owns the store again, readers already see what happened.
    let seen = read(&dir).unwrap();
    assert!(seen.iter().all(|job| job.state == JobState::Interrupted));

    let store = JobStore::open(&dir).unwrap();
    for id in [&queued.id, &running.id] {
        let job = store.get(id).unwrap();
        assert_eq!(job.state, JobState::Interrupted);
        assert_eq!(job.error.as_ref().unwrap().code, ErrorCode::Interrupted);
    }
    let events = store.events_since(&running.id, 0);
    assert_eq!(
        states(&events),
        [JobState::Queued, JobState::Running, JobState::Interrupted]
    );
    let seqs: Vec<u64> = events.iter().map(|event| event.seq).collect();
    assert_eq!(seqs, (1..=seqs.len() as u64).collect::<Vec<_>>(), "no gaps");
    assert_eq!(store.events_since(&running.id, 3).len(), seqs.len() - 3);
}

#[test]
fn a_repeated_request_returns_the_same_job_and_a_different_one_conflicts() {
    let dir = dir("idempotency");
    let mut store = JobStore::open(&dir).unwrap();
    let (first, events) = store.create(turn("hola"), key("k-1")).unwrap();
    assert_eq!(events.len(), 1);
    let (again, events) = store.create(turn("hola"), key("k-1")).unwrap();
    assert_eq!(again.id, first.id);
    assert!(events.is_empty(), "nothing new was created");
    let error = store.create(turn("otra cosa"), key("k-1")).unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert_eq!(store.list().len(), 1);

    drop(store);
    let mut store = JobStore::open(&dir).unwrap();
    let (after_restart, _) = store.create(turn("hola"), key("k-1")).unwrap();
    assert_eq!(after_restart.id, first.id, "the key is remembered on disk");
}

#[test]
fn invalid_requests_and_illegal_moves_change_nothing() {
    let dir = dir("illegal");
    let mut store = JobStore::open(&dir).unwrap();
    assert_eq!(
        store.create(turn("  "), None).unwrap_err().code,
        ErrorCode::InvalidRequest
    );
    let (job, _) = store.create(turn("hola"), None).unwrap();
    assert!(
        store
            .transition(&job.id, JobState::Succeeded, None, None)
            .is_err()
    );
    assert_eq!(store.get(&job.id).unwrap().state, JobState::Queued);
}

#[test]
fn a_torn_journal_line_is_skipped_and_reported() {
    let dir = dir("torn");
    let id = {
        let mut store = JobStore::open(&dir).unwrap();
        let (job, _) = store.create(turn("hola"), None).unwrap();
        store
            .transition(&job.id, JobState::Cancelled, None, None)
            .unwrap();
        job.id
    };
    // A crash in the middle of a write leaves half a line at the end.
    let mut journal = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.join("journal.jsonl"))
        .unwrap();
    std::io::Write::write_all(&mut journal, b"{\"id\":\"half").unwrap();
    drop(journal);

    let store = JobStore::open(&dir).unwrap();
    assert_eq!(store.get(&id).unwrap().state, JobState::Cancelled);
    assert_eq!(store.warnings.len(), 1);
    assert_eq!(read(&dir).unwrap().len(), 1, "readers see the same jobs");
}

#[test]
fn only_one_owner_at_a_time_but_a_dead_owner_does_not_block() {
    let dir = dir("lock");
    let store = JobStore::open(&dir).unwrap();
    assert!(JobStore::open(&dir).err().unwrap().contains("otro proceso"));
    drop(store);
    assert!(!dir.join("owner.lock").exists(), "released on drop");

    // A lock left by a process that no longer exists.
    std::fs::write(dir.join("owner.lock"), "4194303").unwrap();
    assert!(JobStore::open(&dir).is_ok());
}

#[test]
fn secrets_are_redacted_and_events_past_the_cap_are_dropped_except_states() {
    let dir = dir("events");
    let mut store = JobStore::open(&dir).unwrap();
    let (job, _) = store.create(turn("hola"), None).unwrap();
    let event = store
        .append_event(
            &job.id,
            JobEventBody::Agent {
                event: serde_json::json!({ "text": "usa api_key=sk-abcdef0123456789abcdef" }),
            },
        )
        .unwrap();
    let text = serde_json::to_string(&event).unwrap();
    assert!(!text.contains("sk-abcdef0123456789abcdef"), "{text}");

    let progress = || JobEventBody::Progress {
        message: "x".into(),
        fraction: None,
    };
    store.jump_to_seq(&job.id, MAX_EVENTS_PER_JOB);
    let at_cap = store.append_event(&job.id, progress()).unwrap();
    assert_eq!(at_cap.seq, MAX_EVENTS_PER_JOB, "the last one that fits");
    assert!(
        store.append_event(&job.id, progress()).is_none(),
        "past the cap, dropped"
    );
    let state = store
        .append_event(
            &job.id,
            JobEventBody::State {
                state: JobState::Queued,
            },
        )
        .expect("states are never dropped");
    assert_eq!(state.seq, MAX_EVENTS_PER_JOB + 1, "and leave no gap");
    assert!(
        store
            .events_since(&job.id, MAX_EVENTS_PER_JOB - 1)
            .iter()
            .map(|e| e.seq)
            .eq([MAX_EVENTS_PER_JOB, MAX_EVENTS_PER_JOB + 1])
    );
}

#[test]
fn only_the_newest_finished_jobs_and_their_events_are_kept() {
    let dir = dir("prune");
    let first = {
        let mut store = JobStore::open(&dir).unwrap();
        let mut first = None;
        for index in 0..MAX_KEPT_FINISHED + 2 {
            let (job, _) = store.create(turn(&format!("n{index}")), None).unwrap();
            store
                .transition(&job.id, JobState::Cancelled, None, None)
                .unwrap();
            first.get_or_insert(job.id);
        }
        first.unwrap()
    };
    let store = JobStore::open(&dir).unwrap();
    assert_eq!(store.list().len(), MAX_KEPT_FINISHED);
    assert!(store.get(&first).is_none());
    assert!(
        !dir.join("events").join(format!("{first}.jsonl")).exists(),
        "its events go with it"
    );
}

type Outcome = Result<Option<String>, JobError>;

/// Runs each job until the test lets it go (or it is cancelled), so the test controls timing.
struct Gate {
    release: Mutex<Option<Receiver<Outcome>>>,
}

impl Executor for Gate {
    fn run(&self, job: &Job, context: &Context) -> Result<Option<String>, JobError> {
        context.emit(JobEventBody::Progress {
            message: format!("empieza {}", job.id),
            fraction: None,
        });
        let JobSpec::AgentTurn(turn) = &job.spec else {
            return Ok(None);
        };
        match turn.prompt.as_str() {
            "panic" => panic!("executor bug"),
            "fail" => Err(JobError::new(ErrorCode::ProviderFailed, "sin red")),
            "wait" => loop {
                if context.cancelled() {
                    return Ok(None);
                }
                let guard = self.release.lock().unwrap();
                if let Some(Ok(result)) = guard
                    .as_ref()
                    .map(|rx| rx.recv_timeout(Duration::from_millis(10)))
                {
                    return result;
                }
            },
            prompt => Ok(Some(format!("hecho: {prompt}"))),
        }
    }
}

fn runner(name: &str, limit: usize) -> (JobRunner, Sender<Outcome>) {
    let (release, gate) = channel();
    let store = JobStore::open(&dir(name)).unwrap();
    let executor = Gate {
        release: Mutex::new(Some(gate)),
    };
    (JobRunner::new(store, executor, limit), release)
}

/// Polls instead of sleeping a fixed time: a loaded CI runner can take much longer.
fn wait_for(runner: &JobRunner, id: &JobId, state: JobState) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while runner.get(id).unwrap().state != state {
        assert!(
            std::time::Instant::now() < deadline,
            "{id} never reached {state:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_job_runs_to_the_end_and_reports_each_step_in_order() {
    let (runner, _release) = runner("run", 2);
    let events = runner.subscribe();
    let job = runner.submit(turn("listo"), None).unwrap();
    assert!(runner.wait_idle(Duration::from_secs(5)));
    let done = runner.get(&job.id).unwrap();
    assert_eq!(done.state, JobState::Succeeded);
    assert_eq!(done.summary.as_deref(), Some("hecho: listo"));
    let live: Vec<_> = events.try_iter().collect();
    assert_eq!(
        states(&live),
        [JobState::Queued, JobState::Running, JobState::Succeeded]
    );
    assert_eq!(live, runner.events_since(&job.id, 0), "live equals stored");
}

#[test]
fn the_limit_holds_jobs_in_the_queue_and_cancel_stops_queued_and_running_ones() {
    let (runner, release) = runner("limit", 1);
    let first = runner.submit(turn("wait"), None).unwrap();
    let second = runner.submit(turn("wait"), None).unwrap();
    let third = runner.submit(turn("listo"), None).unwrap();
    wait_for(&runner, &first.id, JobState::Running);
    assert_eq!(runner.get(&second.id).unwrap().state, JobState::Queued);
    assert_eq!(runner.get(&third.id).unwrap().state, JobState::Queued);

    let cancelled = runner.cancel(&second.id).unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled, "never started");
    release.send(Ok(Some("primero".into()))).unwrap();
    assert!(runner.wait_idle(Duration::from_secs(5)));
    assert_eq!(runner.get(&first.id).unwrap().state, JobState::Succeeded);
    assert_eq!(runner.get(&third.id).unwrap().state, JobState::Succeeded);

    let running = runner.submit(turn("wait"), None).unwrap();
    wait_for(&runner, &running.id, JobState::Running);
    runner.cancel(&running.id).unwrap();
    assert!(runner.wait_idle(Duration::from_secs(5)));
    assert_eq!(runner.get(&running.id).unwrap().state, JobState::Cancelled);
}

#[test]
fn a_failing_or_panicking_executor_fails_its_job_and_the_runner_keeps_going() {
    let (runner, _release) = runner("failures", 2);
    let failed = runner.submit(turn("fail"), None).unwrap();
    let panicked = runner.submit(turn("panic"), None).unwrap();
    assert!(runner.wait_idle(Duration::from_secs(5)));
    let failed = runner.get(&failed.id).unwrap();
    assert_eq!(failed.state, JobState::Failed);
    assert_eq!(failed.error.unwrap().code, ErrorCode::ProviderFailed);
    let panicked = runner.get(&panicked.id).unwrap();
    assert_eq!(panicked.error.unwrap().code, ErrorCode::Internal);

    let after = runner.submit(turn("listo"), None).unwrap();
    assert!(runner.wait_idle(Duration::from_secs(5)));
    assert_eq!(runner.get(&after.id).unwrap().state, JobState::Succeeded);
}

#[test]
fn a_repeated_submit_does_not_run_the_job_twice() {
    let (runner, _release) = runner("submit-twice", 2);
    let first = runner.submit(turn("listo"), key("same")).unwrap();
    let again = runner.submit(turn("listo"), key("same")).unwrap();
    assert_eq!(first.id, again.id);
    assert!(runner.wait_idle(Duration::from_secs(5)));
    let id: &JobId = &first.id;
    let runs = runner
        .events_since(id, 0)
        .iter()
        .filter(|event| {
            matches!(
                event.body,
                JobEventBody::State {
                    state: JobState::Running
                }
            )
        })
        .count();
    assert_eq!(runs, 1);
}

#[test]
fn only_so_many_jobs_may_wait_but_a_retry_of_one_that_exists_still_works() {
    assert_eq!(MAX_QUEUED, 50, "the real cap");
    let (runner, release) = runner("queue-cap", 1);
    const CAP: usize = 4;
    runner.set_max_queued(CAP);
    let running = runner.submit(turn("wait"), key("first")).unwrap();
    wait_for(&runner, &running.id, JobState::Running);
    for index in 0..CAP {
        runner
            .submit(turn("wait"), key(&format!("q-{index}")))
            .unwrap();
    }
    let refused = runner
        .submit(turn("wait"), key("one-too-many"))
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::TooManyRequests);
    assert_eq!(
        runner.list().len(),
        CAP + 1,
        "a refused job leaves nothing behind"
    );

    let retry = runner.submit(turn("wait"), key("q-0")).unwrap();
    assert_eq!(
        retry.state,
        JobState::Queued,
        "a retry is not a new job, so the cap does not apply"
    );

    for job in runner.list() {
        let _ = runner.cancel(&job.id);
    }
    release.send(Ok(None)).ok();
    assert!(runner.wait_idle(Duration::from_secs(10)));
}
