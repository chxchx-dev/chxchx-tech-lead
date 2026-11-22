//! Settings → Acceso desde fuera: turn the node's reach through the person's own Tailscale on and
//! off from Studio. Always in two steps, like the command line: «Ver qué haría» shows the address,
//! the command and what it means; only «Confirmar» applies it. The node is never put on the
//! Internet. The HTTPS port is the one chosen in Conexión.
use super::*;
use crate::features::remote_access::{self, Overview};
use crate::integrations::tailscale::{Disable, Plan};
use iced::widget::column;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Step {
    Enable(Plan),
    Disable(Disable),
}

#[derive(Default)]
pub(super) struct State {
    overview: Option<Overview>,
    step: Option<Step>,
    busy: bool,
    notice: String,
    failed: bool,
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Event {
    Refresh,
    Loaded(Overview),
    PreviewEnable,
    PreviewDisable,
    Previewed(Result<Option<Step>, String>),
    Cancel,
    Confirm,
    Done(Result<String, String>),
}

#[cfg(test)]
pub(super) static TEST_CLI: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn cli() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        TEST_CLI
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
            .ok_or_else(|| remote_access::NO_TAILSCALE.to_owned())
    }
    #[cfg(not(test))]
    {
        remote_access::locate()
    }
}

fn dir() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        super::identity::home()
            .map(|home| home.join("node"))
            .ok_or_else(|| "No se pudo resolver la carpeta de ChxChx".to_owned())
    }
    #[cfg(not(test))]
    {
        crate::features::node_state::dir()
            .ok_or_else(|| "No se pudo resolver la carpeta de ChxChx".to_owned())
    }
}

fn send(event: Event) -> Message {
    Message::Settings(settings::Event::Remote(event))
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Refresh => {
            if state.settings.remote.busy {
                return Task::none();
            }
            return jobs::quiet(
                || match dir() {
                    Ok(dir) => Event::Loaded(remote_access::overview_with(&dir, cli())),
                    Err(error) => Event::Done(Err(error)),
                },
                |state, event| settings::update(state, settings::Event::Remote(event)),
            );
        }
        Event::Loaded(overview) => state.settings.remote.overview = Some(overview),
        Event::PreviewEnable | Event::PreviewDisable => {
            let enabling = matches!(event, Event::PreviewEnable);
            let https = state.settings.connection.saved_https_port();
            let own = &mut state.settings.remote;
            own.busy = true;
            own.step = None;
            own.notice = "Mirando Tailscale…".into();
            own.failed = false;
            return jobs::quiet(
                move || {
                    let dir = dir()?;
                    let tailscale = cli()?;
                    if enabling {
                        remote_access::preview_enable(&dir, &tailscale, None, https)
                            .map(|plan| Some(Step::Enable(plan)))
                    } else {
                        remote_access::preview_disable(&dir, &tailscale)
                            .map(|found| found.map(|(_, plan)| Step::Disable(plan)))
                    }
                },
                |state, result| {
                    settings::update(state, settings::Event::Remote(Event::Previewed(result)))
                },
            );
        }
        Event::Previewed(result) => {
            let own = &mut state.settings.remote;
            own.busy = false;
            own.failed = false;
            match result {
                Ok(Some(step)) => {
                    own.step = Some(step);
                    own.notice.clear();
                }
                Ok(None) => own.notice = "El acceso desde fuera no está activado.".into(),
                Err(error) => {
                    own.notice = error;
                    own.failed = true;
                }
            }
        }
        Event::Cancel => {
            state.settings.remote.step = None;
            state.settings.remote.notice.clear();
        }
        Event::Confirm => {
            let own = &mut state.settings.remote;
            let Some(step) = own.step.take() else {
                return Task::none();
            };
            own.busy = true;
            own.notice = "Aplicando…".into();
            return jobs::quiet(
                move || {
                    let dir = dir()?;
                    let tailscale = cli()?;
                    match step {
                        Step::Enable(plan) => remote_access::enable(&dir, &tailscale, &plan)
                            .map(|_| format!("Listo: {}", plan.url())),
                        Step::Disable(plan) => remote_access::disable(&dir, &tailscale, plan)
                            .map(|()| "Acceso desde fuera desactivado.".to_owned()),
                    }
                },
                |state, result| {
                    let refresh =
                        settings::update(state, settings::Event::Remote(Event::Done(result)));
                    Task::batch([
                        refresh,
                        settings::update(state, settings::Event::Remote(Event::Refresh)),
                    ])
                },
            );
        }
        Event::Done(result) => {
            let own = &mut state.settings.remote;
            own.busy = false;
            match result {
                Ok(text) => {
                    own.notice = text;
                    own.failed = false;
                }
                Err(error) => {
                    own.notice = error;
                    own.failed = true;
                }
            }
        }
    }
    Task::none()
}

