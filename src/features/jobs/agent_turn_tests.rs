use super::agent_turn::{LiveTurn, UNATTENDED_REFUSAL, drive, prepare, project_root};
use super::*;
use crate::features::agents::{Agent, Provider as AgentProvider};
use crate::features::project_catalog::{ProjectCatalog, RegisteredProject};
use chxchx_protocol::{
    AccessMode, AgentTurnSpec, ErrorCode, Job, JobError, JobEventBody, JobSpec, JobState,
    ProjectId, Provider,
};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn turn(access: AccessMode) -> AgentTurnSpec {
    AgentTurnSpec {
        project: ProjectId::parse("demo").unwrap(),
        provider: Provider::Codex,
        prompt: "revisa el módulo".into(),
        session: Some("thr-1".into()),
        model: Some("gpt-5".into()),
        effort: None,
        access,
        network: false,
    }
}

fn codex(installed: bool) -> Agent {
    Agent {
        id: "codex".into(),
        provider: AgentProvider::Codex,
        command: vec!["codex".into()],
        cwd: PathBuf::from("/p"),
        executable: installed.then(|| PathBuf::from("/opt/bin/codex")),
        configured: false,
    }
}

#[test]
fn a_turn_only_starts_in_a_trusted_project_with_its_agent_installed() {
    let root = Path::new("/p");
    let ask = turn(AccessMode::ReadOnly);
    let untrusted = prepare(&ask, AccessMode::FullAccess, root, false, &[codex(true)]);
    assert_eq!(untrusted.unwrap_err().code, ErrorCode::UntrustedProject);
    let missing = prepare(&ask, AccessMode::FullAccess, root, true, &[codex(false)]);
    assert_eq!(missing.unwrap_err().code, ErrorCode::ProviderUnavailable);

    let spec = prepare(&ask, AccessMode::FullAccess, root, true, &[codex(true)]).unwrap();
    assert_eq!(
        (spec.agent.as_str(), spec.provider.as_str()),
        ("codex", "Codex")
    );
    assert_eq!(spec.executable, Some(PathBuf::from("/opt/bin/codex")));
    assert_eq!(spec.session.as_deref(), Some("thr-1"));
    assert_eq!(spec.model.as_deref(), Some("gpt-5"));
    assert!(spec.trusted);
}

#[test]
fn the_access_level_never_goes_above_the_cap() {
    let root = Path::new("/p");
    let wants_all = turn(AccessMode::FullAccess);
    let capped = prepare(
        &wants_all,
        AccessMode::REMOTE_DEFAULT_CAP,
        root,
        true,
        &[codex(true)],
    );
    assert_eq!(capped.unwrap().mode, "workspace-write");
    let local = prepare(
        &wants_all,
        AccessMode::FullAccess,
        root,
        true,
        &[codex(true)],
    );
    assert_eq!(local.unwrap().mode, "full-access");
}

