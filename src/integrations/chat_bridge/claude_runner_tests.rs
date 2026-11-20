use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-claude-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A shell script standing in for `claude`; `$LOG` is where it records what it was given.
fn fake_claude(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("claude");
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nLOG=\"{}\"\n{body}\n",
            dir.join("claude.log").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn spec(dir: &Path, claude: &Path) -> TurnSpec {
    TurnSpec {
        agent: "claude".into(),
        provider: "Claude".into(),
        project: dir.to_path_buf(),
        prompt: "revisa el módulo".into(),
        mode: "workspace-write".into(),
        agent_command: vec!["claude".into(), "--flag-del-usuario".into()],
        executable: Some(claude.to_path_buf()),
        trusted: true,
        ..Default::default()
    }
}

const INIT: &str = r#"{"type":"system","subtype":"init","session_id":"sesion-1"}"#;
const REPLY: &str =
    r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hecho."}]}}"#;
const RESULT: &str = r#"{"type":"result","is_error":false,"total_cost_usd":0.01,"usage":{"input_tokens":12,"output_tokens":7}}"#;

fn wait_done(turn: &mut Turn) -> Vec<Value> {
    let mut all = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        all.extend(turn.poll());
        if turn.exit.is_some() {
            return all;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the turn never finished: {all:?}");
}

fn answering(records: &[&str]) -> String {
    let printed: String = records.iter().map(|r| format!("echo '{r}'\n")).collect();
    // One user message arrives on stdin; the end of the input (closed after the result) ends it.
    format!(
        "read line\nprintf '%s\\n' \"$*\" > \"$LOG.args\"\nprintf '%s\\n' \"$line\" > \"$LOG.stdin\"\n{printed}cat >/dev/null"
    )
}

#[test]
fn a_new_session_runs_claude_directly_with_the_project_context() {
    let dir = scratch("new");
    let claude = fake_claude(&dir, &answering(&[INIT, REPLY, RESULT]));
    let mut turn = Turn::launch(None, &spec(&dir, &claude)).unwrap();
    let events = wait_done(&mut turn);

    assert_eq!(
        turn.exit,
        Some(0),
        "closing the input after the result ends Claude"
    );
    assert_eq!(events[0]["type"], "start");
    assert_eq!(events[0]["provider"], "Claude");
    let done = events.last().unwrap();
    assert_eq!(done["type"], "done");
    assert_eq!(done["exit_code"], 0);
    assert_eq!(done["cancelled"], false);
    assert_eq!(done["session_id"], "sesion-1");
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "session" && e["session_id"] == "sesion-1")
    );
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "message" && e["text"] == "Hecho.")
    );
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "usage" && e["output_tokens"] == 7)
    );

    let args = fs::read_to_string(dir.join("claude.log.args")).unwrap();
    for expected in [
        "--flag-del-usuario",
        "-p",
        "--output-format stream-json",
        "--input-format stream-json",
        "--permission-prompt-tool stdio",
        "--permission-mode acceptEdits",
    ] {
        assert!(args.contains(expected), "{expected} missing in {args}");
    }
    assert!(!args.contains("--resume"), "{args}");
    let sent: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("claude.log.stdin")).unwrap()).unwrap();
    assert_eq!(sent["type"], "user");
    let content = sent["message"]["content"].as_str().unwrap();
    assert!(content.contains(crate::integrations::chat_context::CHAT_PANEL_REQUEST_MARKER));
    assert!(content.ends_with("revisa el módulo"), "{content}");
}

#[test]
fn a_resumed_session_sends_only_the_request() {
    let dir = scratch("resume");
    let claude = fake_claude(&dir, &answering(&[INIT, REPLY, RESULT]));
    let mut resumed = spec(&dir, &claude);
    resumed.session = Some("sesion-previa".into());
    resumed.model = Some("opus".into());
    resumed.effort = Some("high".into());
    let mut turn = Turn::launch(None, &resumed).unwrap();
    wait_done(&mut turn);

    let args = fs::read_to_string(dir.join("claude.log.args")).unwrap();
    assert!(args.contains("--resume sesion-previa"), "{args}");
    assert!(
        args.contains("--model opus") && args.contains("--effort high"),
        "{args}"
    );
    let sent: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("claude.log.stdin")).unwrap()).unwrap();
    assert_eq!(sent["message"]["content"], "revisa el módulo");
}

