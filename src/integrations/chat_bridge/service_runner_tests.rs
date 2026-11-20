use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-service-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn executable(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A shell script standing in for `chxchx-agent-adapter run`: it logs the request line and every
/// later line, and prints what `script` says.
fn fake_adapter(dir: &Path, script: &str) -> PathBuf {
    let path = dir.join("chxchx-agent-adapter");
    executable(
        &path,
        &format!(
            "read -r request\nprintf '%s\\n' \"$request\" > \"{dir}/request.json\"\n{script}",
            dir = dir.display()
        ),
    );
    path
}

fn spec(dir: &Path, provider: &str) -> TurnSpec {
    TurnSpec {
        agent: provider.to_lowercase(),
        provider: provider.into(),
        project: dir.to_path_buf(),
        prompt: "revisa el módulo".into(),
        mode: "workspace-write".into(),
        agent_command: vec![provider.to_lowercase()],
        executable: Some(PathBuf::from("/opt/bin/agent")),
        trusted: true,
        ..Default::default()
    }
}

fn say(json: &str) -> String {
    format!("printf '%s\\n' '{json}'\n")
}

const SESSION: &str =
    r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"session","session_id":"thr-1"}"#;
const MESSAGE: &str = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"message","role":"assistant","text":"Hecho."}"#;
const DONE: &str = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"adapter_done","exit_code":0,"cancelled":false,"session_id":"thr-1"}"#;

fn wait_done(turn: &mut Turn) -> Vec<Value> {
    let mut all = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        all.extend(turn.poll());
        if turn.exit.is_some() {
            return all;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the turn never finished: {all:?}");
}

fn request(dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(dir.join("request.json")).unwrap()).unwrap()
}

fn kinds(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .map(|e| e["type"].as_str().unwrap_or(""))
        .collect()
}

#[test]
fn a_turn_sends_one_request_and_the_closing_line_becomes_done() {
    let dir = scratch("turn");
    let adapter = fake_adapter(
        &dir,
        &format!(
            "{}{}{}cat >/dev/null",
            say(SESSION),
            say(MESSAGE),
            say(DONE)
        ),
    );
    let mut turn = Turn::start_service(&adapter, &spec(&dir, "Codex")).unwrap();
    let events = wait_done(&mut turn);

    assert_eq!(kinds(&events), ["start", "session", "message", "done"]);
    let done = events.last().unwrap();
    assert_eq!(
        (done["exit_code"].as_i64(), done["cancelled"].as_bool()),
        (Some(0), Some(false))
    );
    assert_eq!(done["session_id"], "thr-1");
    assert_eq!(events[0]["provider"], "Codex");

    let sent = request(&dir);
    assert_eq!(sent["schema"], "chxchx.agent-adapter-request");
    assert_eq!(sent["provider"], "Codex");
    assert_eq!(
        sent["argv"],
        serde_json::json!(["/opt/bin/agent", "app-server"])
    );
    assert_eq!(sent["codex"]["sandbox"], "workspace-write");
    let prompt = sent["prompt"].as_str().unwrap();
    assert!(prompt.contains(crate::integrations::chat_context::CHAT_PANEL_REQUEST_MARKER));
    assert!(prompt.ends_with("revisa el módulo"), "{prompt}");
    assert!(sent["session_id"].is_null());
}

#[test]
fn a_resumed_session_sends_only_the_request_and_its_session() {
    let dir = scratch("resume");
    let adapter = fake_adapter(&dir, &format!("{}cat >/dev/null", say(DONE)));
    let mut resumed = spec(&dir, "OpenCode");
    resumed.session = Some("ses_previa".into());
    resumed.model = Some("ollama/qwen3".into());
    let mut turn = Turn::start_service(&adapter, &resumed).unwrap();
    wait_done(&mut turn);

    let sent = request(&dir);
    assert_eq!(sent["prompt"], "revisa el módulo");
    assert_eq!(sent["session_id"], "ses_previa");
    assert_eq!(sent["model"], "ollama/qwen3");
    assert_eq!(sent["provider"], "OpenCode");
    assert_eq!(sent["argv"][1], "serve");
    assert_eq!(sent["opencode"]["agent"], "build");
}

