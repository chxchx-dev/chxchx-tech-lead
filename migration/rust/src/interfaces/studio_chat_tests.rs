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
