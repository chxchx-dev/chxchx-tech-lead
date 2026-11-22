//! Settings → Nodo local (ADR 0017, H5): is it running, should chat turns go to it, start it, and
//! the devices linked to it (see them, link one with a code, unlink one).
//!
//! Everything that talks to the node runs in the background; the screen only shows the last thing
//! read. Nothing here is needed to use Studio: with the node off, Studio runs everything itself.
use super::*;
use crate::integrations::node_link::{self, Endpoint};
use crate::integrations::node_process;
use chxchx_protocol::{DeviceId, PairingOffer, Role};
use iced::widget::column;

pub(super) use node_link::{Overview as Snapshot, RunningNode as Running};

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
    let mut overview = node_link::overview();
    overview.remote_warning = remote_warning(&overview);
    overview
}

/// Remote access is on but `tailscale serve` does not lead to the node: the phone would see a 502.
#[cfg(feature = "node")]
fn remote_warning(overview: &Snapshot) -> Option<String> {
    use crate::integrations::tailscale;
    let remote = crate::features::node_state::dir()
        .and_then(|dir| crate::features::node_state::read_remote(&dir))?;
    let cli = tailscale::locate()?;
    let config = tailscale::serve_config(&cli).ok()?;
    let port = overview.running.as_ref().map(|node| node.port);
    explain(tailscale::reach(&config, remote.https_port, port))
}

#[cfg(not(feature = "node"))]
fn remote_warning(_: &Snapshot) -> Option<String> {
    None
}

