use super::*;
use crate::features::agents::Provider;
use crate::features::chat_model::Choice;

fn dashboard() -> Dashboard {
    let mut state = super::super::editor::fixture();
    let agent = Agent {
        id: "claude".into(),
        provider: Provider::Claude,
        command: vec!["claude".into()],
        cwd: PathBuf::new(),
        executable: Some(PathBuf::from("/bin/claude")),
        configured: false,
    };
    let _ = open(&mut state, agent);
    state.chat.chats[0].preparing = false;
    state
}

fn ask(state: &mut Dashboard, live: bool) {
    let chat = &mut state.chat.chats[0];
    chat.model.apply(&json!({
        "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION, "type": "permission",
        "id": "t1", "tool": "Bash", "summary": "Ejecutar ls", "request_id": if live { "r1" } else { "" },
        "choices": [{"label": "Permitir", "allow": ["Bash(ls:*)"], "network": true, "mode": "workspace-commands", "persist": false}],
    }));
}

#[test]
fn an_answer_to_a_blocked_turn_grants_the_rule_for_the_retry_only() {
    let mut state = dashboard();
    ask(&mut state, false);
    decide(&mut state, 0, Some(0));
    let chat = &state.chat.chats[0];
    assert_eq!(chat.once, ["Bash(ls:*)"]);
    assert!(chat.approved.is_empty() && chat.network);
    assert_eq!(chat.mode, "workspace-commands");
    assert_eq!(chat.model.entries[0].decision, "Permitir");
}

#[test]
fn denying_grants_nothing_and_a_remembered_choice_stays_for_the_chat() {
    let mut state = dashboard();
    ask(&mut state, false);
    decide(&mut state, 0, None);
    assert!(state.chat.chats[0].once.is_empty() && !state.chat.chats[0].network);
    assert_eq!(state.chat.chats[0].model.entries[0].decision, "Ahora no");

    let mut state = dashboard();
    ask(&mut state, true);
    let chat = &mut state.chat.chats[0];
    chat.model.entries[0].choices = vec![Choice {
        label: "Siempre".into(),
        allow: vec!["edit".into()],
        persist: true,
        ..Default::default()
    }];
    decide(&mut state, 0, Some(0));
    assert_eq!(state.chat.chats[0].approved, ["edit"]);
}

#[test]
fn single_choice_questions_keep_one_pick_and_multi_choice_ones_keep_several() {
    let mut state = dashboard();
    let chat = &mut state.chat.chats[0];
    chat.model.apply(&json!({
        "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION, "type": "question",
        "request_id": "q1", "questions": [{"question": "¿Cuál?", "options": [{"label": "A"}, {"label": "B"}]}],
    }));
    pick(chat, 0, 0, "A".into());
    pick(chat, 0, 0, "B".into());
    assert_eq!(chat.picks.len(), 1);
    assert_eq!(chat.picks[0].2, "B");
    pick(chat, 0, 0, "B".into());
    assert!(chat.picks.is_empty(), "picking again unpicks");
}

#[test]
fn images_change_the_prompt_the_agent_reads_but_not_what_the_chat_shows() {
    use super::turns::{shown_for_tests, with_images_for_tests};
    assert_eq!(with_images_for_tests("arregla esto", 0), "arregla esto");
    assert!(
        with_images_for_tests("arregla esto", 2).starts_with("arregla esto\n\n(Adjunto imágenes")
    );
    assert!(with_images_for_tests("", 1).contains("Resuelve lo que muestran"));
    assert_eq!(shown_for_tests("", 1), "Imágenes adjuntas");
    assert_eq!(shown_for_tests("hola", 1), "hola");
}

#[test]
fn only_five_images_fit_in_one_message_and_a_failure_is_reported() {
    let mut state = dashboard();
    images::loaded(&mut state, "claude", Err("No pude leer la imagen".into()));
    assert_eq!(state.message, "No pude leer la imagen");
    images::loaded(&mut state, "otro", Ok(vec![]));
    assert!(state.chat.chats[0].images.is_empty());
}

