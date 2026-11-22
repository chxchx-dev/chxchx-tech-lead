//! Settings → Carpeta de datos: where ChxChx keeps its data. Changing it is a copy in two steps like
//! the command line: «Ver qué copiaría» checks the new folder and counts; only «Copiar y usar» does
//! it. The folder in use now is never touched (it stays as the backup) and «Volver a la original»
//! undoes the change. Studio and the node follow the new folder after restarting.
use super::*;
use crate::features::data_home::{self, Plan};
use iced::widget::{column, text_input};

#[derive(Default)]
pub(super) struct State {
    typed: String,
    plan: Option<Plan>,
    busy: bool,
    notice: String,
    failed: bool,
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Event {
    Type(String),
    Preview,
    Previewed(Result<Plan, String>),
    Cancel,
    Confirm,
    Back,
    Done(Result<Vec<String>, String>),
}

fn send(event: Event) -> Message {
    Message::Settings(settings::Event::Data(event))
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    let own = &mut state.settings.data;
    match event {
        Event::Type(value) => {
            own.typed = value.chars().take(400).collect();
            own.plan = None;
            own.notice.clear();
        }
        Event::Preview if !own.busy => {
            let typed = own.typed.clone();
            own.busy = true;
            own.plan = None;
            own.failed = false;
            own.notice = "Mirando la carpeta…".into();
            return jobs::quiet(
                move || data_home::preview(&typed),
                |state, result| {
                    settings::update(state, settings::Event::Data(Event::Previewed(result)))
                },
            );
        }
        Event::Preview => {}
        Event::Previewed(result) => {
            own.busy = false;
            match result {
                Ok(plan) => {
                    own.plan = Some(plan);
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
            own.plan = None;
            own.notice.clear();
        }
        Event::Confirm => {
            let Some(plan) = own.plan.take() else {
                return Task::none();
            };
            own.busy = true;
            own.failed = false;
            own.notice = "Copiando… (puede tardar)".into();
            return jobs::quiet(
                move || data_home::apply(&plan),
                |state, result| settings::update(state, settings::Event::Data(Event::Done(result))),
            );
        }
        Event::Back if !own.busy => {
            own.busy = true;
            own.plan = None;
            return jobs::quiet(data_home::back, |state, result| {
                settings::update(state, settings::Event::Data(Event::Done(result)))
            });
        }
        Event::Back => {}
        Event::Done(result) => {
            own.busy = false;
            match result {
                Ok(lines) => {
                    own.notice = lines.join(" ");
                    own.failed = false;
                    own.typed.clear();
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
    let own = &state.settings.data;
    let mut body = column![text("Carpeta de datos").size(14).font(design::UI_BOLD)].spacing(8);
    for text in data_home::describe() {
        let warning = text.starts_with('⚠');
        body = body.push(line(text, warning));
    }
    body = body.push(
        text_input(
            "Carpeta nueva (ruta completa, vacía o que no exista)",
            &own.typed,
        )
        .on_input(|value| send(Event::Type(value)))
        .on_submit(send(Event::Preview))
        .size(12)
        .padding([6, 10])
        .width(Fill)
        .style(styles::input),
    );
    if let Some(plan) = &own.plan {
        let mut shown = column![
            text("Vista previa (no se cambia nada hasta confirmar):")
                .size(12)
                .font(design::UI_BOLD)
        ]
        .spacing(4);
        for text in [
            format!(
                "Copiaría {} archivos ({}) de {} a {}.",
                plan.files,
                data_home::human(plan.bytes),
                plan.from.display(),
                plan.to.display()
            ),
            "La carpeta de ahora no se toca: queda como respaldo y la borras tú cuando todo funcione."
                .to_owned(),
            "Después hay que reiniciar Studio y el nodo. Para deshacerlo: «Volver a la original».".to_owned(),
        ] {
            shown = shown.push(line(text, false));
        }
        body = body.push(container(shown).padding(12).width(Fill).style(styles::well));
        body = body.push(
            row![
                button(text("Copiar y usar").size(12))
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
        let mut preview = button(text("Ver qué copiaría").size(12))
            .padding([6, 14])
            .style(styles::primary);
        let mut back = button(text("Volver a la original").size(12))
            .padding([6, 12])
            .style(styles::secondary);
        if !own.busy {
            if !own.typed.trim().is_empty() {
                preview = preview.on_press(send(Event::Preview));
            }
            back = back.on_press(send(Event::Back));
        }
        body = body.push(row![preview, back].spacing(10));
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
            from: "/home/a/.chxchx-tech-lead".into(),
            to: "/data/chxchx".into(),
            files: 12,
            bytes: 2048,
        }
    }

    fn event(state: &mut Dashboard, event: Event) {
        let _ = update(state, event);
    }

    #[test]
    fn a_preview_waits_for_confirmation_and_typing_again_discards_it() {
        let mut state = super::super::super::editor::fixture();
        event(&mut state, Event::Previewed(Ok(plan())));
        assert!(state.settings.data.plan.is_some() && !state.settings.data.busy);
        event(&mut state, Event::Type("/otra".into()));
        assert!(
            state.settings.data.plan.is_none(),
            "otra carpeta pide otra vista previa"
        );
        event(&mut state, Event::Previewed(Ok(plan())));
        event(&mut state, Event::Cancel);
        assert!(state.settings.data.plan.is_none() && !state.settings.data.busy);
    }

    #[test]
    fn confirming_without_a_preview_does_nothing_and_problems_are_said() {
        let mut state = super::super::super::editor::fixture();
        event(&mut state, Event::Confirm);
        assert!(!state.settings.data.busy && state.settings.data.notice.is_empty());
        event(&mut state, Event::Previewed(Err("no está vacía".into())));
        assert!(state.settings.data.failed && state.settings.data.plan.is_none());
        event(
            &mut state,
            Event::Done(Ok(vec!["Copiados.".into(), "Reinicia.".into()])),
        );
        assert!(!state.settings.data.failed);
        assert_eq!(state.settings.data.notice, "Copiados. Reinicia.");
    }
}
