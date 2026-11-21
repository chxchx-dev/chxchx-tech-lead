//! The cards of a conversation: messages, tool runs, and the two things an agent can ask.

use crate::actions;
use crate::model::{self, Choice, Decision, Item, PermissionCard, QuestionCard, ToolCard};
use crate::store::toast;
use chxchx_protocol::{AnswerBehavior, JobAnswer, JobId};
use dioxus::prelude::*;
use std::collections::BTreeMap;

#[component]
pub fn ItemView(job: JobId, item: Item) -> Element {
    match item {
        Item::Assistant(text) => rsx! { div { class: "msg assistant", "{text}" } },
        Item::Reasoning(text) => rsx! {
            details { class: "fold",
                summary { "Razonamiento" }
                pre { "{model::clip(&text, 4000)}" }
            }
        },
        Item::Tool(card) => rsx! { ToolView { card } },
        Item::Permission(card) => rsx! { PermissionView { job, card } },
        Item::Question(card) => rsx! { QuestionView { job, card } },
        Item::Notice(text) => rsx! { div { class: "note", "{text}" } },
        Item::Error(text) => rsx! { div { class: "note bad", role: "alert", "{text}" } },
    }
}

#[component]
fn ToolView(card: ToolCard) -> Element {
    let (mark, class) = match card.status.as_str() {
        "completed" | "done" | "success" | "succeeded" => ("✓", "ok"),
        "failed" | "error" | "cancelled" => ("✗", "bad"),
        _ => ("…", "running"),
    };
    let title = if card.title.is_empty() {
        "Herramienta".to_owned()
    } else {
        card.title.clone()
    };
    rsx! {
        div { class: "tool {class}",
            div { class: "tool-title",
                span { class: "tool-mark", aria_hidden: "true", "{mark}" }
                span { "{model::clip(&title, 200)}" }
            }
            if !card.output.is_empty() {
                details { class: "fold",
                    summary { "Salida" }
                    pre { "{model::clip(&card.output, 3000)}" }
                }
            }
        }
    }
}

fn decide(job: JobId, answer: JobAnswer, mut busy: Signal<bool>) {
    busy.set(true);
    spawn(async move {
        if let Err(message) = actions::answer(job, answer).await {
            toast(message, true);
        }
        // The card may be gone by now (the job moved on): then there is nothing to release.
        if let Ok(mut flag) = busy.try_write() {
            *flag = false;
        }
    });
}

#[component]
fn PermissionView(job: JobId, card: PermissionCard) -> Element {
    let busy = use_signal(|| false);
    let controls = match (&card.decision, &card.request_id) {
        (Decision::Open, Some(request_id)) => {
            let choices: Vec<Choice> = if card.choices.is_empty() {
                vec![Choice {
                    label: "Permitir".into(),
                    scope: None,
                }]
            } else {
                card.choices.clone()
            };
            let deny_job = job.clone();
            let deny_id = request_id.clone();
            rsx! {
                div { class: "actions",
                    for (index, choice) in choices.into_iter().enumerate() {
                        button {
                            key: "{index}",
                            class: if index == 0 { "btn primary" } else { "btn" },
                            disabled: busy(),
                            onclick: {
                                let job = job.clone();
                                let request_id = request_id.clone();
                                move |_| decide(job.clone(), JobAnswer {
                                    request_id: request_id.clone(),
                                    behavior: Some(AnswerBehavior::Allow),
                                    scope: choice.scope.clone(),
                                    answers: None,
                                }, busy)
                            },
                            "{choice.label}"
                        }
                    }
                    button {
                        class: "btn danger",
                        disabled: busy(),
                        onclick: move |_| decide(deny_job.clone(), JobAnswer {
                            request_id: deny_id.clone(),
                            behavior: Some(AnswerBehavior::Deny),
                            scope: None,
                            answers: None,
                        }, busy),
                        "Rechazar"
                    }
                }
            }
        }
        (Decision::Open, None) => {
            rsx! { p { class: "muted", "El agente ya no espera una respuesta." } }
        }
        (Decision::Answered(label), _) => rsx! { p { class: "decided", "{label}" } },
        (Decision::Expired, _) => {
            rsx! { p { class: "muted", "Caducó antes de que respondieras." } }
        }
    };
    let title = if card.tool.is_empty() {
        "Permiso".to_owned()
    } else {
        format!("Permiso · {}", card.tool)
    };
    let open = card.decision == Decision::Open && card.request_id.is_some();
    rsx! {
        section { class: if open { "card ask open" } else { "card ask" },
            h4 { "{title}" }
            p { class: "ask-summary", "{model::clip(&card.summary, 400)}" }
            if !card.detail.is_empty() && card.detail != card.summary {
                details { class: "fold", open: open,
                    summary { "Detalle" }
                    pre { "{model::clip(&card.detail, 1500)}" }
                }
            }
            {controls}
        }
    }
}

#[component]
fn QuestionView(job: JobId, card: QuestionCard) -> Element {
    let busy = use_signal(|| false);
    // For each question, the labels picked so far.
    let mut picked = use_signal(BTreeMap::<usize, Vec<String>>::new);
    let open = card.decision == Decision::Open;
    let ready = card.asks.iter().enumerate().all(|(index, _)| {
        picked
            .read()
            .get(&index)
            .is_some_and(|labels| !labels.is_empty())
    });
    let submit = {
        let job = job.clone();
        let card = card.clone();
        move |_| {
            let mut answers = serde_json::Map::new();
            for (index, ask) in card.asks.iter().enumerate() {
                let labels = picked.read().get(&index).cloned().unwrap_or_default();
                answers.insert(
                    ask.question.clone(),
                    serde_json::Value::String(labels.join(", ")),
                );
            }
            decide(
                job.clone(),
                JobAnswer {
                    request_id: card.request_id.clone(),
                    behavior: None,
                    scope: None,
                    answers: Some(answers),
                },
                busy,
            );
        }
    };
    rsx! {
        section { class: if open { "card ask open" } else { "card ask" },
            h4 { "El agente pregunta" }
            for (index, ask) in card.asks.iter().cloned().enumerate() {
                fieldset { key: "{index}", disabled: !open || busy(),
                    legend { if ask.header.is_empty() { "{ask.question}" } else { "{ask.header}" } }
                    if !ask.header.is_empty() { p { "{ask.question}" } }
                    div { class: "options",
                        for (slot, (label, description)) in ask.options.iter().cloned().enumerate() {
                            {
                                let chosen = picked.read().get(&index).is_some_and(|labels| labels.contains(&label));
                                let value = label.clone();
                                let multi = ask.multi;
                                rsx! {
                                    button {
                                        key: "{slot}",
                                        r#type: "button",
                                        class: if chosen { "option chosen" } else { "option" },
                                        aria_pressed: "{chosen}",
                                        onclick: move |_| {
                                            let mut all = picked.write();
                                            let labels = all.entry(index).or_default();
                                            if let Some(at) = labels.iter().position(|l| *l == value) {
                                                labels.remove(at);
                                            } else {
                                                if !multi { labels.clear(); }
                                                labels.push(value.clone());
                                            }
                                        },
                                        strong { "{label}" }
                                        if !description.is_empty() { small { "{description}" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            match &card.decision {
                Decision::Open => rsx! {
                    div { class: "actions",
                        button { class: "btn primary", disabled: !ready || busy(), onclick: submit, "Responder" }
                    }
                },
                Decision::Answered(label) => rsx! { p { class: "decided", "{label}" } },
                Decision::Expired => rsx! { p { class: "muted", "Caducó antes de que respondieras." } },
            }
        }
    }
}
