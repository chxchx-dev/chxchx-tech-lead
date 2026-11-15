//! End to end inside Studio: a real project, the real integration code and a fake `chxchx-tech`
//! that speaks the same JSON protocols as the Python CLI. Each step does what a person does
//! (type, press Enter, click a button) and reads back what the screen would show.
use super::update as updater;
use super::*;
use crate::features::agents::{Agent, Provider};
use crate::integrations::chat_bridge as bridge;
use crate::integrations::local_models as models;
use crate::integrations::slash_commands;
use crate::integrations::studio_update as updates;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

/// A folder of its own per test: they run in parallel.
fn sandbox() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "chxchx-e2e-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".claude/skills/revisar")).unwrap();
    std::fs::write(
        dir.join(".claude/skills/revisar/SKILL.md"),
        "---\ndescription: Revisa cambios\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("README.md"), "# Proyecto\n\nHola.\n").unwrap();
    dir
}

const EVENT: &str = r#"{"schema":"chxchx.agent-event","schema_version":1,"type":"#;

/// The CLI the app talks to, answering every command it uses with canned but realistic JSON.
fn fake_cli(dir: &Path) -> PathBuf {
    let log = dir.join("prompts.log");
    let script = format!(
        r##"#!/bin/sh
case "$1 $2" in
  "agent chat-state") echo '{{"schema":"chxchx.chat-state","schema_version":1,"session_id":"","messages":[]}}';;
  "bridge agent-options") echo '{{"schema":"chxchx.agent-options","schema_version":1,"available":true,"default_model":"sonnet","default_effort":"","models":[{{"id":"sonnet","label":"sonnet","efforts":["low","high"],"default_effort":""}}]}}';;
  "bridge local-models") echo '{{"schema":"chxchx.local-models","schema_version":1,"hardware":{{"total_ram_bytes":17179869184,"available_ram_bytes":8589934592,"cpu_cores":8,"disk_free_bytes":500000000000,"models_dir":"/m","accelerator":"sin GPU"}},"ollama":{{"installed":true,"running":true,"version":"0.9","base_url":"http://127.0.0.1:11434","install":{{"command":null,"url":"u"}},"models":[],"loaded":[]}},"opencode":{{"installed":true,"install_hint":"h","configured_models":[]}},"context":32768,"models":[{{"tag":"qwen2.5-coder:7b","label":"Qwen2.5 Coder 7B","size_bytes":5000000000,"context_k":32,"note":"n","fit":"cpu","required_bytes":7000000000,"disk_ok":true,"summary":"s","recommended":true,"downloaded":false,"loaded":false,"configured":false}}]}}';;
  "local-models setup")
    printf '%s\n' '{{"type":"step","step":"download","model":"qwen2.5-coder:7b"}}' '{{"type":"progress","model":"qwen2.5-coder:7b","status":"pulling","completed":50,"total":100}}' '{{"type":"done","model":"ollama/qwen2.5-coder:7b-ctx32k","changed":true}}';;
  "bridge studio-update")
    if [ "$3" = "--apply" ]; then echo '{{"schema":"chxchx.studio-update","current_version":"0.5.3","latest_version":"0.6.0a1","available":true,"updated":true}}';
    else echo '{{"schema":"chxchx.studio-update","current_version":"0.5.3","latest_version":"0.6.0a1","available":true,"updated":false}}'; fi;;
  "agent chat")
    if [ "$3" = "--help" ]; then echo "Usage: agent chat --live"; exit 0; fi
    file=$(echo "$@" | sed 's/.*--prompt-file \([^ ]*\).*/\1/'); cat "$file" >> "{log}"; echo >> "{log}"
    printf '%s\n' '{e}"start"}}' '{e}"message","role":"assistant","text":"Hola, soy el agente de prueba."}}' '{e}"usage","input_tokens":12,"output_tokens":7}}' '{e}"done","exit_code":0,"session_id":"s-1"}}';;
  *) echo "comando inesperado: $*" >&2; exit 2;;
esac
"##,
        log = log.display(),
        e = EVENT
    );
    let path = dir.join("chxchx-tech");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn agent(id: &str, provider: Provider) -> Agent {
    Agent {
        id: id.into(),
        provider,
        command: vec![id.into()],
        cwd: PathBuf::new(),
        executable: Some(PathBuf::from("/bin/true")),
        configured: false,
    }
}

