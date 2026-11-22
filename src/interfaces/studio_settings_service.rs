//! Settings → Servicio del nodo: restart the node's user service so it uses what was chosen in
//! Conexión (the port), or install it. Always in two steps like the command line: «Ver qué haría»
//! lists what changes; only «Confirmar» does it, and never while jobs are running.
use super::*;
use crate::features::node_service::{self, Overview, Preview};
use iced::widget::column;

#[derive(Default)]
pub(super) struct State {
    overview: Option<Overview>,
    preview: Option<Preview>,
    busy: bool,
    notice: String,
    failed: bool,
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Event {
    Refresh,
    Loaded(Overview),
    Preview,
    Previewed(Result<Preview, String>),
    Cancel,
    Confirm,
    Done(Result<Vec<String>, String>),
}

fn send(event: Event) -> Message {
    Message::Settings(settings::Event::Service(event))
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    let own = &mut state.settings.service;
    match event {
        Event::Refresh if !own.busy => {
            return jobs::quiet(
                || Event::Loaded(node_service::overview()),
                |state, event| settings::update(state, settings::Event::Service(event)),
            );
        }
        Event::Refresh => {}
        Event::Loaded(overview) => own.overview = Some(overview),
        Event::Preview if !own.busy => {
            own.busy = true;
            own.preview = None;
            own.failed = false;
            own.notice = "Mirando el servicio…".into();
            return jobs::quiet(node_service::preview, |state, result| {
                settings::update(state, settings::Event::Service(Event::Previewed(result)))
            });
        }
        Event::Preview => {}
        Event::Previewed(result) => {
            own.busy = false;
            match result {
                Ok(preview) => {
                    own.preview = Some(preview);
                    own.notice.clear();
                    own.failed = false;
                }
                Err(error) => {
                    own.notice = error;
                    own.failed = true;
                }
            }
        }
        Event::Cancel => {
            own.preview = None;
            own.notice.clear();
        }
        Event::Confirm => {
            let Some(preview) = own.preview.take() else {
                return Task::none();
            };
            own.busy = true;
            own.failed = false;
            own.notice = "Reiniciando… (hasta unos 20 segundos)".into();
            return jobs::quiet(
                move || node_service::apply(&preview),
                |state, result| {
                    let done =
                        settings::update(state, settings::Event::Service(Event::Done(result)));
                    Task::batch([
                        done,
                        settings::update(state, settings::Event::Service(Event::Refresh)),
                        settings::update(
                            state,
                            settings::Event::Node(settings::node::Event::Refresh),
                        ),
                    ])
                },
            );
        }
        Event::Done(result) => {
            own.busy = false;
            match result {
                Ok(lines) => {
                    own.notice = lines.last().cloned().unwrap_or_default();
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

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let own = &state.settings.service;
    let mut body = column![text("Servicio del nodo").size(14).font(design::UI_BOLD)].spacing(8);
    match &own.overview {
        None => body = body.push(line("Mirando el servicio…", false)),
        Some(overview) => {
            for text in node_service::describe(overview) {
                let warning = text.starts_with('⚠');
                body = body.push(line(text, warning));
            }
        }
    }
    if let Some(preview) = &own.preview {
        let mut shown = column![
            text("Vista previa (no se cambia nada hasta confirmar):")
                .size(12)
                .font(design::UI_BOLD)
        ]
        .spacing(4);
        for text in &preview.lines {
            shown = shown.push(line(text.clone(), false));
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
        let installed = own
            .overview
            .as_ref()
            .is_some_and(|overview| overview.installed);
        let mut act = button(
            text(if installed {
                "Reiniciar…"
            } else {
                "Instalar y arrancar…"
            })
            .size(12),
        )
        .padding([6, 14])
        .style(styles::primary);
        let mut refresh = button(text("Actualizar").size(12))
            .padding([6, 12])
            .style(styles::ghost(false));
        if !own.busy {
            act = act.on_press(send(Event::Preview));
            refresh = refresh.on_press(send(Event::Refresh));
        }
        body = body.push(row![act, refresh].spacing(10));
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
    use crate::integrations::user_service::Plan;

    fn preview() -> Preview {
        Preview {
            installing: false,
            plan: Plan {
                file: None,
                install: vec![],
                uninstall: vec![],
                restart: vec![],
                notes: vec![],
            },
            port: 50000,
            from_port: Some(47821),
            lines: vec!["Puerto del nodo: 47821 → 50000.".into()],
        }
    }

    fn overview(unit: u16, wanted: u16) -> Overview {
        Overview {
            installed: true,
            state: Some("active".into()),
            unit_port: Some(unit),
            wanted_port: wanted,
            running_port: Some(unit),
            running_jobs: Some(0),
        }
    }

    fn event(state: &mut Dashboard, event: Event) {
        let _ = update(state, event);
    }

    #[test]
    fn a_preview_waits_for_confirmation_and_cancelling_leaves_nothing_running() {
        let mut state = super::super::super::editor::fixture();
        event(&mut state, Event::Previewed(Ok(preview())));
        assert!(state.settings.service.preview.is_some());
        event(&mut state, Event::Cancel);
        assert!(state.settings.service.preview.is_none() && !state.settings.service.busy);
        event(&mut state, Event::Confirm);
        assert!(
            !state.settings.service.busy,
            "sin vista previa no se confirma nada"
        );
    }

    #[test]
    fn a_different_port_in_connection_is_flagged_and_results_are_said() {
        let said = node_service::describe(&overview(47821, 50000)).join("\n");
        assert!(
            said.contains("⚠") && said.contains("47821") && said.contains("50000"),
            "{said}"
        );
        let calm = node_service::describe(&overview(47821, 47821)).join("\n");
        assert!(!calm.contains("⚠"), "{calm}");
        let mut state = super::super::super::editor::fixture();
        event(
            &mut state,
            Event::Previewed(Err("Hay 1 trabajo(s) en curso".into())),
        );
        assert!(state.settings.service.failed && state.settings.service.preview.is_none());
        event(
            &mut state,
            Event::Done(Ok(vec![
                "Ejecutado".into(),
                "El nodo responde en el puerto 50000.".into(),
            ])),
        );
        assert!(!state.settings.service.failed);
        assert!(state.settings.service.notice.contains("50000"));
    }
}