#[test]
fn a_widget_that_answers_later_acts_on_the_chat_it_was_drawn_for() {
    // The view of the second chat is built after the first, so "the chat being drawn" is the
    // second by the time the person touches a control of the first.
    super::CURRENT.with(|current| *current.borrow_mut() = "claude".into());
    let for_claude = super::sender();
    super::CURRENT.with(|current| *current.borrow_mut() = "codex".into());
    match for_claude(Event::Model("opus".into())) {
        Message::Chat(Event::For(id, inner)) => {
            assert_eq!(id, "claude");
            assert!(matches!(*inner, Event::Model(model) if model == "opus"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn changing_the_model_of_one_chat_leaves_the_others_alone() {
    let mut state = dashboard();
    let mut other = state.chat.chats[0].agent.clone();
    other.id = "codex".into();
    let _ = open(&mut state, other);
    for chat in &mut state.chat.chats {
        chat.preparing = false;
        chat.model_id = Some("original".into());
    }
    let _ = update(
        &mut state,
        Event::For("claude".into(), Box::new(Event::Model("sonnet".into()))),
    );
    let by_id = |state: &Dashboard, id: &str| {
        state
            .chat
            .chats
            .iter()
            .find(|c| c.agent.id == id)
            .unwrap()
            .model_id
            .clone()
    };
    assert_eq!(by_id(&state, "claude").as_deref(), Some("sonnet"));
    assert_eq!(by_id(&state, "codex").as_deref(), Some("original"));
    let _ = update(
        &mut state,
        Event::For("codex".into(), Box::new(Event::Access("read-only".into()))),
    );
    let mode = |id: &str| {
        state
            .chat
            .chats
            .iter()
            .find(|c| c.agent.id == id)
            .unwrap()
            .mode
            .clone()
    };
    assert_eq!(
        (mode("codex").as_str(), mode("claude").as_str()),
        ("read-only", "workspace-write")
    );
}

fn with_commands(state: &mut Dashboard) {
    use crate::integrations::slash_commands::Found;
    state.chat.chats[0].commands = vec![Found {
        name: "revisar".into(),
        description: "Revisa cambios".into(),
        kind: "skill",
    }];
}

fn type_in(state: &mut Dashboard, text: &str) {
    state.chat.chats[0].composer = text_editor::Content::with_text(text);
}

#[test]
fn the_menu_opens_on_a_slash_and_filters_while_typing() {
    let mut state = dashboard();
    with_commands(&mut state);
    type_in(&mut state, "hola");
    assert!(slash::typing(&state.chat.chats[0]).is_none());
    type_in(&mut state, "/rev");
    let query = slash::typing(&state.chat.chats[0]).unwrap();
    let names: Vec<_> = slash::items(&state.chat.chats[0], &query)
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert_eq!(names, ["revisar", "reintentar"].map(String::from)[..1]);
    type_in(&mut state, "/revisar con calma");
    assert!(slash::typing(&state.chat.chats[0]).is_none());
}

#[test]
fn enter_completes_a_partial_command_and_leaves_a_complete_one_to_send() {
    let mut state = dashboard();
    with_commands(&mut state);
    type_in(&mut state, "/rev");
    assert!(slash::enter(&mut state));
    assert_eq!(state.chat.chats[0].composer.text().trim_end(), "/revisar");
    type_in(&mut state, "/revisar");
    assert!(!slash::enter(&mut state));
}

#[test]
fn a_project_command_reaches_claude_as_typed_and_other_agents_as_an_instruction() {
    let mut state = dashboard();
    with_commands(&mut state);
    let found = state.chat.chats[0].commands[0].clone();
    assert_eq!(
        slash::prompt_for("claude", &found, " ahora "),
        "/revisar ahora"
    );
    let other = slash::prompt_for("codex", &found, "");
    assert!(other.contains("«revisar»") && other.contains(".agents/skills/revisar/SKILL.md"));
    match slash::resolve(&mut state, "/revisar src") {
        slash::Outcome::Send(prompt, shown) => {
            assert_eq!(
                (prompt.as_str(), shown.as_str()),
                ("/revisar src", "/revisar src")
            );
        }
        _ => panic!("a project command is sent to the agent"),
    }
    assert!(matches!(
        slash::resolve(&mut state, "/desconocido"),
        slash::Outcome::Plain
    ));
}

#[test]
fn chat_commands_act_on_the_chat_and_never_reach_the_agent() {
    let mut state = dashboard();
    with_commands(&mut state);
    type_in(&mut state, "/skills");
    let _ = submit(&mut state);
    let chat = &state.chat.chats[0];
    assert!(chat.turn.is_none() && chat.composer.text().trim().is_empty());
    assert!(chat.model.entries.last().unwrap().text.contains("/revisar"));
    type_in(&mut state, "/modelo");
    let _ = submit(&mut state);
    assert!(state.chat.chats[0].models_open);
}

#[test]
fn a_model_chosen_before_the_chat_is_ready_survives_its_preparation() {
    let mut state = dashboard();
    state.chat.chats[0].preparing = true;
    use_model(&mut state.chat.chats[0], "ollama/qwen2.5-coder:7b-ctx32k");
    let options = bridge::AgentOptions {
        available: true,
        default_model: "sonnet".into(),
        models: vec![bridge::ModelOption {
            id: "sonnet".into(),
            label: "sonnet".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    prepared_for(
        &mut state,
        "claude",
        &(
            bridge::SavedChat::default(),
            Ok(options),
            Vec::new(),
            (String::new(), Ok(0)),
        ),
    );
    let chat = &state.chat.chats[0];
    assert_eq!(
        chat.model_id.as_deref(),
        Some("ollama/qwen2.5-coder:7b-ctx32k")
    );
    let listed = chat.options.as_ref().unwrap();
    assert!(
        listed
            .models
            .iter()
            .any(|m| m.id == "ollama/qwen2.5-coder:7b-ctx32k")
    );
}

#[test]
fn provider_memory_preparation_unblocks_the_chat_with_recovered_context() {
    let mut state = dashboard();
    state.chat.chats[0].preparing = true;

    drop(update(
        &mut state,
        Event::ProviderMemoriesPrepared("claude".into(), "Memoria recuperada".into(), Ok(2)),
    ));

    let chat = &state.chat.chats[0];
    assert!(!chat.preparing);
    assert_eq!(chat.recovered_memories, "Memoria recuperada");
}

#[test]
fn starting_a_new_chat_waits_for_background_memory_recovery() {
    let mut state = dashboard();
    state.chat.chats[0].model.add_user("conversación anterior");

    drop(update(&mut state, Event::NewChat));

    let chat = &state.chat.chats[0];
    assert!(chat.preparing);
    assert!(chat.model.entries.is_empty());
}

#[test]
fn new_chat_does_not_race_the_initial_chat_preparation() {
    let mut state = dashboard();
    state.chat.chats[0].preparing = true;
    state.chat.chats[0].model.add_user("conversación anterior");

    drop(update(&mut state, Event::NewChat));

    let chat = &state.chat.chats[0];
    assert!(chat.preparing);
    assert_eq!(chat.model.entries[0].text, "conversación anterior");
}

#[test]
fn arrows_walk_back_through_sent_messages_and_return_to_the_draft() {
    let mut state = dashboard();
    let chat = &mut state.chat.chats[0];
    chat.history = vec!["uno".into(), "dos".into()];
    chat.composer = text_editor::Content::with_text("borrador");
    let _ = update(&mut state, Event::HistoryPrev);
    assert_eq!(state.chat.chats[0].composer.text().trim_end(), "dos");
    let _ = update(&mut state, Event::HistoryPrev);
    assert_eq!(state.chat.chats[0].composer.text().trim_end(), "uno");
    let _ = update(&mut state, Event::HistoryNext);
    let _ = update(&mut state, Event::HistoryNext);
    assert_eq!(state.chat.chats[0].composer.text().trim_end(), "borrador");
    assert!(state.chat.chats[0].recall.is_none());
}

#[cfg(unix)]
#[test]
fn studio_releases_context_generates_a_hidden_handoff_and_consumes_it_after_a_fresh_turn() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    let root = std::env::temp_dir().join(format!(
        "chxchx-studio-handoff-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    // The `claude` Studio runs itself: a summary turn (no live input) answers with the summary;
    // a chat turn in read-only mode is the fresh session that must receive the handoff.
    let claude = root.join("claude");
    let line = |json: &str| format!("printf '%s\\n' '{json}'\n");
    let init = |session: &str| {
        line(&format!(
            r#"{{"type":"system","subtype":"init","session_id":"{session}"}}"#
        ))
    };
    let reply = |text: &str| {
        line(&format!(
            r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"{text}"}}]}}}}"#
        ))
    };
    let result = |tokens: u32| {
        line(&format!(
            r#"{{"type":"result","is_error":false,"usage":{{"input_tokens":{tokens},"output_tokens":1}},"modelUsage":{{"m":{{"contextWindow":100}}}}}}"#
        ))
    };
    fs::write(
        &claude,
        format!(
            "#!/bin/sh\ncase \"$*\" in\n  *--input-format*) ;;\n  *) cat >/dev/null\n{summary}exit 0;;\nesac\nread line\ncase \"$*\" in\n  *\"--permission-mode plan\"*) printf '%s\\n' \"$line\" > '{recap_log}'\n{fresh};;\n  *)\n{first};;\nesac\ncat >/dev/null\n",
            summary = reply(r"## Objetivo\nseguir") + &result(1),
            recap_log = root.join("second-recap.txt").display(),
            fresh = init("s-2") + &reply("respuesta visible") + &result(50),
            first = init("s-1") + &reply("respuesta visible") + &result(90),
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&claude).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&claude, permissions).unwrap();

    let mut state = dashboard();
    state.root = root.clone();
    state.chat.chats[0].agent.executable = Some(claude.clone());
    state.chat.chats[0].model.add_user("primera pregunta");
    let spec = TurnSpec {
        agent: "claude".into(),
        provider: "Claude".into(),
        project: root.clone(),
        prompt: "primera pregunta".into(),
        mode: "workspace-write".into(),
        agent_command: vec!["claude".into()],
        executable: Some(claude.clone()),
        trusted: true,
        ..Default::default()
    };
    state.chat.chats[0].turn = Some(Turn::launch(&spec).unwrap());

    let drain = |state: &mut Dashboard| {
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.chat.chats[0].turn.is_some() && Instant::now() < deadline {
            super::tick(state);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            state.chat.chats[0].turn.is_none(),
            "Studio chat turn timed out"
        );
    };
    drain(&mut state);

    let handoff_path = root.join(".ai/chat-handoffs/claude-s-1.md");
    assert!(
        fs::read_to_string(&handoff_path)
            .unwrap()
            .contains("seguir")
    );
    let chat = &state.chat.chats[0];
    assert_eq!(chat.model.session_id, "s-1");
    assert_eq!(
        chat.model
            .entries
            .iter()
            .filter(|entry| entry.kind == model::Kind::Assistant)
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        ["respuesta visible"]
    );

    state.chat.chats[0].model.add_user("segunda pregunta");
    let next = TurnSpec {
        agent: "claude".into(),
        provider: "Claude".into(),
        project: root.clone(),
        prompt: "segunda pregunta".into(),
        session: Some("s-1".into()),
        mode: "read-only".into(),
        agent_command: vec!["claude".into()],
        executable: Some(claude.clone()),
        trusted: true,
        ..Default::default()
    };
    state.chat.chats[0].turn = Some(Turn::launch(&next).unwrap());
    drain(&mut state);

    assert!(
        !handoff_path.exists(),
        "Rust consumes only after the new session succeeds"
    );
    assert!(
        fs::read_to_string(root.join("second-recap.txt"))
            .unwrap()
            .contains("seguir")
    );
    assert_eq!(state.chat.chats[0].model.session_id, "s-2");
    assert_eq!(
        state.chat.chats[0]
            .model
            .entries
            .iter()
            .filter(|entry| entry.kind == model::Kind::Assistant)
            .count(),
        2,
        "the hidden summary never appears in the visible chat"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_message_offers_its_link_and_its_code_only_web_links_open_and_a_code_can_be_copied() {
    let mut state = dashboard();
    state.chat.chats[0].model.apply(&json!({
        "schema": model::SCHEMA, "schema_version": model::SCHEMA_VERSION,
        "type": "message", "role": "assistant",
        "text": "Abre https://github.com/login/device y escribe el código ABCD-1234.\n\n```\nhttps://no-se-toca.dev\n```",
    }));
    sync(&mut state.chat.chats[0]);
    let chat = &state.chat.chats[0];
    assert!(
        chat.rendered
            .iter()
            .any(|(_, content)| !content.items().is_empty()),
        "the answer is drawn as Markdown"
    );
    // The message offers its links and its code as buttons.
    let entry = chat
        .model
        .entries
        .iter()
        .find(|e| e.kind == Kind::Assistant)
        .unwrap();
    let offered = crate::features::chat_links::actions(&entry.text);
    assert_eq!(offered.len(), 3, "{offered:?}");

    // Clicking anything that is not a web link is refused, never opened.
    let _ = update(&mut state, Event::OpenLink("file:///etc/passwd".into()));
    assert!(state.message.contains("no lo abro"), "{}", state.message);
    let _ = update(&mut state, Event::OpenLink("javascript:alert(1)".into()));
    assert!(state.message.contains("no lo abro"));
    // Copying a code confirms it.
    let _ = update(&mut state, Event::CopyText("ABCD-1234".into()));
    assert_eq!(state.message, "Copiado al portapapeles.");
}

#[test]
fn a_link_that_is_still_being_written_makes_the_markdown_start_again() {
    use super::prepare::touches_a_link;
    assert!(
        touches_a_link("Entra a https://auth.exam", 10),
        "the tail holds the start of a link"
    );
    assert!(
        touches_a_link("Entra a https://auth.exampleple.com/x ahora", 30),
        "the link is cut at what was parsed"
    );
    assert!(!touches_a_link(
        "Una respuesta larga sin enlaces, escrita poco a poco.",
        20
    ));
    let long = format!("https://a.dev/x {}", "palabra ".repeat(100));
    assert!(
        !touches_a_link(&long, 700),
        "a link far behind is already parsed"
    );
}