#[test]
fn nothing_runs_without_the_trust_the_caller_verified() {
    let dir = scratch("untrusted");
    let claude = fake_claude(&dir, "echo should-not-run > \"$LOG.ran\"");
    let mut untrusted = spec(&dir, &claude);
    untrusted.trusted = false;

    let error = Turn::launch(None, &untrusted).err().unwrap();
    assert!(error.contains("confianza"), "{error}");
    let error = Turn::launch_handoff(None, &untrusted).err().unwrap();
    assert!(error.contains("confianza"), "{error}");
    assert!(!dir.join("claude.log.ran").exists());
}

#[test]
fn an_invalid_access_mode_is_refused_before_starting() {
    let dir = scratch("mode");
    let claude = fake_claude(&dir, "echo should-not-run > \"$LOG.ran\"");
    let mut wrong = spec(&dir, &claude);
    wrong.mode = "todo-permitido".into();
    assert!(Turn::launch(None, &wrong).is_err());
    assert!(!dir.join("claude.log.ran").exists());
}

#[test]
fn a_failing_claude_reports_its_exit_code_and_what_it_wrote_to_stderr() {
    let dir = scratch("failure");
    let claude = fake_claude(&dir, "read line\necho 'sin sesión de login' >&2\nexit 3");
    let mut turn = Turn::launch(None, &spec(&dir, &claude)).unwrap();
    let events = wait_done(&mut turn);

    let done = events.last().unwrap();
    assert_eq!(
        (done["type"].as_str(), done["exit_code"].as_i64()),
        (Some("done"), Some(3))
    );
    assert_eq!(done["cancelled"], false);
    std::thread::sleep(Duration::from_millis(100));
    assert!(turn.stderr_tail().contains("sin sesión de login"));
}

