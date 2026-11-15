use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-bridge-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// An executable shell script standing in for the CLI.
fn fake_cli(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("chxchx-tech");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn event(kind: &str, extra: &str) -> String {
    format!(r#"{{"schema":"chxchx.agent-event","schema_version":1,"type":"{kind}"{extra}}}"#)
}

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

fn spec(dir: &Path) -> TurnSpec {
    TurnSpec {
        agent: "claude".into(),
        project: dir.to_path_buf(),
        prompt: "hola, ¿qué tal? \"comillas\"".into(),
        mode: "workspace-write".into(),
        ..Default::default()
    }
}

#[test]
fn only_a_cli_that_speaks_the_chat_protocol_is_accepted() {
    let dir = scratch("probe");
    let good = fake_cli(
        &dir,
        r#"[ "$1 $2 $3" = "agent chat --help" ] && echo "Usage: agent chat --live --lean" && exit 0; exit 2"#,
    );
    assert!(supports_chat(&good));
    let other = scratch("probe-other");
    let rust_cli = fake_cli(
        &other,
        r#"echo "error: unrecognized subcommand 'agent'" >&2; exit 2"#,
    );
    assert!(!supports_chat(&rust_cli));
    let old = fake_cli(
        &scratch("probe-old"),
        r#"echo "Usage: agent chat --lean"; exit 0"#,
    );
    assert!(
        !supports_chat(&old),
        "an older CLI without --live cannot ask the person"
    );
}

#[test]
fn a_turn_passes_every_option_and_reads_the_events_back() {
    let dir = scratch("turn");
    let log = dir.join("args.log");
    let cli = fake_cli(
        &dir,
        &format!(
            r#"echo "$@" > "{log}"; cat "$(echo "$@" | sed 's/.*--prompt-file \([^ ]*\).*/\1/')" > "{log}.prompt"
printf '%s
' '{start}' '{message}' 'ruido que no es json' '{done}'"#,
            log = log.display(),
            start = event("start", ""),
            message = event("message", r#","role":"assistant","text":"eco""#),
            done = event("done", r#","exit_code":0,"session_id":"s-1""#)
        ),
    );
    let mut wanted = spec(&dir);
    wanted.session = Some("s-0".into());
    wanted.network = true;
    wanted.allow = vec!["Bash(ls:*)".into(), "edit".into()];
    wanted.model = Some("sonnet".into());
    wanted.effort = Some("high".into());
    wanted.images = vec![dir.join("a.png")];

    let mut turn = Turn::start(&cli, &wanted).unwrap();
    let events = wait_done(&mut turn);

    let kinds: Vec<_> = events.iter().filter_map(|e| e["type"].as_str()).collect();
    assert_eq!(
        kinds,
        ["start", "message", "done"],
        "lines that are not JSON are skipped"
    );
    assert_eq!(turn.exit, Some(0));
    let args = fs::read_to_string(&log).unwrap();
    for part in [
        "agent chat claude --path",
        "--cancel-on-stdin --live --mode workspace-write",
        "--network",
        "--allow Bash(ls:*) --allow edit",
        "--session s-0",
        "--model sonnet",
        "--effort high",
        "--image",
    ] {
        assert!(args.contains(part), "{part:?} missing from {args}");
    }
    assert_eq!(
        fs::read_to_string(format!("{}.prompt", log.display())).unwrap(),
        "hola, ¿qué tal? \"comillas\"",
        "the prompt travels intact in a file"
    );
}

#[test]
fn answers_reach_the_paused_agent_and_unset_options_are_not_passed() {
    let dir = scratch("answer");
    let cli = fake_cli(
        &dir,
        &format!(
            r#"case "$*" in *--session*|*--model*|*--network*) echo "opciones de más" >&2; exit 9;; esac
printf '%s
' '{ask}'
read answer
printf '{{"schema":"chxchx.agent-event","schema_version":1,"type":"message","text":%s}}
' "$(printf '%s' "$answer" | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))')"
printf '%s
' '{done}'"#,
            ask = event(
                "permission",
                r#","request_id":"r1","id":"t1","tool":"Bash","summary":"Ejecutar","choices":[]"#
            ),
            done = event("done", r#","exit_code":0"#)
        ),
    );
    let mut turn = Turn::start(&cli, &spec(&dir)).unwrap();

    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !seen.iter().any(|e: &Value| e["type"] == "permission") {
        seen.extend(turn.poll());
        std::thread::sleep(Duration::from_millis(10));
    }
    turn.answer(&serde_json::json!({"type": "answer", "request_id": "r1", "behavior": "allow"}))
        .unwrap();
    seen.extend(wait_done(&mut turn));

    let reply = seen
        .iter()
        .find(|e| e["type"] == "message")
        .expect("the agent answered");
    assert!(
        reply["text"]
            .as_str()
            .unwrap()
            .contains(r#""behavior": "allow""#)
            || reply["text"]
                .as_str()
                .unwrap()
                .contains(r#""behavior":"allow""#),
        "{reply}"
    );
    assert_eq!(turn.exit, Some(0), "{}", turn.stderr_tail());
}

#[test]
fn cancelling_stops_the_agent_and_a_dropped_turn_leaves_nothing_behind() {
    let dir = scratch("cancel");
    let cli = fake_cli(
        &dir,
        &format!(
            r#"printf '%s
' '{start}'
while read line; do [ "$line" = "cancel" ] && break; done
printf '%s
' '{done}'"#,
            start = event("start", ""),
            done = event("done", r#","exit_code":143,"cancelled":true"#)
        ),
    );
    let mut turn = Turn::start(&cli, &spec(&dir)).unwrap();
    turn.cancel();
    let events = wait_done(&mut turn);
    assert!(events.iter().any(|e| e["cancelled"] == true));

    let prompt_file = {
        let running =
            Turn::start(&fake_cli(&scratch("cancel-drop"), "sleep 30"), &spec(&dir)).unwrap();
        let file = running.prompt_file.clone();
        assert!(file.exists());
        file
    }; // dropped while the agent was still running
    assert!(!prompt_file.exists(), "the prompt file is removed");
}

#[test]
fn the_cli_is_found_through_the_environment_variable_first() {
    let dir = scratch("find");
    let good = fake_cli(&dir, r#"echo "agent chat --live"; exit 0"#);
    let found = candidates(&dir, &OsString::new());
    assert!(found.is_empty() || found[0] != good);
    // SAFETY: tests of this module do not read the variable concurrently.
    unsafe { std::env::set_var(CLI_ENV, &good) };
    assert_eq!(find_cli(&dir), Some(good));
    unsafe { std::env::remove_var(CLI_ENV) };
}

#[test]
fn model_options_and_saved_chats_are_read_from_the_cli_output() {
    let options = parse_options(r#"{"schema":"chxchx.agent-options","schema_version":1,"available":true,"install_hint":"npm i","default_model":"gpt-x",
        "default_effort":"medium","models":[{"id":"gpt-x","efforts":["low","high"],"default_effort":"low"},{"id":"mini","label":"Mini"},{"id":""}]}"#).unwrap();
    assert_eq!(
        (
            options.default_model.as_str(),
            options.default_effort.as_str()
        ),
        ("gpt-x", "medium")
    );
    assert_eq!(options.models.len(), 2);
    assert_eq!(
        options.models[0],
        ModelOption {
            id: "gpt-x".into(),
            label: "gpt-x".into(),
            efforts: vec!["low".into(), "high".into()],
            default_effort: "low".into()
        }
    );
    assert_eq!(options.models[1].label, "Mini");
    assert!(
        parse_options(r#"{"schema":"chxchx.error","error":"sin trust"}"#)
            .unwrap_err()
            .contains("sin trust")
    );

    let saved = parse_saved_chat(r#"{"schema":"chxchx.chat-state","schema_version":1,"session_id":"s","messages":[{"role":"user","text":"a"},{"role":"tool","text":"x"},{"role":"assistant","text":""},{"role":"assistant","text":"b"}]}"#).unwrap();
    assert_eq!(
        saved,
        SavedChat {
            session_id: "s".into(),
            messages: vec![
                ("user".into(), "a".into()),
                ("assistant".into(), "b".into())
            ]
        }
    );
    assert!(parse_saved_chat(r#"{"schema":"otro"}"#).is_err());
}

#[test]
#[ignore = "needs the real CLI and agents installed"]
fn real_options_load() {
    let root = std::env::current_dir().unwrap();
    let cli = find_cli(&root).expect("cli");
    for agent in ["opencode", "claude", "codex"] {
        let options = agent_options(&cli, &root, agent);
        eprintln!("{agent}: {:?}", options.as_ref().map(|o| o.models.len()));
        assert!(options.is_ok(), "{agent}: {options:?}");
    }
}
