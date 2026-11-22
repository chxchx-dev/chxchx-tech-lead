//! The pages other than the chat: home, projects, jobs, devices and settings.

use super::cards::ItemView;
use crate::actions;
use crate::model::{self, Item};
use crate::net::{self, Permission};
use crate::route::Route;
use crate::store::{self, CLOCK, DEVICES, JOBS, NODE, PREFS, PROJECTS, SESSION, toast};
use chxchx_protocol::{AccessMode, Job, JobId, JobSpec, Provider, Role};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let node = NODE.read().clone();
    let waiting = store::waiting_for_me();
    let recent: Vec<Job> = JOBS
        .read()
        .values()
        .rev()
        .take(5)
        .map(|view| view.job.clone())
        .collect();
    // A line in the voice of the terminal prompt: what the PC is doing right now.
    let voice = node.as_ref().map(|node| {
        if !waiting.is_empty() {
            "CHXCHX-DEV › esperan tu respuesta".to_owned()
        } else if node.jobs.running > 0 {
            format!("CHXCHX-DEV › {} trabajando", node.jobs.running)
        } else {
            "CHXCHX-DEV › listo para trabajar".to_owned()
        }
    });
    rsx! {
        section { class: "page",
            div { class: "card hero",
                div { class: "row",
                    div { class: "hero-id",
                        img { class: "avatar", src: "/img/mascot-96.png", alt: "", width: "52", height: "52" }
                        h3 { "Tu PC" }
                    }
                    if node.is_some() { span { class: "badge ok", "Conectada" } }
                }
                if let Some(line) = &voice { p { class: "mono prompt", "{line}" } }
                match node {
                    Some(node) => rsx! {
                        div { class: "stats",
                            div { class: "stat", b { "{node.jobs.running}" } span { "En curso" } }
                            div { class: "stat", b { "{node.jobs.queued}" } span { "En cola" } }
                            div { class: "stat", b { "{node.version}" } span { "Versión" } }
                        }
                        p { class: "muted small", "Tu rol en este dispositivo: " b { "{model::role_label(node.you.role)}" } }
                    },
                    None => rsx! { p { class: "muted", "Conectando con el nodo…" } },
                }
            }
            if !waiting.is_empty() {
                div { class: "card attention",
                    h3 { "Esperan tu respuesta" }
                    ul { class: "list",
                        for (id, what) in waiting {
                            li { key: "{id}",
                                a { href: "{Route::Job(id.clone()).to_hash()}", "{what}" }
                            }
                        }
                    }
                }
            }
            div { class: "card",
                h3 { "Recientes" }
                if recent.is_empty() {
                    p { class: "muted", "Todavía no hay trabajos. Abre un proyecto y escríbele a un agente." }
                } else {
                    ul { class: "list",
                        for job in recent { JobRow { key: "{job.id}", job } }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Projects() -> Element {
    use_hook(|| {
        if PROJECTS.read().is_none() {
            store::detach(async { actions::load_projects().await });
        }
    });
    let projects = PROJECTS.read().clone();
    rsx! {
        section { class: "page",
            match projects {
                None => rsx! { p { class: "muted", "Cargando proyectos…" } },
                Some(list) if list.is_empty() => rsx! {
                    div { class: "card",
                        h3 { "No hay proyectos de confianza" }
                        p { class: "muted", "En la PC, concede confianza a un proyecto con " code { "chxchx-tech workspace trust" } ". Un dispositivo solo ve los de confianza." }
                    }
                },
                Some(list) => rsx! {
                    ul { class: "list cards",
                        for project in list {
                            {
                                let href = Route::Chat { project: project.id.clone(), provider: Provider::Claude }.to_hash();
                                rsx! {
                                    li { key: "{project.id}",
                                        a { class: "card link", href: "{href}",
                                            div { class: "link-body",
                                                h3 { "{project.name}" }
                                                p { class: "muted small", "Perfil: {project.profile}" }
                                                if !project.stacks.is_empty() {
                                                    div { class: "chips",
                                                        for stack in project.stacks.iter() { span { key: "{stack}", class: "chip", "{stack}" } }
                                                    }
                                                }
                                            }
                                            span { class: "chevron", aria_hidden: "true", "›" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn JobRow(job: Job) -> Element {
    let now = CLOCK();
    // Several turns of the same agent look alike: what was asked tells them apart.
    let excerpt = match &job.spec {
        JobSpec::AgentTurn(turn) => {
            Some(turn.prompt.split_whitespace().collect::<Vec<_>>().join(" "))
        }
        _ => None,
    }
    .filter(|text| !text.is_empty());
    rsx! {
        li { class: "job-row",
            a { href: "{Route::Job(job.id.clone()).to_hash()}",
                span { class: "job-main",
                    span { class: "job-what", "{model::describe(&job.spec)}" }
                    if let Some(prompt) = &excerpt { span { class: "job-prompt", "{prompt}" } }
                }
                span { class: "job-side",
                    span { class: "badge {model::state_class(job.state)}", "{model::state_label(job.state)}" }
                    span { class: "muted when", "{model::ago(now, job.updated_ms)}" }
                }
            }
        }
    }
}

#[component]
pub fn Jobs() -> Element {
    let jobs: Vec<Job> = JOBS
        .read()
        .values()
        .rev()
        .map(|view| view.job.clone())
        .collect();
    let (active, done): (Vec<Job>, Vec<Job>) =
        jobs.into_iter().partition(|job| !job.state.is_terminal());
    rsx! {
        section { class: "page",
            if active.is_empty() && done.is_empty() {
                div { class: "card", p { class: "muted", "No hay trabajos todavía." } }
            }
            if !active.is_empty() {
                div { class: "card",
                    h3 { "En curso" }
                    ul { class: "list", for job in active { JobRow { key: "{job.id}", job } } }
                }
            }
            if !done.is_empty() {
                div { class: "card",
                    h3 { "Terminados" }
                    ul { class: "list", for job in done { JobRow { key: "{job.id}", job } } }
                }
            }
        }
    }
}

#[component]
pub fn JobDetail(id: JobId) -> Element {
    {
        let id = id.clone();
        use_hook(move || {
            store::detach(async move {
                let _ = actions::events_of(&id).await;
            });
        });
    }
    let view = JOBS.read().get(&id).cloned();
    let now = CLOCK();
    let Some(view) = view else {
        return rsx! {
            section { class: "page",
                a { class: "back-link", href: "#/jobs", "‹ Trabajos" }
                div { class: "card", p { class: "muted", "Buscando el trabajo…" } }
            }
        };
    };
    let items: Vec<Item> = model::fold(&view.events, view.terminal());
    let prompt = match &view.job.spec {
        JobSpec::AgentTurn(turn) => Some(turn.prompt.clone()),
        _ => None,
    };
    let cancel = {
        let id = id.clone();
        move |_| {
            let id = id.clone();
            spawn(async move {
                if let Err(message) = actions::cancel(id).await {
                    toast(message, true);
                }
            });
        }
    };
    rsx! {
        section { class: "page",
            a { class: "back-link", href: "#/jobs", "‹ Trabajos" }
            div { class: "card",
                div { class: "row",
                    h3 { "{model::describe(&view.job.spec)}" }
                    span { class: "badge {model::state_class(view.job.state)}", "{model::state_label(view.job.state)}" }
                }
                p { class: "muted", "{model::ago(now, view.job.updated_ms)}" }
                if let JobSpec::AgentTurn(turn) = &view.job.spec {
                    p { class: "muted", "Acceso: {turn.access.as_str()}" if turn.network { " · con red" } }
                }
                if let Some(error) = &view.job.error { div { class: "note bad", role: "alert", "{error.message}" } }
                if let Some(summary) = &view.job.summary { p { class: "summary", "{summary}" } }
                if !view.terminal() && store::can_operate() {
                    div { class: "actions", button { class: "btn danger", onclick: cancel, "Cancelar el trabajo" } }
                }
            }
            div { class: "thread",
                if let Some(prompt) = prompt { div { class: "msg user", "{prompt}" } }
                for (index, item) in items.into_iter().enumerate() {
                    ItemView { key: "{index}", job: id.clone(), item }
                }
                if view.events.is_empty() && !view.terminal() { p { class: "muted", "Esperando eventos…" } }
            }
        }
    }
}

#[component]
pub fn Devices() -> Element {
    let admin = store::role() == Role::Admin;
    use_hook(move || {
        if admin {
            store::detach(async { actions::load_devices().await });
        }
    });
    let mine = SESSION
        .read()
        .as_ref()
        .map(|session| session.device.id.clone());
    let devices = DEVICES.read().clone();
    let mut confirming = use_signal(|| None::<chxchx_protocol::DeviceId>);
    let now = CLOCK();
    if !admin {
        return rsx! {
            section { class: "page",
                div { class: "card",
                    h3 { "Dispositivos" }
                    p { class: "muted", "Solo un administrador puede ver y desvincular dispositivos. Desde la PC: " code { "chxchx-tech node devices" } "." }
                }
            }
        };
    }
    rsx! {
        section { class: "page",
            match devices {
                None => rsx! { p { class: "muted", "Cargando…" } },
                Some(list) => rsx! {
                    ul { class: "list cards",
                        for device in list {
                            li { key: "{device.id}", class: "card",
                                div { class: "row",
                                    h3 { "{device.name}" }
                                    if Some(&device.id) == mine.as_ref() { span { class: "badge ok", "Este dispositivo" } }
                                }
                                p { class: "muted",
                                    "{model::role_label(device.role)}"
                                    if let Some(seen) = device.last_seen_ms { " · visto {model::ago(now, seen)}" }
                                }
                                div { class: "actions",
                                    if confirming() == Some(device.id.clone()) {
                                        button { class: "btn danger",
                                            onclick: {
                                                let id = device.id.clone();
                                                move |_| {
                                                    let id = id.clone();
                                                    confirming.set(None);
                                                    spawn(async move {
                                                        match actions::revoke(id).await {
                                                            Ok(()) => toast("Dispositivo desvinculado", false),
                                                            Err(message) => toast(message, true),
                                                        }
                                                    });
                                                }
                                            },
                                            "Sí, desvincular"
                                        }
                                        button { class: "btn", onclick: move |_| confirming.set(None), "No" }
                                    } else {
                                        button { class: "btn", onclick: { let id = device.id.clone(); move |_| confirming.set(Some(id.clone())) }, "Desvincular" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
            div { class: "card",
                h3 { "Vincular otro dispositivo" }
                p { class: "muted", "Se hace en la PC, por seguridad: " code { "chxchx-tech node pair" } " crea un código de un solo uso." }
            }
        }
    }
}

#[component]
pub fn Settings() -> Element {
    let session = SESSION.read().clone();
    let node = NODE.read().clone();
    let mut permission = use_signal(net::notification_permission);
    let access = PREFS.read().access;
    rsx! {
        section { class: "page",
            if let Some(session) = session {
                div { class: "card",
                    h3 { "Este dispositivo" }
                    dl { class: "facts",
                        dt { "Nombre" } dd { "{session.device.name}" }
                        dt { "Rol" } dd { "{model::role_label(session.device.role)}" }
                    }
                }
            }
            div { class: "card",
                h3 { "Acceso predeterminado" }
                label { class: "field", "Al escribir a un agente"
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
            }
            div { class: "card",
                h3 { "Avisos" }
                match permission() {
                    Permission::Granted => rsx! { p { class: "muted", "Activados. Te avisamos cuando un agente pide permiso o termina, mientras la app esté abierta o en segundo plano." } },
                    Permission::Denied => rsx! { p { class: "muted", "Los bloqueaste en el navegador. Cámbialo en los ajustes del sitio." } },
                    Permission::Unsupported => rsx! { p { class: "muted", "Este navegador no permite avisos. En iPhone hay que añadir la app a la pantalla de inicio." } },
                    Permission::Default => rsx! {
                        p { class: "muted", "Para saber cuándo un agente pide permiso o termina." }
                        div { class: "actions",
                            button { class: "btn primary", onclick: move |_| { spawn(async move { permission.set(net::request_notifications().await); }); }, "Activar avisos" }
                        }
                    },
                }
                p { class: "muted small", "Solo llegan con la app abierta o en segundo plano: avisos con la app cerrada necesitan notificaciones push, que no incluye esta versión." }
            }
            div { class: "card",
                h3 { "Instalar en el teléfono" }
                p { class: "muted", "Android (Chrome): menú ⋮ → «Instalar app». iPhone (Safari): botón de compartir → «Añadir a pantalla de inicio»." }
            }
            if let Some(node) = node {
                div { class: "card",
                    h3 { "Nodo" }
                    dl { class: "facts",
                        dt { "Versión" } dd { "{node.version}" }
                        dt { "Protocolo" } dd { "{node.protocol}" }
                    }
                }
            }
            div { class: "card",
                div { class: "actions",
                    button { class: "btn danger", onclick: move |_| { actions::sign_out(); }, "Cerrar sesión en este dispositivo" }
                }
                p { class: "muted small", "Solo olvida el token aquí. Para desvincularlo del todo, hazlo desde un administrador o desde la PC." }
            }
        }
    }
}