/// What each way of not reaching the node means for the person.
#[cfg(feature = "node")]
fn explain(reach: crate::integrations::tailscale::Reach) -> Option<String> {
    use crate::integrations::tailscale::Reach;
    match reach {
        Reach::Serving(_) | Reach::NodeDown(_) => None,
        Reach::WrongPort { node_port, .. } => Some(format!(
            "El celular verá un error 502: Tailscale apunta a otro puerto y el nodo está en el {node_port}. \
             Corrígelo con `chxchx-tech node remote enable --apply`."
        )),
        Reach::Other => Some(
            "El puerto de acceso remoto sirve otra cosa: el celular no llegará al nodo. \
             Revisa `chxchx-tech node remote status`."
                .to_owned(),
        ),
        Reach::NotServed => Some(
            "Tailscale ya no sirve el nodo: el celular no llegará. Vuelve a activarlo con \
             `chxchx-tech node remote enable --apply`."
                .to_owned(),
        ),
    }
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
    if let Some(warning) = &snapshot.remote_warning {
        body = body.push(text(warning.clone()).size(12).color(design::danger()));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chxchx_protocol::Device;

    fn studio() -> Dashboard {
        super::super::super::editor::fixture()
    }

    fn device(id: &str, name: &str) -> Device {
        Device {
            id: DeviceId::parse(id).unwrap(),
            name: name.into(),
            role: Role::Operator,
            created_ms: 1,
            last_seen_ms: None,
        }
    }

    fn running(devices: Option<Vec<Device>>) -> Snapshot {
        Snapshot {
            running: Some(Running {
                port: 47821,
                version: "0.7.1".into(),
                running: 1,
                queued: 2,
                devices,
            }),
            address: Some("https://pc.tail1234.ts.net/".into()),
            remote_warning: None,
        }
    }

    fn stopped() -> Snapshot {
        Snapshot {
            running: None,
            address: None,
            remote_warning: None,
        }
    }

    fn event(state: &mut Dashboard, event: Event) {
        let _ = update(state, event);
    }

    #[test]
    fn what_the_node_says_is_shown_and_a_stopped_node_forgets_what_depended_on_it() {
        let mut state = studio();
        event(
            &mut state,
            Event::Loaded(running(Some(vec![device("d1", "Pixel")]))),
        );
        let node = &state.settings.node;
        let shown = node
            .snapshot
            .as_ref()
            .and_then(|s| s.running.as_ref())
            .unwrap();
        assert_eq!((shown.port, shown.running, shown.queued), (47821, 1, 2));

        // A code and a pending unlink make no sense once the node is gone.
        event(&mut state, Event::AskUnlink(DeviceId::parse("d1").unwrap()));
        state.settings.node.code = Some(("ABCDE-FGH23".into(), None, 0));
        event(&mut state, Event::Loaded(stopped()));
        assert!(state.settings.node.code.is_none());
        assert!(state.settings.node.confirming.is_none());
        assert!(
            state
                .settings
                .node
                .snapshot
                .as_ref()
                .unwrap()
                .running
                .is_none()
        );
    }

    #[test]
    fn unlinking_takes_two_taps_and_can_be_called_off() {
        let mut state = studio();
        let id = DeviceId::parse("d1").unwrap();
        event(
            &mut state,
            Event::Loaded(running(Some(vec![device("d1", "Pixel")]))),
        );

        event(&mut state, Event::AskUnlink(id.clone()));
        assert_eq!(state.settings.node.confirming.as_ref(), Some(&id));
        assert!(!state.settings.node.busy, "asking changes nothing yet");

        event(&mut state, Event::CancelUnlink);
        assert!(state.settings.node.confirming.is_none());
        assert!(!state.settings.node.busy);

        event(&mut state, Event::AskUnlink(id.clone()));
        event(&mut state, Event::Unlink(id));
        assert!(
            state.settings.node.confirming.is_none(),
            "the question is over"
        );
        assert!(state.settings.node.busy, "the request is in flight");
    }

    #[test]
    fn the_result_of_an_unlink_or_a_stop_is_told_to_the_person() {
        let mut state = studio();
        state.settings.node.busy = true;
        event(&mut state, Event::Unlinked(Ok(())));
        assert!(!state.settings.node.busy);
        assert!(state.settings.node.notice.contains("desvinculado"));

        event(
            &mut state,
            Event::Unlinked(Err("El nodo no responde.".into())),
        );
        assert_eq!(state.settings.node.notice, "El nodo no responde.");

        event(&mut state, Event::Stopped(Ok(())));
        assert!(state.settings.node.notice.contains("deteniendo"));
        event(&mut state, Event::Started(Err("puerto ocupado".into())));
        assert_eq!(state.settings.node.notice, "puerto ocupado");
    }

    #[test]
    fn a_pairing_code_comes_with_the_address_to_open_when_remote_access_is_on() {
        let mut state = studio();
        let offer = || PairingOffer {
            code: "ABCDE-FGH23".into(),
            role: Role::Operator,
            expires_ms: 99,
        };
        event(&mut state, Event::Loaded(running(None)));
        event(&mut state, Event::Code(Ok(offer())));
        let (code, address, _) = state.settings.node.code.clone().unwrap();
        assert_eq!(code, "ABCDE-FGH23");
        assert_eq!(
            address.as_deref(),
            Some("https://pc.tail1234.ts.net/#code=ABCDE-FGH23"),
            "the code goes after the #: it never reaches a server"
        );

        // Without remote access there is no address to give, only the code.
        let mut local = studio();
        event(
            &mut local,
            Event::Loaded(Snapshot {
                address: None,
                ..running(None)
            }),
        );
        event(&mut local, Event::Code(Ok(offer())));
        assert_eq!(local.settings.node.code.unwrap().1, None);
    }

    #[test]
    fn a_refused_pairing_code_shows_why_and_keeps_no_code() {
        let mut state = studio();
        state.settings.node.busy = true;
        event(
            &mut state,
            Event::Code(Err("Vinculación cerrada por intentos fallidos".into())),
        );
        assert!(!state.settings.node.busy);
        assert!(state.settings.node.code.is_none());
        assert!(state.settings.node.notice.contains("cerrada"));
    }

    #[test]
    fn refreshing_while_something_is_in_flight_does_nothing() {
        let mut state = studio();
        state.settings.node.snapshot = Some(stopped());
        state.settings.node.busy = true;
        event(&mut state, Event::Refresh);
        assert!(state.settings.node.busy, "still the same request");
    }

    #[test]
    fn using_the_node_for_chat_turns_is_remembered() {
        let mut state = studio();
        assert!(!state.session.preferences().node.delegate, "off by default");
        event(&mut state, Event::Delegate(true));
        assert!(state.session.preferences().node.delegate);
        event(&mut state, Event::Delegate(false));
        assert!(!state.session.preferences().node.delegate);
    }

    #[cfg(feature = "node")]
    #[test]
    fn a_tailscale_that_does_not_lead_to_the_node_is_explained_in_words() {
        use crate::integrations::tailscale::Reach;
        assert_eq!(
            explain(Reach::Serving("http://127.0.0.1:47821".into())),
            None
        );
        assert_eq!(
            explain(Reach::NodeDown("http://127.0.0.1:47821".into())),
            None
        );
        let wrong = explain(Reach::WrongPort {
            proxy: "http://127.0.0.1:47821".into(),
            node_port: 47900,
        })
        .unwrap();
        assert!(wrong.contains("502") && wrong.contains("47900"), "{wrong}");
        assert!(wrong.contains("node remote enable --apply"), "{wrong}");
        assert!(explain(Reach::NotServed).unwrap().contains("ya no sirve"));
        assert!(explain(Reach::Other).unwrap().contains("otra cosa"));
    }

    #[test]
    fn the_warning_is_kept_with_what_the_node_said() {
        let mut state = studio();
        let mut snapshot = running(Some(vec![]));
        snapshot.remote_warning = Some("El celular verá un error 502".into());
        event(&mut state, Event::Loaded(snapshot));
        let kept = state.settings.node.snapshot.as_ref().unwrap();
        assert!(kept.remote_warning.as_deref().unwrap().contains("502"));
        let _ = card(&state);
    }

    #[test]
    fn the_card_draws_in_every_state_without_panicking() {
        let mut state = studio();
        let id = DeviceId::parse("d1").unwrap();
        let mut states: Vec<Option<Snapshot>> = vec![
            None,
            Some(stopped()),
            Some(running(None)),
            Some(running(Some(vec![]))),
            Some(running(Some(vec![
                device("d1", "Pixel"),
                device("d2", "Tablet"),
            ]))),
        ];
        for snapshot in states.drain(..) {
            state.settings.node.snapshot = snapshot;
            for (busy, confirming, code, notice) in [
                (false, None, None, ""),
                (
                    true,
                    Some(id.clone()),
                    Some(("ABCDE-FGH23".to_owned(), None, 0)),
                    "algo",
                ),
            ] {
                state.settings.node.busy = busy;
                state.settings.node.confirming = confirming;
                state.settings.node.code = code;
                state.settings.node.notice = notice.to_owned();
                let _ = card(&state);
            }
        }
    }
}
