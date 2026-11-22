use super::agent_turn::{
    EXPIRED_REFUSAL, LiveTurn, UNATTENDED_REFUSAL, drive, prepare, project_root,
};
use super::*;
use crate::features::agents::{Agent, Provider as AgentProvider};
use crate::features::project_catalog::{ProjectCatalog, RegisteredProject};
use chxchx_protocol::{
    AccessMode, AgentTurnSpec, AnswerBehavior, ErrorCode, Job, JobAnswer, JobError, JobEventBody,
    JobSpec, JobState, ProjectId, Provider,
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
    /// Keeps running until it has received an answer.
    until_answered: bool,
    answers: Arc<Mutex<Vec<Value>>>,
    cancelled: Arc<AtomicBool>,
}

impl LiveTurn for FakeTurn {
    fn poll(&mut self) -> Vec<Value> {
        self.batches.pop_front().unwrap_or_default()
    }
    fn exit(&self) -> Option<i32> {
        let waiting = (self.until_cancelled && !self.cancelled.load(Ordering::SeqCst))
            || (self.until_answered && self.answers.lock().unwrap().is_empty());
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

struct Driving(Mutex<Option<FakeTurn>>, Duration);

impl Executor for Driving {
    fn run(&self, _job: &Job, context: &Context) -> Result<Option<String>, JobError> {
        let mut fake = self.0.lock().unwrap().take().expect("one turn per test");
        drive(&mut fake, context, Duration::from_millis(1), self.1).result
    }
}

fn run_fake(name: &str, fake: FakeTurn) -> (JobRunner, Job) {
    start(name, fake, false, Duration::from_secs(600))
}

fn start(name: &str, fake: FakeTurn, attended: bool, timeout: Duration) -> (JobRunner, Job) {
    let dir = std::env::temp_dir().join(format!("chxchx-agent-job-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = JobStore::open(&dir).unwrap();
    let executor = Driving(Mutex::new(Some(fake)), timeout);
    let runner = if attended {
        JobRunner::attended(store, executor, 1)
    } else {
        JobRunner::new(store, executor, 1)
    };
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
    let events = agent_events(&runner, &job);
    assert_eq!(
        events.len(),
        5,
        "every agent event is kept, plus the decision"
    );

    let answers = answers.lock().unwrap();
    assert_eq!(answers.len(), 1, "only the live request is answered");
    assert_eq!(answers[0]["request_id"], "req-1");
    assert_eq!(answers[0]["behavior"], "deny");
    let decision = events.iter().find(|e| e["type"] == "answered").unwrap();
    assert_eq!(decision["request_id"], "req-1");
    assert_eq!(decision["automatic"], true);
    assert_eq!(decision["reason"], UNATTENDED_REFUSAL);
}

fn wait_for_card(runner: &JobRunner, job: &Job) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !agent_events(runner, job)
        .iter()
        .any(|e| e["type"] == "permission")
    {
        assert!(std::time::Instant::now() < deadline, "the card never came");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn allow(request_id: &str) -> JobAnswer {
    JobAnswer {
        request_id: request_id.into(),
        behavior: Some(AnswerBehavior::Allow),
        scope: None,
        answers: None,
    }
}

#[test]
fn with_someone_attending_the_turn_waits_for_the_answer_and_passes_it_on() {
    let answers = Arc::new(Mutex::new(Vec::new()));
    let fake = FakeTurn {
        batches: VecDeque::from([vec![
            json!({"type": "permission", "request_id": "req-7", "summary": "cargo test"}),
        ]]),
        until_answered: true,
        answers: answers.clone(),
        ..Default::default()
    };
    let (runner, job) = start("attended", fake, true, Duration::from_secs(600));
    wait_for_card(&runner, &job);
    assert_eq!(
        runner.get(&job.id).unwrap().state,
        JobState::Running,
        "it waits"
    );

    let wrong = runner.answer(&job.id, allow("req-other")).unwrap_err();
    assert_eq!(
        wrong.code,
        ErrorCode::Conflict,
        "an unknown request is refused"
    );
    runner.answer(&job.id, allow("req-7")).unwrap();
    assert_eq!(
        runner.answer(&job.id, allow("req-7")).unwrap_err().code,
        ErrorCode::Conflict,
        "a request is answered once"
    );
    assert!(runner.wait_idle(Duration::from_secs(5)));

    assert_eq!(runner.get(&job.id).unwrap().state, JobState::Succeeded);
    let sent = answers.lock().unwrap();
    assert_eq!(
        sent[0],
        json!({"type": "answer", "request_id": "req-7", "behavior": "allow"})
    );
    let decision = agent_events(&runner, &job)
        .into_iter()
        .find(|e| e["type"] == "answered")
        .unwrap();
    assert_eq!(decision["behavior"], "allow");
    assert!(decision.get("automatic").is_none());
    let late = runner.answer(&job.id, allow("req-7")).unwrap_err();
    assert_eq!(late.code, ErrorCode::Conflict, "the job is over");
}

#[test]
fn a_card_nobody_answers_in_time_is_refused_and_the_turn_goes_on() {
    let answers = Arc::new(Mutex::new(Vec::new()));
    let fake = FakeTurn {
        batches: VecDeque::from([vec![
            json!({"type": "permission", "request_id": "req-8", "summary": "git push"}),
        ]]),
        until_answered: true,
        answers: answers.clone(),
        ..Default::default()
    };
    let (runner, job) = start("expired", fake, true, Duration::from_millis(30));
    assert!(runner.wait_idle(Duration::from_secs(5)));
    assert_eq!(answers.lock().unwrap()[0]["behavior"], "deny");
    let decision = agent_events(&runner, &job)
        .into_iter()
        .find(|e| e["type"] == "answered")
        .unwrap();
    assert_eq!(decision["reason"], EXPIRED_REFUSAL);
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

// ---- «Permitir X en este chat» from another device ---------------------------------------------

/// Like `Driving`, and keeps what the turn says the person allowed for the rest of the chat.
struct Remembering(Mutex<Option<FakeTurn>>, Arc<Mutex<Vec<String>>>);

impl Executor for Remembering {
    fn run(&self, _job: &Job, context: &Context) -> Result<Option<String>, JobError> {
        let mut fake = self.0.lock().unwrap().take().expect("one turn per test");
        let outcome = drive(
            &mut fake,
            context,
            Duration::from_millis(1),
            Duration::from_secs(600),
        );
        *self.1.lock().unwrap() = outcome.remembered.clone();
        outcome.result
    }
}

/// A card offering one choice for the chat, answered with `answer`; returns what was remembered.
fn answered_with(name: &str, choice_persists: bool, answer: JobAnswer) -> Vec<String> {
    let fake = FakeTurn {
        batches: VecDeque::from([vec![json!({
            "type": "permission", "request_id": "req-9", "tool": "Bash", "summary": "touch /tmp/x",
            "choices": [
                {"label": "Permitir este comando", "allow": ["Bash(touch /tmp/x)"], "persist": false},
                {"label": "Permitir «touch» en este chat", "allow": ["Bash(touch:*)"], "persist": choice_persists},
            ]
        })]]),
        until_answered: true,
        ..Default::default()
    };
    let remembered = Arc::new(Mutex::new(Vec::new()));
    let dir = std::env::temp_dir().join(format!("chxchx-agent-job-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let runner = JobRunner::attended(
        JobStore::open(&dir).unwrap(),
        Remembering(Mutex::new(Some(fake)), remembered.clone()),
        1,
    );
    let job = runner
        .submit(JobSpec::AgentTurn(turn(AccessMode::ReadOnly)), None)
        .unwrap();
    wait_for_card(&runner, &job);
    runner.answer(&job.id, answer).unwrap();
    assert!(runner.wait_idle(Duration::from_secs(5)));
    remembered.lock().unwrap().clone()
}

fn allow_in_chat(request_id: &str) -> JobAnswer {
    JobAnswer {
        scope: Some("chat".into()),
        ..allow(request_id)
    }
}

#[test]
fn allowing_for_the_chat_remembers_the_rules_the_agent_offered_and_nothing_else() {
    assert_eq!(
        answered_with("chat-yes", true, allow_in_chat("req-9")),
        ["Bash(touch:*)"],
        "the rule of the choice that persists, not the one-off one"
    );
}

#[test]
fn allowing_once_or_refusing_or_a_choice_that_does_not_persist_remembers_nothing() {
    assert!(
        answered_with("chat-once", true, allow("req-9")).is_empty(),
        "allowed once"
    );
    assert!(
        answered_with("chat-nopersist", false, allow_in_chat("req-9")).is_empty(),
        "the agent did not offer to remember it"
    );
    let deny = JobAnswer {
        behavior: Some(AnswerBehavior::Deny),
        scope: Some("chat".into()),
        ..allow("req-9")
    };
    assert!(
        answered_with("chat-deny", true, deny).is_empty(),
        "a refusal allows nothing"
    );
}
