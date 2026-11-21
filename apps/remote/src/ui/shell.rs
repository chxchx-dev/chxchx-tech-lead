//! The frame: pairing when there is no session, otherwise the header, the page and the tab bar.

use super::chat::Chat;
use super::pages::{Devices, Home, JobDetail, Jobs, Projects, Settings};
use crate::actions;
use crate::engine;
use crate::net;
use crate::route::{Route, Tab, code_in_hash};
use crate::store::{self, CONN, Conn, ROUTE, SESSION, TOAST};
use chxchx_protocol::Role;
use dioxus::prelude::*;

pub fn app() -> Element {
    // The engine runs for the life of the page.
    use_hook(|| {
        store::detach(engine::run());
    });
    rsx! {
        if SESSION.read().is_some() { Shell {} } else { Pairing {} }
        Toasts {}
    }
}

#[component]
fn Shell() -> Element {
    let route = ROUTE.read().clone();
    let page = match &route {
        Route::Home => rsx! { Home {} },
        Route::Projects => rsx! { Projects {} },
        Route::Chat { project, provider } => rsx! {
            Chat { key: "{project}-{provider.as_str()}", project: project.clone(), provider: *provider }
        },
        Route::Jobs => rsx! { Jobs {} },
        Route::Job(id) => rsx! { JobDetail { key: "{id}", id: id.clone() } },
        Route::Devices => rsx! { Devices {} },
        Route::Settings => rsx! { Settings {} },
    };
    let title = match &route {
        Route::Home => "ChxChx".to_owned(),
        Route::Projects => "Proyectos".to_owned(),
        Route::Chat { .. } => "Conversación".to_owned(),
        Route::Jobs | Route::Job(_) => "Trabajos".to_owned(),
        Route::Devices => "Dispositivos".to_owned(),
        Route::Settings => "Ajustes".to_owned(),
    };
    rsx! {
        div { class: "app",
            header { class: "top",
                h1 { "{title}" }
                ConnDot {}
            }
            ConnBanner {}
            main { class: "content", id: "content", {page} }
            TabBar { active: route.tab() }
        }
    }
}

#[component]
fn ConnDot() -> Element {
    let (class, text) = match CONN() {
        Conn::Online => ("dot online", "Conectado".to_owned()),
        Conn::Connecting => ("dot connecting", "Conectando".to_owned()),
        Conn::Offline(reason) => ("dot offline", format!("Sin conexión: {reason}")),
    };
    rsx! { span { class: "{class}", role: "img", aria_label: "{text}", title: "{text}" } }
}

#[component]
fn ConnBanner() -> Element {
    match CONN() {
        Conn::Offline(reason) => rsx! {
            div { class: "banner", role: "status",
                "Sin conexión con tu PC ({reason}). Reintentando; lo que pase mientras tanto se recupera al volver."
            }
        },
        _ => rsx! {},
    }
}

#[component]
fn TabBar(active: Tab) -> Element {
    let admin = store::role() == Role::Admin;
    let waiting = store::waiting_for_me().len();
    let tab = |tab: Tab, href: &'static str, label: &'static str, glyph: &'static str| {
        rsx! {
            a {
                key: "{label}",
                class: if active == tab { "tab active" } else { "tab" },
                href: "{href}",
                aria_current: if active == tab { "page" } else { "false" },
                span { class: "glyph", aria_hidden: "true", "{glyph}" }
                span { "{label}" }
                if tab == Tab::Home && waiting > 0 { span { class: "pip", aria_label: "{waiting} esperando tu respuesta", "{waiting}" } }
            }
        }
    };
    rsx! {
        nav { class: "tabs", aria_label: "Secciones",
            {tab(Tab::Home, "#/", "Inicio", "⌂")}
            {tab(Tab::Projects, "#/projects", "Proyectos", "▤")}
            {tab(Tab::Jobs, "#/jobs", "Trabajos", "≡")}
            if admin { {tab(Tab::Devices, "#/devices", "Dispositivos", "◉")} }
            {tab(Tab::Settings, "#/settings", "Ajustes", "⚙")}
        }
    }
}

#[component]
fn Pairing() -> Element {
    let mut code = use_signal(|| code_in_hash(&net::hash()).unwrap_or_default());
    let mut name = use_signal(default_name);
    let mut busy = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    let from_qr = code_in_hash(&net::hash()).is_some();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        let (code_text, name_text) = (code.read().clone(), name.read().clone());
        busy.set(true);
        failure.set(None);
        spawn(async move {
            // On success this screen is replaced by the app: it has nothing left to update.
            if let Err(message) = actions::pair(code_text, name_text).await {
                failure.set(Some(message));
                busy.set(false);
            }
        });
    };
    rsx! {
        main { class: "pairing",
            h1 { "ChxChx" }
            p { class: "lead", "Vincula este dispositivo con tu PC para ver tus proyectos y hablar con tus agentes." }
            form { onsubmit: submit,
                label { class: "field", "Código de vinculación"
                    input {
                        r#type: "text", value: "{code}", placeholder: "XXXXX-XXXXX",
                        autocomplete: "off", autocapitalize: "characters", spellcheck: "false",
                        maxlength: "11", inputmode: "text", required: true,
                        oninput: move |event| code.set(event.value().to_ascii_uppercase()),
                    }
                }
                label { class: "field", "Nombre de este dispositivo"
                    input {
                        r#type: "text", value: "{name}", maxlength: "64", required: true,
                        oninput: move |event| name.set(event.value()),
                    }
                }
                if let Some(message) = failure() { div { class: "note bad", role: "alert", "{message}" } }
                button { class: "btn primary wide", r#type: "submit", disabled: busy() || code.read().trim().is_empty(), if busy() { "Vinculando…" } else { "Vincular" } }
            }
            if from_qr { p { class: "muted", "El código vino del QR. Toca «Vincular» para confirmarlo." } }
            div { class: "card",
                h3 { "¿Dónde saco el código?" }
                p { class: "muted", "En tu PC ejecuta " code { "chxchx-tech node pair" } ". Vale 5 minutos y un solo uso." }
            }
        }
    }
}

/// A name the person will recognise in the list of devices.
fn default_name() -> String {
    let agent = web_sys::window()
        .and_then(|window| window.navigator().user_agent().ok())
        .unwrap_or_default();
    if agent.contains("iPhone") {
        "iPhone".to_owned()
    } else if agent.contains("iPad") {
        "iPad".to_owned()
    } else if agent.contains("Android") {
        "Teléfono Android".to_owned()
    } else {
        "Navegador".to_owned()
    }
}

#[component]
fn Toasts() -> Element {
    let toast = TOAST.read().clone();
    use_effect(move || {
        if let Some(current) = TOAST.read().clone() {
            spawn(async move {
                net::sleep(4500).await;
                // Only the toast that was shown: a newer one keeps its own time.
                if TOAST.read().as_ref() == Some(&current) {
                    *TOAST.write() = None;
                }
            });
        }
    });
    rsx! {
        div { class: "toasts", aria_live: "polite",
            if let Some(toast) = toast {
                div { class: if toast.bad { "toast bad" } else { "toast" }, role: "status", "{toast.text}" }
            }
        }
    }
}