#[test]
fn cancelling_stops_claude_and_what_it_started_and_is_not_a_failure() {
    let dir = scratch("cancel");
    // A child of Claude that would outlive it if only the parent were stopped.
    let claude = fake_claude(
        &dir,
        &format!(
            "read line\nsleep 60 &\necho $! > \"$LOG.child\"\n{}\nwait",
            "echo '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s\"}'"
        ),
    );
    let mut turn = Turn::launch(None, &spec(&dir, &claude)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !dir.join("claude.log.child").exists() && Instant::now() < deadline {
        turn.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    let child: u32 = fs::read_to_string(dir.join("claude.log.child"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    turn.cancel();
    let events = wait_done(&mut turn);

    let done = events.last().unwrap();
    assert_eq!(done["type"], "done");
    assert_eq!(done["cancelled"], true);
    assert!(turn.was_cancelled());
    std::thread::sleep(Duration::from_millis(300));
    let alive = Command::new("/bin/kill")
        .args(["-0", &child.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    assert!(!alive, "the process Claude started must stop with it");
}

#[test]
fn a_permission_request_is_answered_through_claudes_own_stdin() {
    let dir = scratch("live");
    let request = r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","tool_use_id":"t1","input":{"command":"ls"}}}"#;
    let claude = fake_claude(
        &dir,
        &format!(
            "read line\necho '{INIT}'\necho '{request}'\nread answer\nprintf '%s\\n' \"$answer\" > \"$LOG.answer\"\necho '{REPLY}'\necho '{RESULT}'\ncat >/dev/null"
        ),
    );
    let mut turn = Turn::launch(None, &spec(&dir, &claude)).unwrap();

    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !seen.iter().any(|e: &Value| e["type"] == "permission") && Instant::now() < deadline {
        seen.extend(turn.poll());
        std::thread::sleep(Duration::from_millis(10));
    }
    let card = seen
        .iter()
        .find(|e| e["type"] == "permission")
        .expect("a permission card");
    assert_eq!(card["request_id"], "r1");

    turn.answer(&serde_json::json!({"type":"answer","request_id":"r1","behavior":"allow"}))
        .unwrap();
    let events = wait_done(&mut turn);

    assert_eq!(events.last().unwrap()["exit_code"], 0);
    let answer: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("claude.log.answer")).unwrap()).unwrap();
    assert_eq!(answer["type"], "control_response");
    assert_eq!(answer["response"]["request_id"], "r1");
    assert_eq!(answer["response"]["response"]["behavior"], "allow");
    assert_eq!(
        answer["response"]["response"]["updatedInput"]["command"],
        "ls"
    );
}

#[test]
fn images_are_copied_into_the_project_and_listed_in_the_prompt() {
    let dir = scratch("images");
    let claude = fake_claude(&dir, &answering(&[INIT, REPLY, RESULT]));
    let picture = dir.join("captura.png");
    fs::write(&picture, b"png").unwrap();
    let mut with_image = spec(&dir, &claude);
    with_image.session = Some("sesion-previa".into());
    with_image.images = vec![picture];
    let mut turn = Turn::launch(None, &with_image).unwrap();
    wait_done(&mut turn);

    let sent: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("claude.log.stdin")).unwrap()).unwrap();
    let content = sent["message"]["content"].as_str().unwrap();
    assert!(content.contains("Imágenes adjuntas"), "{content}");
    assert!(
        content.contains(".ai/chat-attachments/") && content.contains("captura.png"),
        "{content}"
    );
    let copies: Vec<_> = fs::read_dir(dir.join(".ai/chat-attachments"))
        .unwrap()
        .collect();
    assert_eq!(copies.len(), 1);
}

#[test]
fn attachments_never_follow_a_symlinked_folder() {
    let dir = scratch("images-link");
    let outside = scratch("images-link-outside");
    std::os::unix::fs::symlink(&outside, dir.join(".ai")).unwrap();
    let claude = fake_claude(&dir, "echo should-not-run > \"$LOG.ran\"");
    let picture = dir.join("captura.png");
    fs::write(&picture, b"png").unwrap();
    let mut with_image = spec(&dir, &claude);
    with_image.session = Some("sesion-previa".into());
    with_image.images = vec![picture];

    assert!(Turn::launch(None, &with_image).is_err());
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
}

#[test]
fn the_summary_turn_is_read_only_plain_text_and_returns_only_the_summary() {
    let dir = scratch("handoff");
    let claude = fake_claude(
        &dir,
        &format!(
            "printf '%s\\n' \"$*\" > \"$LOG.args\"\ncat > \"$LOG.stdin\"\necho '{INIT}'\necho '{{\"type\":\"assistant\",\"message\":{{\"content\":[{{\"type\":\"text\",\"text\":\"Resumen: migrar el runner.\"}}]}}}}'\necho '{RESULT}'"
        ),
    );
    let mut summary = spec(&dir, &claude);
    summary.session = Some("sesion-llena".into());
    summary.mode = "full-access".into(); // whatever the chat used, the summary reads only
    let mut turn = Turn::launch_handoff(None, &summary).unwrap();
    assert!(turn.is_handoff());
    let events = wait_done(&mut turn);

    assert_eq!(turn.take_handoff_text(), "Resumen: migrar el runner.");
    assert!(
        events.iter().all(|e| e["type"] != "message"),
        "the summary never reaches the chat: {events:?}"
    );
    let args = fs::read_to_string(dir.join("claude.log.args")).unwrap();
    assert!(args.contains("--resume sesion-llena"), "{args}");
    assert!(args.contains("--permission-mode plan"), "{args}");
    assert!(
        !args.contains("--input-format") && !args.contains("bypassPermissions"),
        "{args}"
    );
    assert_eq!(
        fs::read_to_string(dir.join("claude.log.stdin")).unwrap(),
        crate::features::chat_handoff::PROMPT
    );
}

#[test]
fn a_summary_turn_that_fails_leaves_no_handoff() {
    let dir = scratch("handoff-fails");
    let claude = fake_claude(&dir, "cat >/dev/null\nexit 1");
    let mut summary = spec(&dir, &claude);
    summary.session = Some("sesion-llena".into());
    let mut turn = Turn::launch_handoff(None, &summary).unwrap();
    wait_done(&mut turn);
    assert_eq!(turn.take_handoff_text(), "");
}

#[test]
fn other_providers_still_need_the_cli() {
    let dir = scratch("others");
    let mut codex = spec(&dir, Path::new("/bin/true"));
    codex.provider = "Codex".into();
    let error = Turn::launch(None, &codex).err().unwrap();
    assert!(error.contains("CLI"), "{error}");
}
