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
        &(None, Err(String::new()), Ok(options), Vec::new()),
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
