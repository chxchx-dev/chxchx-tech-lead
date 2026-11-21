//! Settings → Nodo local (ADR 0017, H5): is it running, should chat turns go to it, start it, and
//! the devices linked to it (see them, link one with a code, unlink one).
//!
//! Everything that talks to the node runs in the background; the screen only shows the last thing
//! read. Nothing here is needed to use Studio: with the node off, Studio runs everything itself.
use super::*;
use crate::integrations::node_link::{self, Endpoint};
use crate::integrations::node_process;
use chxchx_protocol::{Device, DeviceId, PairingOffer, Role};
use iced::widget::column;

/// What was found the last time the node was asked.
#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) struct Snapshot {
    /// `None` when no node answers.
    pub running: Option<Running>,
    /// The address devices open, when remote access is enabled.
    pub address: Option<String>,
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) struct Running {
    pub port: u16,
    pub version: String,
    pub running: usize,
    pub queued: usize,
    /// `None` when the devices could not be read.
    pub devices: Option<Vec<Device>>,
}

#[derive(Default)]
pub(super) struct State {
    snapshot: Option<Snapshot>,
    /// A pairing code just made: (code, address to open, expires at).
    code: Option<(String, Option<String>, u64)>,
    /// The device whose unlinking is waiting for a second tap.
    confirming: Option<DeviceId>,
    busy: bool,
    notice: String,
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Event {
    Refresh,
    Loaded(Snapshot),
    /// Hand chat turns to the node (or not).
    Delegate(bool),
    Start,
    Started(Result<(), String>),
    Stop,
    Stopped(Result<(), String>),
    NewCode,
    Code(Result<PairingOffer, String>),
    CopyCode,
    AskUnlink(DeviceId),
    CancelUnlink,
    Unlink(DeviceId),
    Unlinked(Result<(), String>),
}

/// Reads the node. Blocking: off the UI thread.
fn read() -> Snapshot {
    let address = crate::features::node_state::dir()
        .and_then(|dir| crate::features::node_state::read_remote(&dir))
        .map(|remote| remote.url());
    let running = node_link::endpoint().and_then(|endpoint| {
        let status = node_link::status(&endpoint).ok()?;
        Some(Running {
            port: endpoint.port,
            version: status.version,
            running: status.jobs.running,
            queued: status.jobs.queued,
            devices: node_link::devices(&endpoint).ok(),
        })
    });
    Snapshot { running, address }
}

/// A node to talk to for an action, or why there is none.
fn endpoint() -> Result<Endpoint, String> {
    node_link::endpoint().ok_or_else(|| "El nodo no responde.".to_owned())
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Refresh => {
            if state.settings.node.busy {
                return Task::none();
            }
            return jobs::quiet(read, |state, snapshot| {
                settings::update(state, settings::Event::Node(Event::Loaded(snapshot)))
            });
        }
        Event::Loaded(snapshot) => {
            // A pairing code or an unlink that was waiting is meaningless once the node is gone.
            if snapshot.running.is_none() {
                state.settings.node.code = None;
                state.settings.node.confirming = None;
            }
            state.settings.node.snapshot = Some(snapshot);
        }
        Event::Delegate(on) => {
            state
                .session
                .edit(|preferences| preferences.node.delegate = on);
        }
        Event::Start => {
            state.settings.node.busy = true;
            state.settings.node.notice.clear();
            return jobs::quiet(node_process::start, |state, result| {
                settings::update(state, settings::Event::Node(Event::Started(result)))
            });
        }
        Event::Started(result) => {
            state.settings.node.busy = false;
            if let Err(error) = result {
                state.settings.node.notice = error;
            }
            return refresh(state);
        }
        Event::Stop => {
            state.settings.node.busy = true;
            return jobs::quiet(
                || endpoint().and_then(|endpoint| node_link::stop(&endpoint)),
                |state, result| {
                    settings::update(state, settings::Event::Node(Event::Stopped(result)))
                },
            );
        }
        Event::Stopped(result) => {
            state.settings.node.busy = false;
            state.settings.node.notice = match result {
                Ok(()) => "El nodo se está deteniendo: cancela lo que corría.".to_owned(),
                Err(error) => error,
            };
            // It takes a moment to finish; the periodic refresh shows when it is gone.
            return refresh(state);
        }
        Event::NewCode => {
            state.settings.node.busy = true;
            state.settings.node.notice.clear();
            return jobs::quiet(
                || endpoint().and_then(|endpoint| node_link::pairing(&endpoint, Role::Operator)),
                |state, result| settings::update(state, settings::Event::Node(Event::Code(result))),
            );
        }
        Event::Code(result) => {
            state.settings.node.busy = false;
            match result {
                Ok(offer) => {
                    let address = state
                        .settings
                        .node
                        .snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.address.clone())
                        .map(|url| format!("{url}#code={}", offer.code));
                    state.settings.node.code = Some((offer.code, address, offer.expires_ms));
                }
                Err(error) => state.settings.node.notice = error,
            }
        }
        Event::CopyCode => {
            if let Some((code, address, _)) = &state.settings.node.code {
                return iced::clipboard::write(address.clone().unwrap_or_else(|| code.clone()));
            }
        }
        Event::AskUnlink(id) => state.settings.node.confirming = Some(id),
        Event::CancelUnlink => state.settings.node.confirming = None,
        Event::Unlink(id) => {
            state.settings.node.confirming = None;
            state.settings.node.busy = true;
            return jobs::quiet(
                move || {
                    endpoint().and_then(|endpoint| node_link::revoke(&endpoint, &id).map(|_| ()))
                },
                |state, result| {
                    settings::update(state, settings::Event::Node(Event::Unlinked(result)))
                },
            );
        }
        Event::Unlinked(result) => {
            state.settings.node.busy = false;
            state.settings.node.notice = match result {
                Ok(()) => "Dispositivo desvinculado: su token ya no vale.".to_owned(),
                Err(error) => error,
            };
            return refresh(state);
        }
    }
    Task::none()
}