fn line<'a>(content: impl Into<String>, warning: bool) -> Element<'a, Message> {
    text(content.into())
        .size(12)
        .color(if warning {
            design::warning()
        } else {
            design::muted()
        })
        .into()
}

fn preview(step: &Step, host_hint: &str) -> Vec<String> {
    match step {
        Step::Enable(plan) => {
            let mut lines = vec![
                format!("Equipo: {}", plan.host),
                format!(
                    "Dirección: {} (solo dentro de tu tailnet, nunca en Internet)",
                    plan.url()
                ),
            ];
            lines.push(if plan.already_in_place {
                format!(
                    "Tailscale ya sirve {} en el puerto {}: no hay que tocarlo.",
                    plan.target, plan.https_port
                )
            } else {
                format!("Comando: {}", plan.command.join(" "))
            });
            lines.push(
                "Efecto: el nodo, que solo escucha en 127.0.0.1, responderá también a ese nombre. \
                 Cada dispositivo debe vincularse con un código para hacer algo."
                    .into(),
            );
            lines.push(
                "Ten en cuenta: Tailscale emite un certificado HTTPS (la primera vez tarda hasta ~1 min) \
                 y el nombre de la máquina queda en los registros públicos de transparencia de certificados."
                    .into(),
            );
            lines.push("Para deshacerlo: «Desactivar».".into());
            lines
        }
        Step::Disable(plan) => {
            let mut lines = vec![format!("El nodo dejará de responder a {host_hint}.")];
            lines.push(match plan {
                Disable::Remove { https_port } => {
                    format!("Comando: tailscale serve --https={https_port} off")
                }
                Disable::NothingToRemove => "Tailscale ya no sirve ese puerto.".into(),
                Disable::NotOurs { https_port } => format!(
                    "El puerto {https_port} de Tailscale sirve algo que no es un proxy local: no lo toco."
                ),
            });
            lines
        }
    }
}

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let own = &state.settings.remote;
    let mut body = column![
        text("Acceso desde fuera (Tailscale)")
            .size(14)
            .font(design::UI_BOLD)
    ]
    .spacing(8);
    match &own.overview {
        None => body = body.push(line("Mirando Tailscale…", false)),
        Some(overview) => {
            for text in remote_access::describe(overview) {
                let warning = text.starts_with('⚠') || text.starts_with("No encuentro");
                body = body.push(line(text, warning));
            }
        }
    }
    if let Some(step) = &own.step {
        let host = own
            .overview
            .as_ref()
            .and_then(|overview| overview.remote.as_ref())
            .map_or("este nombre".to_owned(), |remote| remote.host.clone());
        let mut shown = column![
            text("Vista previa (no se cambia nada hasta confirmar):")
                .size(12)
                .font(design::UI_BOLD)
        ]
        .spacing(4);
        for text in preview(step, &host) {
            shown = shown.push(line(text, false));
        }
        body = body.push(container(shown).padding(12).width(Fill).style(styles::well));
        body = body.push(
            row![
                button(text("Confirmar").size(12))
                    .padding([6, 14])
                    .style(styles::primary)
                    .on_press(send(Event::Confirm)),
                button(text("Cancelar").size(12))
                    .padding([6, 12])
                    .style(styles::secondary)
                    .on_press(send(Event::Cancel)),
            ]
            .spacing(10),
        );
    } else {
        let on = own
            .overview
            .as_ref()
            .is_some_and(|overview| overview.remote.is_some());
        let ready = own
            .overview
            .as_ref()
            .is_some_and(|overview| overview.cli.is_ok());
        let free = !own.busy && ready;
        let mut enable = button(text(if on { "Volver a activar" } else { "Activar…" }).size(12))
            .padding([6, 14])
            .style(styles::primary);
        let mut disable = button(text("Desactivar…").size(12))
            .padding([6, 12])
            .style(styles::secondary);
        let mut refresh = button(text("Actualizar").size(12))
            .padding([6, 12])
            .style(styles::ghost(false));
        if free {
            enable = enable.on_press(send(Event::PreviewEnable));
            if on {
                disable = disable.on_press(send(Event::PreviewDisable));
            }
        }
        if !own.busy {
            refresh = refresh.on_press(send(Event::Refresh));
        }
        body = body.push(row![enable, disable, refresh].spacing(10));
    }
    if !own.notice.is_empty() {
        body = body.push(line(own.notice.clone(), own.failed));
    }
    container(body)
        .padding(20)
        .width(Fill)
        .style(styles::card)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> Plan {
        Plan {
            host: "fedora.tail33e808.ts.net".into(),
            https_port: 8443,
            target: "http://127.0.0.1:47821".into(),
            command: vec![
                "tailscale".into(),
                "serve".into(),
                "--bg".into(),
                "--https=8443".into(),
                "http://127.0.0.1:47821".into(),
            ],
            already_in_place: false,
        }
    }

    fn event(state: &mut Dashboard, event: Event) {
        let _ = update(state, event);
    }

    #[test]
    fn a_preview_waits_for_confirmation_and_can_be_cancelled_without_touching_anything() {
        let mut state = super::super::super::editor::fixture();
        event(&mut state, Event::Previewed(Ok(Some(Step::Enable(plan())))));
        assert!(state.settings.remote.step.is_some());
        let shown = preview(state.settings.remote.step.as_ref().unwrap(), "x").join("\n");
        assert!(
            shown.contains("https://fedora.tail33e808.ts.net:8443/"),
            "{shown}"
        );
        assert!(
            shown.contains("tailscale serve --bg --https=8443"),
            "{shown}"
        );
        assert!(shown.contains("nunca en Internet") && !shown.to_lowercase().contains("funnel"));
        event(&mut state, Event::Cancel);
        assert!(state.settings.remote.step.is_none());
        assert!(
            !state.settings.remote.busy,
            "cancelar no deja nada en marcha"
        );
    }

    #[test]
    fn confirming_without_a_preview_does_nothing() {
        let mut state = super::super::super::editor::fixture();
        event(&mut state, Event::Confirm);
        assert!(!state.settings.remote.busy);
        assert!(state.settings.remote.notice.is_empty());
    }

    #[test]
    fn problems_and_results_are_said_in_words() {
        let mut state = super::super::super::editor::fixture();
        event(
            &mut state,
            Event::Previewed(Err("Tailscale no responde".into())),
        );
        assert!(state.settings.remote.failed && state.settings.remote.step.is_none());
        event(&mut state, Event::Previewed(Ok(None)));
        assert!(state.settings.remote.notice.contains("no está activado"));
        event(&mut state, Event::Done(Ok("Listo".into())));
        assert!(!state.settings.remote.failed && state.settings.remote.notice == "Listo");
        let off = preview(
            &Step::Disable(Disable::NotOurs { https_port: 443 }),
            "a.ts.net",
        );
        assert!(off.join(" ").contains("no lo toco"));
    }
}