/// A Studio opened on a real, trusted project.
fn studio(dir: &Path) -> Dashboard {
    let mut state = editor::fixture();
    state.root = dir.to_path_buf();
    state.overview = crate::features::project_overview::inspect_project(dir).map(|mut overview| {
        overview.workspace.trusted = true;
        overview
    });
    assert!(
        state.overview.is_ok(),
        "the sandbox project must be readable"
    );
    state.agents.catalog.agents = vec![
        agent("claude", Provider::Claude),
        agent("opencode", Provider::OpenCode),
    ];
    state
}

fn until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {what}");
}

/// What `chat::open` does off the UI thread, then the message it sends back.
fn open_chat(state: &mut Dashboard, cli: &Path, agent: &Agent) {
    let _ = chat::update(state, chat::Event::Open(agent.clone()));
    let root = state.root.clone();
    let prepared = (
        Some(cli.to_path_buf()),
        bridge::restore_chat(cli, &root, &agent.id),
        bridge::agent_options(cli, &root, &agent.id),
        slash_commands::discover(&root, &agent.id),
    );
    let _ = chat::update(
        state,
        chat::Event::Prepared(
            agent.id.clone(),
            chat::PreparedBox(std::sync::Arc::new(prepared)),
        ),
    );
}

fn say(state: &mut Dashboard, text: &str) {
    state.chat.chats[state.chat.active].composer =
        iced::widget::text_editor::Content::with_text(text);
    let _ = chat::update(state, chat::Event::Send);
}

fn wait_for_answer(state: &mut Dashboard) {
    until("the agent to answer", || {
        let _ = chat::update(state, chat::Event::Tick);
        state.chat.chats[state.chat.active].turn.is_none()
    });
}

#[test]
fn a_conversation_from_opening_the_chat_to_the_answer() {
    let dir = sandbox();
    let cli = fake_cli(&dir);
    let mut state = studio(&dir);
    let claude = agent("claude", Provider::Claude);

    open_chat(&mut state, &cli, &claude);
    let chat = &state.chat.chats[0];
    assert!(
        !chat.preparing && chat.problem.is_none(),
        "{:?}",
        chat.problem
    );
    assert_eq!(
        chat.model_id.as_deref(),
        Some("sonnet"),
        "the agent's default model"
    );
    assert_eq!(
        chat.commands.len(),
        1,
        "the project's skill is offered as /revisar"
    );
    assert!(state.panes.shows_chat("claude"), "the chat has a tab");

    say(&mut state, "hola");
    assert!(state.chat.chats[0].turn.is_some(), "Enter starts a turn");
    wait_for_answer(&mut state);

    let chat = &state.chat.chats[0];
    let kinds: Vec<_> = chat.model.entries.iter().map(|e| e.kind).collect();
    assert_eq!(kinds, [model_kind::User, model_kind::Assistant]);
    assert_eq!(chat.model.entries[0].text, "hola");
    assert!(chat.model.entries[1].text.contains("agente de prueba"));
    assert_eq!(
        (
            chat.model.usage.input_tokens,
            chat.model.usage.output_tokens
        ),
        (12, 7)
    );
    assert_eq!(
        chat.model.session_id, "s-1",
        "the session is kept for the next turn"
    );
    assert!(
        chat.composer.text().trim().is_empty(),
        "the box is cleared after sending"
    );
    assert!(
        !chat.durations.is_empty(),
        "the answer shows how long it took"
    );
}

use crate::features::chat_model::Kind as model_kind;

#[test]
fn a_project_command_travels_as_typed_and_a_chat_command_stays_in_the_app() {
    let dir = sandbox();
    let cli = fake_cli(&dir);
    let mut state = studio(&dir);
    open_chat(&mut state, &cli, &agent("claude", Provider::Claude));

    say(&mut state, "/revisar src");
    wait_for_answer(&mut state);
    let prompts = std::fs::read_to_string(dir.join("prompts.log")).unwrap();
    assert!(prompts.contains("/revisar src"), "{prompts}");
    assert_eq!(state.chat.chats[0].model.entries[0].text, "/revisar src");

    let before = std::fs::read_to_string(dir.join("prompts.log")).unwrap();
    say(&mut state, "/modelo");
    assert!(state.chat.chats[0].models_open, "/modelo opens the picker");
    assert!(state.chat.chats[0].turn.is_none());
    say(&mut state, "/skills");
    let last = state.chat.chats[0].model.entries.last().unwrap();
    assert!(last.text.contains("/revisar"), "{}", last.text);
    assert_eq!(
        std::fs::read_to_string(dir.join("prompts.log")).unwrap(),
        before,
        "chat commands never reach the agent"
    );
}