#[test]
fn images_travel_as_attachments_inside_the_project_not_in_the_text() {
    let dir = scratch("images");
    let adapter = fake_adapter(&dir, &format!("{}cat >/dev/null", say(DONE)));
    let picture = dir.join("captura.png");
    fs::write(&picture, b"png").unwrap();
    let mut with_image = spec(&dir, "Codex");
    with_image.session = Some("thr-0".into());
    with_image.images = vec![picture];
    let mut turn = Turn::start_service(&adapter, &with_image).unwrap();
    wait_done(&mut turn);

    let sent = request(&dir);
    assert_eq!(
        sent["prompt"], "revisa el módulo",
        "the provider gets files, not a listing"
    );
    let images = sent["images"].as_array().unwrap();
    assert_eq!(images.len(), 1);
    let staged = PathBuf::from(images[0].as_str().unwrap());
    assert!(
        staged.starts_with(dir.join(".ai/chat-attachments")),
        "{staged:?}"
    );
    assert!(staged.is_file());
}

#[test]
fn nothing_runs_without_trust_and_a_bad_access_mode_is_refused() {
    let dir = scratch("trust");
    let adapter = fake_adapter(&dir, "echo should-not-run > \"$0.ran\"");
    let mut untrusted = spec(&dir, "Codex");
    untrusted.trusted = false;
    assert!(
        Turn::start_service(&adapter, &untrusted)
            .err()
            .unwrap()
            .contains("confianza")
    );
    assert!(Turn::start_service_handoff(&adapter, &untrusted).is_err());
    let mut wrong = spec(&dir, "Codex");
    wrong.mode = "todo-permitido".into();
    assert!(Turn::start_service(&adapter, &wrong).is_err());
    assert!(!dir.join("chxchx-agent-adapter.ran").exists());
}

#[test]
fn answers_and_cancel_reach_the_service_and_a_cancelled_turn_says_so() {
    let dir = scratch("control");
    let card = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"permission","live":true,"request_id":"codex-1","id":"t1","tool":"Bash","summary":"ls","detail":"","certain":true,"choices":[]}"#;
    let cancelled = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"adapter_done","exit_code":0,"cancelled":true,"session_id":"thr-1"}"#;
    let adapter = fake_adapter(
        &dir,
        &format!(
            "{}{}read -r answer\nprintf '%s\\n' \"$answer\" > \"{d}/answer.json\"\nread -r stop\nprintf '%s\\n' \"$stop\" > \"{d}/stop.txt\"\n{}",
            say(SESSION),
            say(card),
            say(cancelled),
            d = dir.display()
        ),
    );
    let mut turn = Turn::start_service(&adapter, &spec(&dir, "Codex")).unwrap();
    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !seen.iter().any(|e: &Value| e["type"] == "permission") && Instant::now() < deadline {
        seen.extend(turn.poll());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(seen.iter().any(|e| e["type"] == "permission"));

    let reply = serde_json::json!({"type":"answer","request_id":"codex-1","behavior":"allow","scope":"session"});
    turn.answer(&reply).unwrap();
    turn.cancel();
    let events = wait_done(&mut turn);

    let answered: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("answer.json")).unwrap()).unwrap();
    assert_eq!(answered, reply);
    assert_eq!(
        fs::read_to_string(dir.join("stop.txt")).unwrap().trim(),
        "cancel"
    );
    let done = events.last().unwrap();
    assert_eq!(
        (done["type"].as_str(), done["cancelled"].as_bool()),
        (Some("done"), Some(true))
    );
    assert!(turn.was_cancelled());
}

#[test]
fn a_service_that_dies_without_reporting_still_ends_the_turn() {
    let dir = scratch("crash");
    let adapter = fake_adapter(
        &dir,
        &format!("{}echo 'Traceback: algo falló' >&2\nexit 3", say(SESSION)),
    );
    let mut turn = Turn::start_service(&adapter, &spec(&dir, "Codex")).unwrap();
    let events = wait_done(&mut turn);

    let done = events.last().unwrap();
    assert_eq!(
        (done["type"].as_str(), done["exit_code"].as_i64()),
        (Some("done"), Some(3))
    );
    assert_eq!(done["session_id"], "thr-1");
    std::thread::sleep(Duration::from_millis(100));
    assert!(turn.stderr_tail().contains("Traceback"));
}