fn refresh(state: &mut Dashboard) -> Task<Message> {
    update(state, Event::Refresh)
}

fn ago(then_ms: u64) -> String {
    let seconds = crate::features::jobs::store::now_ms().saturating_sub(then_ms) / 1000;
    match seconds {
        0..60 => "hace un momento".to_owned(),
        60..3600 => format!("hace {} min", seconds / 60),
        3600..86_400 => format!("hace {} h", seconds / 3600),
        _ => format!("hace {} d", seconds / 86_400),
    }
}

fn role_label(role: Role) -> &'static str {
    match role {
        Role::Reader => "Lector",
        Role::Operator => "Operador",
        Role::Admin => "Administrador",
    }
}

fn pill<'a>(label: &'a str, event: Event) -> iced::widget::Button<'a, Message> {
    button(text(label).size(12))
        .padding([6, 12])
        .style(styles::secondary)
        .on_press(Message::Settings(settings::Event::Node(event)))
}

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let node = &state.settings.node;
    let delegate = state.session.preferences().node.delegate;
    let mut body = column![
        row![
            text("Nodo local").size(13).font(design::UI_BOLD),
            Space::new().width(Fill),
            pill("Actualizar", Event::Refresh),
        ]
        .align_y(iced::Center),
        text(
            "El nodo corre tus agentes aunque cierres Studio y te deja seguirlos desde el teléfono. \
             Es opcional: sin él, Studio hace todo por sí mismo."
        )
        .size(12)
        .color(design::muted()),
    ]
    .spacing(10);

    let Some(snapshot) = &node.snapshot else {
        return finish(
            body.push(text("Buscando el nodo…").size(12).color(design::muted())),
            node,
        );
    };

    match &snapshot.running {
        Some(running) => {
            body = body.push(
                row![
                    icons::icon("dot", design::success(), 14.0),
                    text(format!(
                        "En marcha · 127.0.0.1:{} · versión {} · {} en curso, {} en cola",
                        running.port, running.version, running.running, running.queued
                    ))
                    .size(12),
                    Space::new().width(Fill),
                    pill("Detener", Event::Stop),
                ]
                .spacing(6)
                .align_y(iced::Center),
            );
        }
        None => {
            body = body.push(
                row![
                    icons::icon("dot", design::faint(), 14.0),
                    text("Detenido").size(12),
                    Space::new().width(Fill),
                    pill("Iniciar el nodo", Event::Start),
                ]
                .spacing(6)
                .align_y(iced::Center),
            );
            body = body.push(
                text("Para que arranque al iniciar sesión: `chxchx-tech node service install`.")
                    .size(11)
                    .color(design::faint()),
            );
        }
    }

    body = body.push(
        row![
            button(text(if delegate { "Sí" } else { "No" }).size(12))
                .padding([6, 12])
                .style(styles::ghost(delegate))
                .on_press(Message::Settings(settings::Event::Node(Event::Delegate(
                    !delegate
                )))),
            text("Usar el nodo para los turnos del chat").size(12),
        ]
        .spacing(10)
        .align_y(iced::Center),
    );
    if delegate && snapshot.running.is_none() {
        body = body.push(
            text("El nodo no está en marcha: por ahora los turnos corren en Studio.")
                .size(11)
                .color(design::warning()),
        );
    }

    if let Some(running) = &snapshot.running {
        body = body.push(devices_section(node, snapshot, running));
    }
    finish(body, node)
}