#[test]
fn two_chats_keep_their_own_model() {
    let dir = sandbox();
    let cli = fake_cli(&dir);
    let mut state = studio(&dir);
    open_chat(&mut state, &cli, &agent("claude", Provider::Claude));
    open_chat(&mut state, &cli, &agent("opencode", Provider::OpenCode));
    let _ = chat::update(
        &mut state,
        chat::Event::For(
            "opencode".into(),
            Box::new(chat::Event::Model("ollama/x".into())),
        ),
    );
    let ids: Vec<_> = state
        .chat
        .chats
        .iter()
        .map(|c| c.model_id.clone())
        .collect();
    assert_eq!(
        ids,
        [Some("sonnet".to_owned()), Some("ollama/x".to_owned())]
    );
}

#[test]
fn local_models_from_the_diagnosis_to_a_chat_with_the_installed_model() {
    let dir = sandbox();
    let cli = fake_cli(&dir);
    let mut state = studio(&dir);

    let snapshot = models::snapshot(&cli, &dir).expect("the diagnosis reads");
    let _ = local::update(
        &mut state,
        local::Event::Loaded(Box::new(Ok(snapshot)), Some(cli.clone())),
    );
    assert!(state.local.snapshot.as_ref().unwrap().models[0].recommended);

    let _ = local::update(&mut state, local::Event::Install("qwen2.5-coder:7b".into()));
    assert!(state.local.busy(), "the install runs in the background");
    until("the install to finish", || {
        let _ = local::update(&mut state, local::Event::Tick);
        !state.local.busy()
    });
    assert!(state.local.error.is_none(), "{:?}", state.local.error);
    assert!(state.local.result.as_deref().unwrap().contains("conectado"));

    // Once connected, its chat opens with that model chosen.
    let opencode = agent("opencode", Provider::OpenCode);
    let name = "qwen2.5-coder:7b-ctx32k";
    let _ = chat::update(
        &mut state,
        chat::Event::OpenWith(opencode, local::model_ref(name)),
    );
    let chat = &state.chat.chats[0];
    assert_eq!(
        chat.model_id.as_deref(),
        Some("ollama/qwen2.5-coder:7b-ctx32k")
    );
}

#[test]
fn updating_studio_from_the_menu_to_the_restart_notice() {
    let dir = sandbox();
    let cli = fake_cli(&dir);
    let mut state = studio(&dir);

    let _ = updater::update(&mut state, updater::Event::Open);
    assert_eq!(state.update.phase, updater::Phase::Checking);
    let _ = updater::update(&mut state, updater::Event::Checked(updates::check(&cli)));
    assert!(matches!(state.update.phase, updater::Phase::Available(_)));
    assert!(updater::dialog(&state).is_some());

    let _ = updater::update(&mut state, updater::Event::Apply);
    let _ = updater::update(&mut state, updater::Event::Close);
    assert!(
        matches!(state.update.phase, updater::Phase::Applying(_)),
        "cannot be dismissed mid-install"
    );
    let _ = updater::update(&mut state, updater::Event::Applied(updates::apply(&cli)));
    assert!(matches!(&state.update.phase, updater::Phase::Done(i) if i.latest == "0.6.0a1"));
    let _ = updater::update(&mut state, updater::Event::Close);
    assert!(updater::dialog(&state).is_none());
}

#[test]
fn opening_a_file_brings_the_editor_back_over_chats_and_a_markdown_file_previews() {
    let dir = sandbox();
    let cli = fake_cli(&dir);
    let mut state = studio(&dir);
    open_chat(&mut state, &cli, &agent("claude", Provider::Claude));
    let pane = state.panes.find(|c| *c == panes::Content::Editor).unwrap();
    assert_ne!(
        state.panes.grid.get(pane).unwrap().current(),
        Some(&panes::Content::Editor),
        "the chat tab is in front"
    );

    let _ = actions::update(
        &mut state,
        Message::Editor(editor::Event::Open("README.md".into())),
    );
    assert_eq!(
        state.panes.grid.get(pane).unwrap().current(),
        Some(&panes::Content::Editor),
        "opening a file shows the editor"
    );

    // The file arrives (what the background read sends back) and is shown beside its preview.
    let document = crate::features::project_files::read(&dir, Path::new("README.md")).unwrap();
    let _ = editor::update(&mut state, editor::Event::Loaded(Ok(document)));
    let _ = editor::update(
        &mut state,
        editor::Event::Mode(editor::tools::ViewMode::Split),
    );
    let preview = state
        .editor
        .preview
        .as_ref()
        .expect("the preview is rendered");
    assert_eq!(preview.0, Path::new("README.md"));
}
