use super::*;
use serde_json::json;

fn event(mut fields: Value) -> Value {
    fields["schema"] = json!(SCHEMA);
    fields["schema_version"] = json!(SCHEMA_VERSION);
    fields
}

#[test]
fn foreign_or_future_events_are_ignored() {
    let mut model = Model::default();
    assert!(!model.apply(&json!({"type": "message", "text": "x"})));
    assert!(
        !model
            .apply(&json!({"schema": SCHEMA, "schema_version": 2, "type": "message", "text": "x"}))
    );
    assert!(
        !model
            .apply(&json!({"schema": "otro", "schema_version": 1, "type": "message", "text": "x"}))
    );
    assert!(model.entries.is_empty());
}

#[test]
fn messages_reasoning_and_notices_become_entries_and_empty_ones_are_dropped() {
    let mut model = Model::default();
    model.add_user("hola");
    assert!(model.apply(&event(json!({"type": "start", "agent": "codex"}))));
    assert!(model.apply(&event(
        json!({"type": "message", "role": "assistant", "text": "respuesta"})
    )));
    assert!(model.apply(&event(json!({"type": "reasoning", "text": "pienso"}))));
    assert!(model.apply(&event(
        json!({"type": "notice", "message": "Contexto liberado", "error": false})
    )));
    assert!(model.apply(&event(
        json!({"type": "notice", "message": "falló algo", "error": true})
    )));
    assert!(model.apply(&event(json!({"type": "message", "text": "   "}))));

    let kinds: Vec<_> = model.entries.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        [
            Kind::User,
            Kind::Assistant,
            Kind::Reasoning,
            Kind::Notice,
            Kind::Error
        ]
    );
}

#[test]
fn the_session_is_taken_from_session_and_done_events() {
    let mut model = Model::default();
    model.apply(&event(json!({"type": "session", "session_id": "s-1"})));
    assert_eq!(model.session_id, "s-1");
    model.apply(&event(
        json!({"type": "done", "exit_code": 0, "session_id": "s-2"}),
    ));
    assert_eq!(model.session_id, "s-2");
    model.apply(&event(
        json!({"type": "done", "exit_code": 0, "session_id": null}),
    ));
    assert_eq!(model.session_id, "s-2");
}

#[test]
fn tool_events_update_their_card_in_place() {
    let mut model = Model::default();
    model.apply(&event(json!({"type": "tool", "id": "t1", "kind": "command", "title": "bash: ls", "status": "running", "output": ""})));
    model.apply(&event(
        json!({"type": "tool", "id": "t2", "kind": "tool", "title": "leer", "status": "running"}),
    ));
    model.apply(&event(
        json!({"type": "tool", "id": "t1", "status": "completed", "output": "a.py\n"}),
    ));

    assert_eq!(model.entries.len(), 2);
    let first = &model.entries[0];
    assert_eq!(
        (
            first.status.as_str(),
            first.output.as_str(),
            first.text.as_str(),
            first.revision
        ),
        ("completed", "a.py\n", "bash: ls", 1)
    );
    assert_eq!(model.entries[1].revision, 0);
}

#[test]
fn usage_adds_up_over_turns_and_limits_keep_the_latest_known_values() {
    let mut model = Model::default();
    model.apply(&event(
        json!({"type": "usage", "input_tokens": 40_000, "output_tokens": 1_000, "cost_usd": 0.1}),
    ));
    model.apply(&event(
        json!({"type": "usage", "input_tokens": 500, "output_tokens": 500, "cost_usd": null}),
    ));
    model.apply(&event(json!({"type": "limits", "context_tokens": 90_000, "context_window": 100_000,
        "primary": {"used_percent": 12.5, "window_minutes": 300, "resets_at": 1}, "secondary": null, "plan": "pro"})));
    model.apply(&event(
        json!({"type": "limits", "context_tokens": 95_000, "primary": null}),
    ));

    let usage = &model.usage;
    assert_eq!((usage.input_tokens, usage.output_tokens), (40_500, 1_500));
    assert_eq!(
        usage.summary(),
        "40.5k de entrada · 1.5k de salida · US$ 0.1000"
    );
    assert_eq!(
        (usage.context_tokens, usage.context_window),
        (95_000, 100_000)
    );
    assert_eq!(usage.context_percent(), Some(95.0));
    assert_eq!(
        usage.primary.map(|w| w.used_percent),
        Some(12.5),
        "a later null does not erase it"
    );
    assert_eq!(usage.plan, "pro");
    assert_eq!(Model::default().usage.summary(), "");
    assert_eq!(Model::default().usage.context_percent(), None);
}

#[test]
fn tokens_are_written_compactly() {
    assert_eq!(compact_tokens(999), "999");
    assert_eq!(compact_tokens(1_000), "1k");
    assert_eq!(compact_tokens(1_234), "1.2k");
    assert_eq!(compact_tokens(1_500_000), "1.5M");
}

fn permission(id: &str, request: &str) -> Value {
    event(
        json!({"type": "permission", "id": id, "request_id": request, "tool": "Bash", "summary": "Ejecutar un comando: ls",
        "detail": "", "certain": true, "choices": [
            {"label": "Una vez", "allow": [], "network": false, "persist": false, "scope": "session"},
            {"label": "Siempre", "allow": ["Bash(ls:*)"], "network": true, "mode": "full-access", "persist": true}]}),
    )
}