fn devices_section<'a>(
    node: &'a State,
    snapshot: &'a Snapshot,
    running: &'a Running,
) -> Element<'a, Message> {
    let mut section = column![
        row![
            text("Dispositivos vinculados")
                .size(13)
                .font(design::UI_BOLD),
            Space::new().width(Fill),
            pill("Vincular un dispositivo", Event::NewCode),
        ]
        .align_y(iced::Center),
    ]
    .spacing(8);
    if let Some((code, address, expires)) = &node.code {
        let minutes = expires.saturating_sub(crate::features::jobs::store::now_ms()) / 60_000;
        let mut shown = column![
            row![
                text(code.clone()).size(20).font(design::MONO),
                Space::new().width(Fill),
                pill("Copiar", Event::CopyCode),
            ]
            .align_y(iced::Center),
            text(format!("Un solo uso; caduca en unos {minutes} min."))
                .size(11)
                .color(design::muted()),
        ]
        .spacing(4);
        shown = shown.push(
            text(match address {
                Some(address) => format!("En el dispositivo, con Tailscale activo, abre {address}"),
                None => "El acceso desde fuera no está activado: `chxchx-tech node remote enable`."
                    .to_owned(),
            })
            .size(11)
            .color(design::muted()),
        );
        section = section.push(
            container(shown)
                .padding(10)
                .width(Fill)
                .style(styles::tinted(design::accent())),
        );
    }
    match &running.devices {
        None => {
            section = section.push(
                text("No se pudieron leer los dispositivos.")
                    .size(12)
                    .color(design::muted()),
            )
        }
        Some(list) if list.is_empty() => {
            section = section.push(
                text("Todavía no hay ninguno.")
                    .size(12)
                    .color(design::muted()),
            );
        }
        Some(list) => {
            for device in list {
                let seen = device
                    .last_seen_ms
                    .map_or("sin actividad desde que arrancó el nodo".to_owned(), ago);
                let confirming = node.confirming.as_ref() == Some(&device.id);
                let action: Element<'_, Message> = if confirming {
                    row![
                        pill("Sí, desvincular", Event::Unlink(device.id.clone())),
                        pill("No", Event::CancelUnlink),
                    ]
                    .spacing(6)
                    .into()
                } else {
                    pill("Desvincular", Event::AskUnlink(device.id.clone())).into()
                };
                section = section.push(
                    row![
                        column![
                            text(device.name.clone()).size(12).font(design::UI_BOLD),
                            text(format!("{} · {seen}", role_label(device.role)))
                                .size(11)
                                .color(design::muted()),
                        ]
                        .spacing(2),
                        Space::new().width(Fill),
                        action,
                    ]
                    .align_y(iced::Center),
                );
            }
        }
    }
    if snapshot.address.is_none() {
        section = section.push(
            text("Para verlo desde fuera de casa: `chxchx-tech node remote enable`.")
                .size(11)
                .color(design::faint()),
        );
    }
    section.into()
}

fn finish<'a>(body: iced::widget::Column<'a, Message>, node: &'a State) -> Element<'a, Message> {
    let mut body = body;
    if node.busy {
        body = body.push(text("Trabajando…").size(11).color(design::muted()));
    }
    if !node.notice.is_empty() {
        body = body.push(text(node.notice.clone()).size(12).color(design::warning()));
    }
    container(body)
        .padding(14)
        .width(Fill)
        .style(styles::card)
        .into()
}
