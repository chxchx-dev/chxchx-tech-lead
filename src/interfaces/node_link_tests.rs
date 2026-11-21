//! Studio's side of the node, against a real node: a turn that runs there, survives the window
//! that started it and is picked up again.

use super::api_tests::{TOKEN, with_real_node_using};
use crate::features::jobs::{Context, Executor};
use crate::integrations::chat_bridge::{Turn, TurnSpec};
use crate::integrations::node_link::{self, Endpoint};
use chxchx_protocol::{Job, JobError, JobEventBody, JobId, JobSpec, JobState, ProjectId};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn say(context: &Context, event: Value) {
    let mut event = event;
    event["schema"] = json!("chxchx.agent-event");
    event["schema_version"] = json!(1);
    context.emit(JobEventBody::Agent { event });
}

/// An agent that talks, asks for a permission and waits for the answer.
struct Scripted;

impl Executor for Scripted {
    fn run(&self, _job: &Job, context: &Context) -> Result<Option<String>, JobError> {
        say(context, json!({"type": "session", "session_id": "sess-1"}));
        say(
            context,
            json!({"type": "message", "role": "assistant", "text": "Voy a mirar."}),
        );
        context.expect_answer("req-1");
        say(
            context,
            json!({
                "type": "permission", "id": "p1", "request_id": "req-1", "tool": "Bash",
                "summary": "cargo test", "detail": "cargo test", "certain": true, "choices": []
            }),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if context.cancelled() {
                return Ok(None);
            }
            if let Some(answer) = context.answers().into_iter().next() {
                let decision = format!("{:?}", answer.behavior);
                say(
                    context,
                    json!({"type": "message", "role": "assistant", "text": format!("Respuesta: {decision}")}),
                );
                return Ok(Some("hecho".into()));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(JobError::new(
            chxchx_protocol::ErrorCode::Internal,
            "nadie respondió",
        ))
    }
}

fn spec() -> TurnSpec {
    TurnSpec {
        agent: "codex".into(),
        provider: "Codex".into(),
        project: PathBuf::from("/p"),
        prompt: "revisa".into(),
        mode: "workspace-write".into(),
        agent_command: vec!["codex".into()],
        executable: Some(PathBuf::from("/bin/codex")),
        agent_cwd: PathBuf::from("/p"),
        trusted: true,
        ..Default::default()
    }
}

fn project() -> ProjectId {
    ProjectId::parse("demo").unwrap()
}

/// Polls the turn until `wanted` says it saw what it was waiting for.
fn until(turn: &mut Turn, seen: &mut Vec<Value>, wanted: impl Fn(&[Value], &Turn) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        seen.extend(turn.poll());
        if wanted(seen, turn) {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("never saw it; got {seen:?}");
}

fn has(seen: &[Value], kind: &str, key: &str, text: &str) -> bool {
    seen.iter()
        .any(|e| e["type"] == kind && e[key].as_str().is_some_and(|v| v.contains(text)))
}

#[test]
fn a_turn_runs_on_the_node_and_the_chat_sees_and_answers_what_the_agent_does() {
    with_real_node_using("studio-turn", Scripted, |port| {
        let endpoint = Endpoint::new(port, TOKEN);
        let mut turn = Turn::launch_on_node(&spec(), endpoint, &project()).unwrap();
        assert!(turn.is_remote() && turn.is_running());
        let mut seen = Vec::new();
        until(&mut turn, &mut seen, |s, _| {
            has(s, "permission", "summary", "cargo test")
        });
        assert!(has(&seen, "message", "text", "Voy a mirar."));
        let ask = seen.iter().find(|e| e["type"] == "permission").unwrap();
        assert_eq!(ask["request_id"], "req-1", "the chat can answer it");

        turn.answer(&json!({"type": "answer", "request_id": "req-1", "behavior": "allow"}))
            .unwrap();
        until(&mut turn, &mut seen, |_, t| !t.is_running());
        assert!(has(&seen, "message", "text", "Respuesta: Some(Allow)"));
        assert_eq!(turn.exit, Some(0));
        assert!(!turn.was_cancelled());
    });
}

#[test]
fn closing_the_window_does_not_stop_the_job_and_a_new_one_picks_it_up() {
    with_real_node_using("studio-reattach", Scripted, |port| {
        let endpoint = Endpoint::new(port, TOKEN);
        let mut first = Turn::launch_on_node(&spec(), endpoint.clone(), &project()).unwrap();
        let job_id = first.node_job().cloned().unwrap();
        let mut seen = Vec::new();
        until(&mut first, &mut seen, |s, _| {
            has(s, "permission", "summary", "cargo test")
        });

        // The window is closed: Studio stops following, nothing is cancelled.
        drop(first);
        std::thread::sleep(Duration::from_millis(400));
        let job = node_link::job(&endpoint, &job_id).unwrap();
        assert_eq!(
            job.state,
            JobState::Running,
            "the node went on without Studio"
        );

        // The jobs of that project that are still running are found again.
        let alias = project();
        let running =
            node_link::running_turns(&endpoint, &alias, chxchx_protocol::Provider::Codex).unwrap();
        assert_eq!(running.iter().map(|j| &j.id).collect::<Vec<_>>(), [&job_id]);
        let other =
            node_link::running_turns(&endpoint, &alias, chxchx_protocol::Provider::Claude).unwrap();
        assert!(other.is_empty(), "another agent's chat is not mixed in");

        // A new Studio attaches, sees everything said so far, answers and sees the end.
        let mut second = Turn::attach_to_node(endpoint.clone(), &job, "Codex");
        let mut replay = Vec::new();
        until(&mut second, &mut replay, |s, _| {
            has(s, "permission", "summary", "cargo test")
        });
        assert!(
            has(&replay, "message", "text", "Voy a mirar."),
            "the history comes back"
        );
        second
            .answer(&json!({"type": "answer", "request_id": "req-1", "behavior": "deny"}))
            .unwrap();
        until(&mut second, &mut replay, |_, t| !t.is_running());
        assert!(has(&replay, "message", "text", "Respuesta: Some(Deny)"));
        assert_eq!(
            node_link::job(&endpoint, &job_id).unwrap().state,
            JobState::Succeeded
        );
    });
}

#[test]
fn stopping_the_turn_in_studio_stops_the_job_on_the_node() {
    with_real_node_using("studio-cancel", Scripted, |port| {
        let endpoint = Endpoint::new(port, TOKEN);
        let mut turn = Turn::launch_on_node(&spec(), endpoint.clone(), &project()).unwrap();
        let id = turn.node_job().cloned().unwrap();
        let mut seen = Vec::new();
        until(&mut turn, &mut seen, |s, _| {
            has(s, "permission", "summary", "cargo test")
        });
        turn.cancel();
        until(&mut turn, &mut seen, |_, t| !t.is_running());
        assert!(turn.was_cancelled());
        assert_eq!(turn.exit, Some(0), "a stop is not a failure");
        let deadline = Instant::now() + Duration::from_secs(5);
        while node_link::job(&endpoint, &id).unwrap().state != JobState::Cancelled {
            assert!(Instant::now() < deadline, "the node never cancelled it");
            std::thread::sleep(Duration::from_millis(20));
        }
    });
}

#[test]
fn a_node_that_cannot_be_reached_fails_the_launch_or_ends_the_turn_with_a_reason() {
    let unused = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let endpoint = Endpoint::new(unused, TOKEN);
    let refused = Turn::launch_on_node(&spec(), endpoint.clone(), &project());
    assert!(refused.err().unwrap().contains("no responde"));

    // A turn that was running when the node went away ends with an explanation, not a hang.
    let lost = Job {
        id: JobId::parse("lost-1").unwrap(),
        spec: JobSpec::WorkspaceStart { project: project() },
        state: JobState::Running,
        created_ms: 0,
        updated_ms: 0,
        idempotency_key: None,
        summary: None,
        error: None,
    };
    let mut turn = Turn::attach_to_node(endpoint, &lost, "Codex");
    let mut seen = Vec::new();
    until(&mut turn, &mut seen, |_, t| !t.is_running());
    assert_eq!(turn.exit, Some(1));
    assert!(has(
        &seen,
        "error",
        "message",
        "Se perdió la conexión con el nodo"
    ));
}