#[test]
fn a_project_is_found_by_its_alias_and_only_when_its_folder_is_there() {
    let project = |alias: &str, available: bool| RegisteredProject {
        alias: alias.into(),
        name: alias.into(),
        profile: "rust".into(),
        path: Some(format!("/projects/{alias}")),
        stacks: Vec::new(),
        available,
        is_current: false,
    };
    let catalog = ProjectCatalog {
        projects: vec![project("demo", true), project("gone", false)],
        current: None,
    };
    let id = |alias: &str| ProjectId::parse(alias).unwrap();
    assert_eq!(
        project_root(&catalog, &id("demo")).unwrap(),
        PathBuf::from("/projects/demo")
    );
    for missing in ["gone", "other"] {
        assert_eq!(
            project_root(&catalog, &id(missing)).unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}

/// A turn that plays back batches of events, one batch per poll, then exits.
#[derive(Default)]
struct FakeTurn {
    batches: VecDeque<Vec<Value>>,
    code: i32,
    stderr: String,
    /// Keeps running (polling nothing) until it is cancelled.
    until_cancelled: bool,
    answers: Arc<Mutex<Vec<Value>>>,
    cancelled: Arc<AtomicBool>,
}

impl LiveTurn for FakeTurn {
    fn poll(&mut self) -> Vec<Value> {
        self.batches.pop_front().unwrap_or_default()
    }
    fn exit(&self) -> Option<i32> {
        let waiting = self.until_cancelled && !self.cancelled.load(Ordering::SeqCst);
        (self.batches.is_empty() && !waiting).then_some(self.code)
    }
    fn answer(&mut self, answer: &Value) -> Result<(), String> {
        self.answers.lock().unwrap().push(answer.clone());
        Ok(())
    }
    fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
    fn stderr_tail(&self) -> String {
        self.stderr.clone()
    }
}

struct Driving(Mutex<Option<FakeTurn>>);

impl Executor for Driving {
    fn run(&self, _job: &Job, context: &Context) -> Result<Option<String>, JobError> {
        let mut fake = self.0.lock().unwrap().take().expect("one turn per test");
        drive(&mut fake, context, Duration::from_millis(1)).result
    }
}

fn run_fake(name: &str, fake: FakeTurn) -> (JobRunner, Job) {
    let dir = std::env::temp_dir().join(format!("chxchx-agent-job-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = JobStore::open(&dir).unwrap();
    let runner = JobRunner::new(store, Driving(Mutex::new(Some(fake))), 1);
    let job = runner
        .submit(JobSpec::AgentTurn(turn(AccessMode::ReadOnly)), None)
        .unwrap();
    (runner, job)
}

fn agent_events(runner: &JobRunner, job: &Job) -> Vec<Value> {
    runner
        .events_since(&job.id, 0)
        .into_iter()
        .filter_map(|event| match event.body {
            JobEventBody::Agent { event } => Some(event),
            _ => None,
        })
        .collect()
}

#[test]
fn a_finished_turn_keeps_every_event_and_ends_with_the_last_answer() {
    let answers = Arc::new(Mutex::new(Vec::new()));
    let fake = FakeTurn {
        batches: VecDeque::from([
            vec![json!({"type": "session", "session_id": "thr-9"})],
            vec![
                json!({"type": "permission", "request_id": "req-1", "summary": "rm -rf build"}),
                json!({"type": "permission", "summary": "only shown, nothing to answer"}),
            ],
            vec![json!({"type": "message", "role": "assistant", "text": "Listo, revisado."})],
        ]),
        answers: answers.clone(),
        ..Default::default()
    };
    let (runner, job) = run_fake("ok", fake);
    assert!(runner.wait_idle(Duration::from_secs(5)));

    let done = runner.get(&job.id).unwrap();
    assert_eq!(done.state, JobState::Succeeded);
    assert_eq!(done.summary.as_deref(), Some("Listo, revisado."));
    assert_eq!(
        agent_events(&runner, &job).len(),
        4,
        "every agent event is kept"
    );

    let answers = answers.lock().unwrap();
    assert_eq!(answers.len(), 1, "only the live request is answered");
    assert_eq!(answers[0]["request_id"], "req-1");
    assert_eq!(answers[0]["behavior"], "deny");
    let said_why = runner.events_since(&job.id, 0).into_iter().any(|event| {
        matches!(&event.body, JobEventBody::Progress { message, .. } if message.starts_with(UNATTENDED_REFUSAL))
    });
    assert!(said_why, "the job explains the refusal");
}

#[test]
fn a_provider_that_fails_fails_the_job_with_what_it_printed() {
    let fake = FakeTurn {
        code: 2,
        stderr: "codex: not logged in\n".into(),
        ..Default::default()
    };
    let (runner, job) = run_fake("fail", fake);
    assert!(runner.wait_idle(Duration::from_secs(5)));
    let error = runner.get(&job.id).unwrap().error.unwrap();
    assert_eq!(error.code, ErrorCode::ProviderFailed);
    assert!(
        error.message.contains("código 2: codex: not logged in"),
        "{}",
        error.message
    );
}

#[test]
fn cancelling_the_job_stops_the_turn_and_ends_it_cancelled() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let fake = FakeTurn {
        until_cancelled: true,
        cancelled: cancelled.clone(),
        code: 130,
        ..Default::default()
    };
    let (runner, job) = run_fake("cancel", fake);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while runner.get(&job.id).unwrap().state != JobState::Running {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    runner.cancel(&job.id).unwrap();
    assert!(runner.wait_idle(Duration::from_secs(5)));
    assert!(
        cancelled.load(Ordering::SeqCst),
        "the turn itself was stopped"
    );
    assert_eq!(runner.get(&job.id).unwrap().state, JobState::Cancelled);
}
