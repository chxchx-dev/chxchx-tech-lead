//! The conversation with an agent in one project.

use super::cards::ItemView;
use crate::actions::{self, Draft};
use crate::model::{self, Item};
use crate::net;
use crate::route::Route;
use crate::store::{self, PREFS, PROJECT_DATA, ProjectData, toast};
use chxchx_protocol::{AccessMode, AgentSummary, ChatMessage, JobId, JobSpec, ProjectId, Provider};
use dioxus::prelude::*;

const PROVIDERS: [Provider; 3] = [Provider::Claude, Provider::Codex, Provider::OpenCode];

#[component]
pub fn Chat(project: ProjectId, provider: Provider) -> Element {
    let key = project.to_string();
    {
        let (project, key) = (project.clone(), key.clone());
        use_hook(move || {
            // Opened once; the refresh button loads it again.
            if !PROJECT_DATA.read().contains_key(&key) {
                store::detach(async move { actions::open_chat(&project).await });
            }
        });
    }
    let data: ProjectData = PROJECT_DATA.read().get(&key).cloned().unwrap_or_default();
    let agents: Vec<AgentSummary> = data.agents.clone().unwrap_or_default();
    let agent = agents
        .iter()
        .find(|agent| agent.provider == provider)
        .cloned();
    let loading = data.agents.is_none() && data.error.is_none();

    let live = store::live_jobs(&key, provider, data.opened_ms);
    let running_prompts: Vec<String> = live
        .iter()
        .filter(|view| view.job.created_ms < data.opened_ms)
        .filter_map(|view| match &view.job.spec {
            JobSpec::AgentTurn(turn) => Some(turn.prompt.clone()),
            _ => None,
        })
        .collect();
    let history: Vec<ChatMessage> = model::without_running(
        data.chats
            .as_deref()
            .and_then(|chats| store::snapshot_of(chats, provider))
            .map(|chat| chat.messages)
            .unwrap_or_default(),
        &running_prompts,
    );
    let working: Option<JobId> = live
        .iter()
        .find(|view| !view.terminal())
        .map(|view| view.job.id.clone());
    let total = history.len() + live.iter().map(|view| view.events.len() + 1).sum::<usize>();

    // Keep the latest message in view as the conversation grows.
    // `use_effect` only reruns for signals it reads; `use_reactive` makes `total` one of them.
    use_effect(use_reactive!(|(total,)| {
        let _ = total;
        spawn(net::follow_end());
    }));

    let mut draft = use_signal(String::new);
    let mut sending = use_signal(|| false);
    let mut model_pick = use_signal(String::new);
    let mut effort_pick = use_signal(String::new);

    let send = {
        let (project, key) = (project.clone(), key.clone());
        let agent = agent.clone();
        move || {
            let prompt = draft.read().trim().to_owned();
            if prompt.is_empty() || sending() {
                return;
            }
            let prefs = PREFS.read().clone();
            let model_id = model_pick.read().clone();
            let effort = effort_pick.read().clone();
            let default_model = agent
                .as_ref()
                .map(|a| a.default_model.clone())
                .unwrap_or_default();
            let request = Draft {
                project: project.clone(),
                provider,
                prompt,
                access: prefs.access,
                network: prefs.network && store::role() == chxchx_protocol::Role::Admin,
                model: (!model_id.is_empty() && model_id != default_model).then_some(model_id),
                effort: (!effort.is_empty()).then_some(effort),
                session: store::continuation(&key, provider),
            };
            sending.set(true);
            spawn(async move {
                let sent = actions::send_turn(request).await;
                if let Err(message) = &sent {
                    toast(message.clone(), true);
                }
                // The screen may have been left meanwhile: its signals are gone then.
                if sent.is_ok()
                    && let Ok(mut text) = draft.try_write()
                {
                    text.clear();
                }
                if let Ok(mut flag) = sending.try_write() {
                    *flag = false;
                }
            });
        }
    };
    let on_key = {
        let mut send = send.clone();
        move |event: KeyboardEvent| {
            let modifiers = event.modifiers();
            if event.key() == Key::Enter && (modifiers.ctrl() || modifiers.meta()) {
                event.prevent_default();
                send();
            }
        }
    };
    let stop = {
        let working = working.clone();
        move |_| {
            if let Some(id) = working.clone() {
                spawn(async move {
                    if let Err(message) = actions::cancel(id).await {
                        toast(message, true);
                    }
                });
            }
        }
    };

    let access = PREFS.read().access;
    let models = agent.as_ref().map(|a| a.models.clone()).unwrap_or_default();
    let chosen_model = model_pick.read().clone();
    let efforts: Vec<String> = models
        .iter()
        .find(|m| {
            m.id == chosen_model
                || (chosen_model.is_empty()
                    && agent.as_ref().is_some_and(|a| a.default_model == m.id))
        })
        .map(|m| m.efforts.clone())
        .unwrap_or_default();
    let installed = agent.as_ref().is_some_and(|a| a.installed);

    rsx! {
        section { class: "chat",
            div { class: "chat-head",
                a { class: "back", href: "#/projects", aria_label: "Volver a proyectos", "‹" }
                h2 { "{project}" }
                button {
                    class: "icon", aria_label: "Recargar la conversación", title: "Recargar",
                    onclick: {
                        let project = project.clone();
                        move |_| {
                            let project = project.clone();
                            store::detach(async move { actions::open_chat(&project).await });
                        }
                    },
                    "↻"
                }
            }
            nav { class: "segments", aria_label: "Agente",
                for candidate in PROVIDERS {
                    {
                        let is_installed = agents.iter().any(|a| a.provider == candidate && a.installed);
                        let target = Route::Chat { project: project.clone(), provider: candidate };
                        rsx! {
                            a {
                                key: "{candidate.as_str()}",
                                class: if candidate == provider { "segment active" } else if loading || is_installed { "segment" } else { "segment off" },
                                href: "{target.to_hash()}",
                                aria_current: if candidate == provider { "page" } else { "false" },
                                "{model::provider_label(candidate)}"
                            }
                        }
                    }
                }
            }
            if let Some(error) = &data.error { div { class: "note bad", role: "alert", "{error}" } }
            if !loading && !installed && data.error.is_none() {
                div { class: "card",
                    h3 { "{model::provider_label(provider)} no está instalado en la PC" }
                    if let Some(agent) = &agent { p { class: "muted", "{agent.install_hint}" } }
                }
            }
            div { class: "thread", aria_live: "polite",
                if loading { p { class: "muted", "Cargando la conversación…" } }
                for (index, message) in history.iter().enumerate() {
                    div { key: "h{index}", class: if message.role == "user" { "msg user" } else { "msg assistant" }, "{message.text}" }
                }
                for view in live.iter().cloned() {
                    Turn { key: "{view.job.id}", view }
                }
                div { id: "chat-end" }
            }
            if store::can_operate() {
                if installed {
                    details { class: "options-panel",
                        summary { "Opciones" }
                        label { "Acceso"
                            select {
                                value: if access == AccessMode::ReadOnly { "read-only" } else { "workspace-write" },
                                onchange: move |event| {
                                    let mut prefs = PREFS.write();
                                    prefs.access = AccessMode::parse(&event.value()).unwrap_or(AccessMode::ReadOnly);
                                    store::save_prefs(&prefs);
                                },
                                option { value: "read-only", "Solo leer" }
                                option { value: "workspace-write", "Leer y editar" }
                            }
                        }
                        p { class: "muted", "Desde un dispositivo vinculado el máximo es «leer y editar»; los comandos se aprueban uno a uno." }
                        if store::role() == chxchx_protocol::Role::Admin {
                            label { class: "check",
                                input {
                                    r#type: "checkbox",
                                    checked: PREFS.read().network,
                                    onchange: move |event| {
                                        let mut prefs = PREFS.write();
                                        prefs.network = event.checked();
                                        store::save_prefs(&prefs);
                                    },
                                }
                                " Permitir red"
                            }
                        }
                        if !models.is_empty() {
                            label { "Modelo"
                                select {
                                    value: "{model_pick}",
                                    onchange: move |event| { model_pick.set(event.value()); effort_pick.set(String::new()); },
                                    option { value: "", "Predeterminado" }
                                    for model in models.iter() { option { key: "{model.id}", value: "{model.id}", "{model.label}" } }
                                }
                            }
                        }
                        if !efforts.is_empty() {
                            label { "Esfuerzo"
                                select {
                                    value: "{effort_pick}",
                                    onchange: move |event| effort_pick.set(event.value()),
                                    option { value: "", "Predeterminado" }
                                    for effort in efforts.iter() { option { key: "{effort}", value: "{effort}", "{effort}" } }
                                }
                            }
                        }
                    }
                    form { class: "composer",
                        onsubmit: {
                            let mut send = send.clone();
                            move |event: FormEvent| { event.prevent_default(); send(); }
                        },
                        textarea {
                            rows: "2", placeholder: "Escribe un mensaje…", aria_label: "Mensaje",
                            value: "{draft}", disabled: working.is_some(),
                            oninput: move |event| draft.set(event.value()),
                            onkeydown: on_key,
                        }
                        if working.is_some() {
                            button { class: "btn danger", r#type: "button", onclick: stop, "Detener" }
                        } else {
                            button { class: "btn primary", r#type: "submit", disabled: draft.read().trim().is_empty() || sending(), "Enviar" }
                        }
                    }
                }
            } else {
                p { class: "muted pad", "Con el rol de lector solo puedes mirar la conversación." }
            }
        }
    }
}

/// One turn of the conversation, as a live job tells it.
#[component]
fn Turn(view: store::JobView) -> Element {
    let prompt = match &view.job.spec {
        JobSpec::AgentTurn(turn) => turn.prompt.clone(),
        _ => String::new(),
    };
    let items: Vec<Item> = model::fold(&view.events, view.terminal());
    let thinking = !view.terminal() && items.is_empty();
    rsx! {
        div { class: "turn",
            div { class: "msg user", "{prompt}" }
            for (index, item) in items.into_iter().enumerate() {
                ItemView { key: "{index}", job: view.job.id.clone(), item }
            }
            if thinking { div { class: "note", "El agente está trabajando…" } }
        }
    }
}