#[test]
fn a_permission_card_carries_its_choices_and_is_not_asked_twice() {
    let mut model = Model::default();
    model.apply(&permission("p1", "r1"));
    model.apply(&permission("p1", "r1"));

    assert_eq!(model.entries.len(), 1);
    let card = &model.entries[0];
    assert!(card.certain && card.is_open_request() && model.awaiting_answer());
    assert_eq!(card.choices[0].scope.as_deref(), Some("session"));
    assert_eq!(
        card.choices[1],
        Choice {
            label: "Siempre".into(),
            allow: vec!["Bash(ls:*)".into()],
            network: true,
            mode: Some("full-access".into()),
            persist: true,
            scope: None
        }
    );

    model.decide(0, "Una vez");
    assert!(!model.awaiting_answer() && model.entries[0].decision == "Una vez");
    model.apply(&permission("p1", "r2")); // answered: a new request for the same tool may ask again
    assert_eq!(model.entries.len(), 2);
}

#[test]
fn unanswered_live_requests_expire_when_the_turn_ends() {
    let mut model = Model::default();
    model.apply(&permission("p1", "r1"));
    model.apply(&permission("p2", "")); // after the fact: nobody is waiting for an answer
    model.apply(&event(json!({"type": "question", "request_id": "q1", "id": "t3", "questions": [
        {"question": "Color?", "header": "Color", "multiSelect": false, "options": [{"label": "Rojo", "description": "r"}, {"label": "Azul"}]}]})));

    assert!(model.awaiting_answer());
    model.expire_live_requests();

    assert!(!model.awaiting_answer());
    assert_eq!(model.entries[0].decision, "Ya no esperaba tu respuesta");
    assert_eq!(
        model.entries[1].decision, "",
        "a card without request id stays answerable"
    );
    assert_eq!(model.entries[2].decision, "Ya no esperaba tu respuesta");
    assert_eq!(
        model.entries[2].questions[0].options,
        [
            ("Rojo".to_owned(), "r".to_owned()),
            ("Azul".to_owned(), String::new())
        ]
    );
}

#[test]
fn a_question_without_questions_is_dropped_and_not_now_is_recorded() {
    let mut model = Model::default();
    model.apply(&event(
        json!({"type": "question", "request_id": "q1", "questions": []}),
    ));
    assert!(model.entries.is_empty());
    model.apply(&permission("p1", "r1"));
    model.decide(0, "");
    assert_eq!(model.entries[0].decision, "Ahora no");
    model.decide(9, "x"); // out of range: nothing happens
}

#[test]
fn clearing_forgets_everything() {
    let mut model = Model::default();
    model.add_user("hola");
    model.apply(&event(json!({"type": "session", "session_id": "s"})));
    model.apply(&event(
        json!({"type": "usage", "input_tokens": 5, "output_tokens": 5}),
    ));
    model.clear();
    assert!(
        model.entries.is_empty() && model.session_id.is_empty() && model.usage == Usage::default()
    );
}

#[test]
fn a_huge_tool_output_is_kept_only_by_its_end() {
    let mut model = Model::default();
    let huge = format!("{}FIN", "x".repeat(200_000));
    model.apply(&event(
        json!({"type": "tool", "id": "t", "title": "cat", "status": "completed", "output": huge}),
    ));
    let kept = &model.entries[0].output;
    assert!(
        kept.chars().count() <= MAX_OUTPUT + 1 && kept.ends_with("FIN") && kept.starts_with('…')
    );
}

#[test]
fn old_entries_give_up_their_output_but_keep_their_place() {
    let mut model = Model::default();
    for n in 0..5 {
        model.apply(&event(
            json!({"type": "tool", "id": format!("t{n}"), "title": "ls", "output": "salida larga"}),
        ));
        model.apply(&event(json!({"type": "reasoning", "text": "pienso"})));
    }
    model.compact(4);
    assert_eq!(model.entries.len(), 10);
    assert!(
        model.entries[..6]
            .iter()
            .all(|e| e.kind != Kind::Tool || e.output.is_empty())
    );
    assert!(
        model.entries[..6]
            .iter()
            .all(|e| e.kind != Kind::Reasoning || e.text.is_empty())
    );
    assert_eq!(
        model.entries[8].output, "salida larga",
        "the latest stay whole"
    );
}

#[test]
fn live_usage_is_summed_by_message_replaced_on_repeat_and_cleared_by_the_final_figures() {
    let live = |id: &str, input: i64, output: i64| {
        json!({"schema": SCHEMA, "schema_version": SCHEMA_VERSION, "type": "usage_live",
               "message_id": id, "input_tokens": input, "output_tokens": output})
    };
    let mut model = Model::default();
    assert!(model.apply(&live("a", 100, 10)));
    assert!(model.apply(&live("b", 50, 5)));
    assert!(
        model.apply(&live("a", 120, 20)),
        "the same message reports again: it replaces"
    );
    assert_eq!(model.live_tokens(), (170, 25));
    assert_eq!(
        model.usage.input_tokens, 0,
        "lo en vivo no toca los totales"
    );
    model.apply(
        &json!({"schema": SCHEMA, "schema_version": SCHEMA_VERSION, "type": "usage",
                        "input_tokens": 400, "output_tokens": 30}),
    );
    assert_eq!(model.live_tokens(), (0, 0));
    assert_eq!(
        (model.usage.input_tokens, model.usage.output_tokens),
        (400, 30)
    );
}