#[test]
fn the_summary_turn_is_read_only_returns_only_the_summary_and_refuses_cards() {
    let dir = scratch("handoff");
    let card = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"permission","live":true,"request_id":"codex-9","id":"t","tool":"Bash","summary":"x","detail":"","certain":true,"choices":[]}"#;
    let summary = r###"{"schema":"chxchx.agent-event","schema_version":1,"type":"message","role":"assistant","text":"## Objetivo\nseguir"}"###;
    let adapter = fake_adapter(
        &dir,
        &format!(
            "{}read -r answer\nprintf '%s\\n' \"$answer\" > \"{d}/refusal.json\"\n{}{}cat >/dev/null",
            say(card),
            say(summary),
            say(DONE),
            d = dir.display()
        ),
    );
    let mut with_session = spec(&dir, "Codex");
    with_session.session = Some("thr-llena".into());
    with_session.mode = "full-access".into(); // whatever the chat used, the summary reads only
    let mut turn = Turn::start_service_handoff(&adapter, &with_session).unwrap();
    assert!(turn.is_handoff());
    let events = wait_done(&mut turn);

    assert_eq!(turn.take_handoff_text(), "## Objetivo\nseguir");
    assert!(
        events
            .iter()
            .all(|e| e["type"] != "message" && e["type"] != "permission"),
        "{events:?}"
    );
    let sent = request(&dir);
    assert_eq!(sent["codex"]["sandbox"], "read-only");
    assert_eq!(sent["session_id"], "thr-llena");
    assert_eq!(sent["prompt"], crate::features::chat_handoff::PROMPT);
    let refusal: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("refusal.json")).unwrap()).unwrap();
    assert_eq!(
        (refusal["request_id"].as_str(), refusal["behavior"].as_str()),
        (Some("codex-9"), Some("deny"))
    );
}

#[test]
fn a_summary_that_fails_leaves_no_handoff() {
    let dir = scratch("handoff-fails");
    let failed = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"adapter_done","exit_code":1,"cancelled":false,"session_id":"thr-llena"}"#;
    let adapter = fake_adapter(&dir, &format!("{}exit 1", say(failed)));
    let mut with_session = spec(&dir, "Codex");
    with_session.session = Some("thr-llena".into());
    let mut turn = Turn::start_service_handoff(&adapter, &with_session).unwrap();
    wait_done(&mut turn);
    assert_eq!(turn.take_handoff_text(), "");
}

/// The real service, end to end: Rust → `chxchx-agent-adapter` (Python) → a fake `codex app-server`.
/// Needs `python3`; without it the test says so and passes, since Python is not a Rust requirement.
#[test]
fn the_real_service_drives_a_fake_codex_for_a_rust_turn() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("services/agent-adapter");
    let python = ["python3", "python"].into_iter().find(|name| {
        Command::new(name)
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
    });
    let Some(python) = python else {
        eprintln!("skipped: no python3 on this machine");
        return;
    };
    let dir = scratch("real");
    let (source, fakes) = (repo.join("src"), repo.join("tests"));
    let installed = Command::new(python)
        .current_dir(&dir)
        .env("PYTHONPATH", &fakes)
        .args([
            "-c",
            "import pathlib, fake_clis; fake_clis.install(pathlib.Path('bin'), 'codex', fake_clis.CODEX)",
        ])
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    let adapter = dir.join("chxchx-agent-adapter");
    executable(
        &adapter,
        &format!(
            "PYTHONPATH=\"{}\" exec {python} -m chxchx_agent_adapter \"$@\"",
            source.display()
        ),
    );
    let described = agent_adapter::describe(&adapter).expect("the service answers `contract`");
    assert_eq!(described.providers, ["Codex", "OpenCode"]);

    let mut real = spec(&dir, "Codex");
    real.executable = Some(dir.join("bin/codex"));
    let mut turn = Turn::start_service(&adapter, &real).unwrap();
    let events = wait_done(&mut turn);

    assert_eq!(kinds(&events)[0], "start");
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "message" && e["text"] == "eco: revisa el módulo"),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "usage" && e["output_tokens"] == 8)
    );
    let done = events.last().unwrap();
    assert_eq!(
        (done["type"].as_str(), done["exit_code"].as_i64()),
        (Some("done"), Some(0))
    );
    assert_eq!(done["session_id"], "thread-fake-1");
    assert_eq!(turn.exit, Some(0));
}

#[test]
fn context_usage_from_the_service_decides_when_the_context_is_released() {
    let dir = scratch("limits");
    let limits = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"limits","context_tokens":900,"context_window":1000}"#;
    let adapter = fake_adapter(&dir, &format!("{}{}cat >/dev/null", say(limits), say(DONE)));
    let mut turn = Turn::start_service(&adapter, &spec(&dir, "Codex")).unwrap();

    wait_done(&mut turn);

    assert!(turn.needs_context_release());
    assert!(!turn.was_cancelled());
}
