//! What the client shows, as plain data: the cards of a conversation, labels and times.
//! Nothing here touches the browser, so it is tested natively (`cargo test -p chxchx-remote`).

use chxchx_protocol::{ChatMessage, JobEvent, JobEventBody, JobSpec, JobState, Provider};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Assistant(String),
    Reasoning(String),
    Tool(ToolCard),
    Permission(PermissionCard),
    Question(QuestionCard),
    Notice(String),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCard {
    pub id: String,
    pub title: String,
    /// What the agent reports: `running`, `completed`, `failed`… (empty when it does not say).
    pub status: String,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub label: String,
    pub scope: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Decision {
    /// Waiting for the person.
    Open,
    Answered(String),
    /// The job ended before anyone answered.
    Expired,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PermissionCard {
    /// Without one the agent cannot be answered any more: the card only informs.
    pub request_id: Option<String>,
    pub tool: String,
    pub summary: String,
    pub detail: String,
    pub choices: Vec<Choice>,
    pub decision: Decision,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub question: String,
    pub header: String,
    pub multi: bool,
    pub options: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestionCard {
    pub request_id: String,
    pub asks: Vec<Ask>,
    pub decision: Decision,
}

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_owned()
}

/// The conversation a job's events tell, in order. `terminal` is whether the job is over: a card
/// nobody answered by then can no longer be answered.
pub fn fold(events: &[JobEvent], terminal: bool) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for event in events {
        match &event.body {
            JobEventBody::State { state } => {
                if *state == JobState::Cancelled {
                    items.push(Item::Notice("Cancelado".into()));
                }
            }
            JobEventBody::Progress { message, .. } => items.push(Item::Notice(message.clone())),
            JobEventBody::Error { error } => items.push(Item::Error(error.message.clone())),
            JobEventBody::Agent { event } => apply_agent(&mut items, event),
        }
    }
    if terminal {
        for item in &mut items {
            match item {
                Item::Permission(card)
                    if card.decision == Decision::Open && card.request_id.is_some() =>
                {
                    card.decision = Decision::Expired;
                }
                Item::Question(card) if card.decision == Decision::Open => {
                    card.decision = Decision::Expired;
                }
                _ => {}
            }
        }
    }
    items
}

fn apply_agent(items: &mut Vec<Item>, event: &Value) {
    match event["type"].as_str().unwrap_or_default() {
        "message" if event["role"] == "assistant" => {
            let body = text(event, "text");
            if !body.trim().is_empty() {
                items.push(Item::Assistant(body));
            }
        }
        "reasoning" => {
            let body = text(event, "text");
            if !body.trim().is_empty() {
                items.push(Item::Reasoning(body));
            }
        }
        "notice" => {
            let message = text(event, "message");
            if !message.is_empty() {
                items.push(if event["error"].as_bool() == Some(true) {
                    Item::Error(message)
                } else {
                    Item::Notice(message)
                });
            }
        }
        "error" => {
            let message = text(event, "message");
            if !message.is_empty() {
                items.push(Item::Error(message));
            }
        }
        "tool" => {
            let id = text(event, "id");
            // Later events about the same tool update its card in place.
            let existing = (!id.is_empty())
                .then(|| {
                    items.iter_mut().rev().find_map(|item| match item {
                        Item::Tool(card) if card.id == id => Some(card),
                        _ => None,
                    })
                })
                .flatten();
            match existing {
                Some(card) => {
                    card.status = text(event, "status");
                    let output = text(event, "output");
                    if !output.is_empty() {
                        card.output = output;
                    }
                    let title = text(event, "title");
                    if !title.is_empty() {
                        card.title = title;
                    }
                }
                None => items.push(Item::Tool(ToolCard {
                    id,
                    title: text(event, "title"),
                    status: text(event, "status"),
                    output: text(event, "output"),
                })),
            }
        }
        "permission" => {
            let id = text(event, "request_id");
            let choices = event["choices"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|choice| {
                    let label = text(choice, "label");
                    (!label.is_empty()).then(|| Choice {
                        label,
                        scope: choice["scope"].as_str().map(str::to_owned),
                    })
                })
                .collect();
            items.push(Item::Permission(PermissionCard {
                request_id: (!id.is_empty()).then_some(id),
                tool: text(event, "tool"),
                summary: text(event, "summary"),
                detail: text(event, "detail"),
                choices,
                decision: Decision::Open,
            }));
        }
        "question" => {
            let asks: Vec<Ask> = event["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|ask| ask["question"].is_string())
                .map(|ask| Ask {
                    question: text(ask, "question"),
                    header: text(ask, "header"),
                    multi: ask["multiSelect"].as_bool() == Some(true),
                    options: ask["options"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|option| (text(option, "label"), text(option, "description")))
                        .collect(),
                })
                .collect();
            let id = text(event, "request_id");
            if !asks.is_empty() && !id.is_empty() {
                items.push(Item::Question(QuestionCard {
                    request_id: id,
                    asks,
                    decision: Decision::Open,
                }));
            }
        }
        "answered" => {
            let id = text(event, "request_id");
            let automatic = event["automatic"].as_bool() == Some(true);
            let label = match event["behavior"].as_str() {
                Some("allow") => "Permitido".to_owned(),
                Some("deny") if automatic => "Rechazado automáticamente".to_owned(),
                Some("deny") => "Rechazado".to_owned(),
                _ => "Respondida".to_owned(),
            };
            for item in items.iter_mut().rev() {
                match item {
                    Item::Permission(card) if card.request_id.as_deref() == Some(&id) => {
                        card.decision = Decision::Answered(label);
                        break;
                    }
                    Item::Question(card) if card.request_id == id => {
                        card.decision = Decision::Answered(label);
                        break;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// The agent session the events announced last, to continue the conversation.
pub fn session_of(events: &[JobEvent]) -> Option<String> {
    events.iter().rev().find_map(|event| match &event.body {
        JobEventBody::Agent { event } if event["type"] == "session" => event["session_id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(str::to_owned),
        _ => None,
    })
}

/// How many cards wait for the person.
pub fn waiting(items: &[Item]) -> usize {
    items
        .iter()
        .filter(|item| match item {
            Item::Permission(card) => card.request_id.is_some() && card.decision == Decision::Open,
            Item::Question(card) => card.decision == Decision::Open,
            _ => false,
        })
        .count()
}

pub fn state_label(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "En cola",
        JobState::Running => "En curso",
        JobState::Succeeded => "Terminado",
        JobState::Failed => "Falló",
        JobState::Cancelled => "Cancelado",
        JobState::Interrupted => "Interrumpido",
    }
}

pub fn state_class(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Succeeded => "ok",
        JobState::Failed | JobState::Interrupted => "bad",
        JobState::Cancelled => "muted",
    }
}

pub fn provider_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "Claude",
        Provider::Codex => "Codex",
        Provider::OpenCode => "OpenCode",
    }
}

pub fn role_label(role: chxchx_protocol::Role) -> &'static str {
    match role {
        chxchx_protocol::Role::Reader => "Lector",
        chxchx_protocol::Role::Operator => "Operador",
        chxchx_protocol::Role::Admin => "Administrador",
    }
}

/// A job in one line: what it does and where.
pub fn describe(spec: &JobSpec) -> String {
    match spec {
        JobSpec::AgentTurn(turn) => {
            format!("{} en {}", provider_label(turn.provider), turn.project)
        }
        JobSpec::WorkspaceStart { project } => format!("Iniciar el espacio de {project}"),
        JobSpec::WorkspaceStop { project } => format!("Detener el espacio de {project}"),
        JobSpec::LocalModelInstall { model } => format!("Instalar el modelo {model}"),
        JobSpec::LocalModelRemove { model } => format!("Quitar el modelo {model}"),
    }
}

pub fn ago(now_ms: u64, then_ms: u64) -> String {
    let seconds = now_ms.saturating_sub(then_ms) / 1000;
    match seconds {
        0..5 => "ahora".to_owned(),
        5..60 => format!("hace {seconds} s"),
        60..3600 => format!("hace {} min", seconds / 60),
        3600..86_400 => format!("hace {} h", seconds / 3600),
        _ => format!("hace {} d", seconds / 86_400),
    }
}

/// The saved messages without the ones a live job already shows. A turn that was running when the
/// chat opened had its user message saved already; its live card shows it, so it is dropped here.
pub fn without_running(
    mut messages: Vec<ChatMessage>,
    running_prompts: &[String],
) -> Vec<ChatMessage> {
    for prompt in running_prompts {
        let wanted = prompt.trim();
        if let Some(at) = messages
            .iter()
            .rposition(|message| message.role == "user" && message.text.trim() == wanted)
        {
            messages.remove(at);
        }
    }
    messages
}

/// The first `max` characters, with an ellipsis when something was cut.
pub fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;
    use chxchx_protocol::{JobError, JobId};
    use serde_json::json;

    fn agent(seq: u64, event: Value) -> JobEvent {
        JobEvent {
            job: JobId::parse("j1").unwrap(),
            seq,
            at_ms: 0,
            body: JobEventBody::Agent { event },
        }
    }

    fn events(bodies: Vec<Value>) -> Vec<JobEvent> {
        bodies
            .into_iter()
            .enumerate()
            .map(|(index, event)| agent(index as u64 + 1, event))
            .collect()
    }

    #[test]
    fn a_turn_becomes_messages_and_tool_cards_updated_in_place() {
        let items = fold(
            &events(vec![
                json!({"type": "session", "session_id": "s-9"}),
                json!({"type": "message", "role": "assistant", "text": "Voy a revisar."}),
                json!({"type": "tool", "id": "t1", "title": "cargo test", "status": "running"}),
                json!({"type": "tool", "id": "t1", "status": "completed", "output": "ok"}),
                json!({"type": "message", "role": "assistant", "text": "   "}),
                json!({"type": "message", "role": "user", "text": "no es del agente"}),
                json!({"type": "message", "role": "assistant", "text": "Listo."}),
            ]),
            true,
        );
        assert_eq!(
            items.len(),
            3,
            "the blank and the user message are left out: {items:?}"
        );
        assert_eq!(items[0], Item::Assistant("Voy a revisar.".into()));
        assert_eq!(
            items[1],
            Item::Tool(ToolCard {
                id: "t1".into(),
                title: "cargo test".into(),
                status: "completed".into(),
                output: "ok".into(),
            })
        );
        assert_eq!(items[2], Item::Assistant("Listo.".into()));
    }

    #[test]
    fn a_permission_waits_until_it_is_answered_and_then_shows_what_was_decided() {
        let ask = json!({
            "type": "permission", "request_id": "req-1", "tool": "Bash", "summary": "cargo test",
            "detail": "cargo test --all",
            "choices": [{"label": "Permitir esta vez", "scope": "once"}, {"label": "", "scope": "x"}]
        });
        let open = fold(&events(vec![ask.clone()]), false);
        let Item::Permission(card) = &open[0] else {
            panic!("{open:?}")
        };
        assert_eq!(card.decision, Decision::Open);
        assert_eq!(
            card.choices,
            [Choice {
                label: "Permitir esta vez".into(),
                scope: Some("once".into())
            }]
        );
        assert_eq!(waiting(&open), 1);

        let allowed = fold(
            &events(vec![
                ask.clone(),
                json!({"type": "answered", "request_id": "req-1", "behavior": "allow"}),
            ]),
            false,
        );
        let Item::Permission(card) = &allowed[0] else {
            panic!()
        };
        assert_eq!(card.decision, Decision::Answered("Permitido".into()));
        assert_eq!(waiting(&allowed), 0);

        let timed_out = fold(
            &events(vec![
                ask.clone(),
                json!({
                    "type": "answered", "request_id": "req-1", "behavior": "deny", "automatic": true
                }),
            ]),
            true,
        );
        let Item::Permission(card) = &timed_out[0] else {
            panic!()
        };
        assert_eq!(
            card.decision,
            Decision::Answered("Rechazado automáticamente".into())
        );

        let ended = fold(&events(vec![ask]), true);
        let Item::Permission(card) = &ended[0] else {
            panic!()
        };
        assert_eq!(card.decision, Decision::Expired, "the job ended first");
        assert_eq!(waiting(&ended), 0);
    }

    #[test]
    fn a_card_without_a_request_only_informs() {
        let items = fold(
            &events(vec![
                json!({"type": "permission", "tool": "Bash", "summary": "ls"}),
            ]),
            false,
        );
        let Item::Permission(card) = &items[0] else {
            panic!()
        };
        assert_eq!(card.request_id, None);
        assert_eq!(waiting(&items), 0, "nothing can be answered");
    }

    #[test]
    fn a_question_keeps_its_options_and_is_closed_by_its_answer() {
        let question = json!({
            "type": "question", "request_id": "q-1",
            "questions": [{
                "question": "¿Qué base de datos?", "header": "BD", "multiSelect": false,
                "options": [{"label": "PostgreSQL", "description": "Servidor"}, {"label": "SQLite"}]
            }, {"nope": true}]
        });
        let open = fold(&events(vec![question.clone()]), false);
        let Item::Question(card) = &open[0] else {
            panic!("{open:?}")
        };
        assert_eq!(card.asks.len(), 1);
        assert_eq!(
            card.asks[0].options[0],
            ("PostgreSQL".into(), "Servidor".into())
        );
        assert_eq!(card.asks[0].options[1], ("SQLite".into(), String::new()));
        assert_eq!(waiting(&open), 1);
        let done = fold(
            &events(vec![
                question,
                json!({"type": "answered", "request_id": "q-1", "answers": {"x": "y"}}),
            ]),
            false,
        );
        let Item::Question(card) = &done[0] else {
            panic!()
        };
        assert_eq!(card.decision, Decision::Answered("Respondida".into()));
        assert!(
            fold(
                &events(vec![
                    json!({"type": "question", "request_id": "q", "questions": []})
                ]),
                false
            )
            .is_empty()
        );
    }

    #[test]
    fn errors_notices_and_cancellation_are_shown() {
        let mut all = events(vec![
            json!({"type": "notice", "message": "Contexto liberado"}),
            json!({"type": "notice", "message": "Falló algo", "error": true}),
            json!({"type": "error", "message": "Sin red"}),
        ]);
        all.push(JobEvent {
            job: JobId::parse("j1").unwrap(),
            seq: 4,
            at_ms: 0,
            body: JobEventBody::Error {
                error: JobError::new(
                    chxchx_protocol::ErrorCode::ProviderFailed,
                    "El agente falló",
                ),
            },
        });
        all.push(JobEvent {
            job: JobId::parse("j1").unwrap(),
            seq: 5,
            at_ms: 0,
            body: JobEventBody::State {
                state: JobState::Cancelled,
            },
        });
        let items = fold(&all, true);
        assert_eq!(
            items,
            [
                Item::Notice("Contexto liberado".into()),
                Item::Error("Falló algo".into()),
                Item::Error("Sin red".into()),
                Item::Error("El agente falló".into()),
                Item::Notice("Cancelado".into()),
            ]
        );
    }

    #[test]
    fn the_session_is_the_last_one_announced() {
        let all = events(vec![
            json!({"type": "session", "session_id": "a"}),
            json!({"type": "message", "role": "assistant", "text": "x"}),
            json!({"type": "session", "session_id": "b"}),
            json!({"type": "session", "session_id": ""}),
        ]);
        assert_eq!(session_of(&all).as_deref(), Some("b"));
        assert_eq!(session_of(&[]), None);
    }

    #[test]
    fn a_running_turn_is_not_shown_twice() {
        let message = |role: &str, text: &str| ChatMessage {
            role: role.into(),
            text: text.into(),
        };
        let saved = vec![
            message("user", "hola"),
            message("assistant", "buenas"),
            message("user", "revisa esto"),
        ];
        let shown = without_running(saved.clone(), &["  revisa esto ".to_owned()]);
        assert_eq!(shown, saved[..2], "only the turn that is running live");
        let same_words_earlier = vec![
            message("user", "hola"),
            message("assistant", "x"),
            message("user", "hola"),
        ];
        let shown = without_running(same_words_earlier, &["hola".to_owned()]);
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].text, "hola", "the older identical message stays");
        assert_eq!(without_running(saved.clone(), &[]), saved);
        assert_eq!(
            without_running(saved.clone(), &["otra cosa".to_owned()]),
            saved
        );
    }

    #[test]
    fn labels_times_and_clipping_read_naturally() {
        assert_eq!(state_label(JobState::Interrupted), "Interrumpido");
        assert_eq!(state_class(JobState::Failed), "bad");
        assert_eq!(ago(10_000, 9_000), "ahora");
        assert_eq!(ago(100_000, 70_000), "hace 30 s");
        assert_eq!(ago(10 * 60_000, 0), "hace 10 min");
        assert_eq!(ago(3 * 3_600_000, 0), "hace 3 h");
        assert_eq!(ago(2 * 86_400_000, 0), "hace 2 d");
        assert_eq!(
            ago(0, 5_000),
            "ahora",
            "a clock a little behind never goes negative"
        );
        assert_eq!(clip("hola", 10), "hola");
        assert_eq!(clip("ñandú grande", 5), "ñandú…");
    }
}
