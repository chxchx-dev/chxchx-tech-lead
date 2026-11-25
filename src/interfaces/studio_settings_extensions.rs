//! Settings → Extensiones: the official extensions (ADR 0018). Each one is off until the person
//! reads what it creates, what it costs and how to remove it, and says yes. Turning it off keeps
//! the data; deleting it asks twice and leaves a copy first.
use super::*;
use crate::features::extensions::{self, Extension};
use crate::features::studio_preferences::ExtensionPrefs;
use iced::widget::column;

#[derive(Default)]
pub(super) struct State {
    /// The extension whose «what it does» is being shown before turning it on.
    asking: Option<&'static str>,
    /// The extension whose data is about to be deleted (second tap).
    deleting: Option<&'static str>,
    busy: bool,
    notice: String,
    failed: bool,
}

#[derive(Clone, Debug)]
pub(in crate::interfaces::studio) enum Event {
    /// Show what turning it on does.
    Ask(&'static str),
    Cancel,
    /// The person accepted: turn it on.
    Enable(&'static str),
    Disable(&'static str),
    AskDelete(&'static str),
    ConfirmDelete(&'static str),
    Prepared(Result<String, String>),
    Deleted(Result<Option<String>, String>),
    /// Write `.ai/board.json` after every change of this project (off by default).
    AutoExport(bool),
}

fn send(event: Event) -> Message {
    Message::Settings(settings::Event::Extensions(event))
}

fn now() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Ask(id) => {
            state.settings.extensions.asking = Some(id);
            state.settings.extensions.notice.clear();
        }
        Event::Cancel => {
            state.settings.extensions.asking = None;
            state.settings.extensions.deleting = None;
        }
        Event::Enable(id) => {
            if extensions::find(id).is_none() {
                return Task::none();
            }
            let accepted = now();
            state.session.edit(|preferences| {
                preferences.extensions.insert(
                    id.to_owned(),
                    ExtensionPrefs {
                        enabled: true,
                        accepted_at: Some(accepted),
                    },
                );
            });
            state.settings.extensions.asking = None;
            state.settings.extensions.notice.clear();
            return prepare(state, id);
        }
        Event::Disable(id) => {
            state.session.edit(|preferences| {
                if let Some(entry) = preferences.extensions.get_mut(id) {
                    entry.enabled = false;
                }
            });
            if id == extensions::HIVE && state.shell.side == shell::Side::Hive {
                state.shell.side = shell::Side::Explorer;
            }
            state.settings.extensions.notice =
                "Desactivada. Sus datos se conservaron: al volver a activarla los encuentras."
                    .into();
            state.settings.extensions.failed = false;
        }
        Event::AskDelete(id) => state.settings.extensions.deleting = Some(id),
        Event::ConfirmDelete(id) if !state.settings.extensions.busy => {
            state.settings.extensions.deleting = None;
            state.settings.extensions.busy = true;
            return delete(state, id);
        }
        Event::ConfirmDelete(_) => {}
        Event::Prepared(result) => {
            state.settings.extensions.busy = false;
            match result {
                Ok(text) => {
                    state.settings.extensions.notice = text;
                    state.settings.extensions.failed = false;
                }
                Err(error) => {
                    state.settings.extensions.notice = error;
                    state.settings.extensions.failed = true;
                }
            }
        }
        Event::Deleted(result) => {
            state.settings.extensions.busy = false;
            match result {
                Ok(copy) => {
                    state.settings.extensions.notice = match copy {
                        Some(copy) => format!("Datos borrados. Copia guardada en {copy}."),
                        None => "Este proyecto no tenía datos que borrar.".into(),
                    };
                    state.settings.extensions.failed = false;
                }
                Err(error) => {
                    state.settings.extensions.notice = error;
                    state.settings.extensions.failed = true;
                }
            }
        }
        Event::AutoExport(on) => state
            .session
            .edit_session(|session| session.hive_auto_export = on),
    }
    Task::none()
}

/// Creates (or migrates) the database of this project right away, so a failure shows now and not
/// the first time the board is opened.
#[cfg(feature = "hive")]
fn prepare(state: &mut Dashboard, id: &'static str) -> Task<Message> {
    if id != extensions::HIVE {
        return Task::none();
    }
    let root = state.root.clone();
    state.settings.extensions.busy = true;
    jobs::quiet(
        move || {
            crate::features::hive::store::open(&root).map(|(_, opened)| match opened.backup {
                Some(_) => "Colmena activada. Se actualizó su base y quedó una copia.".to_owned(),
                None => "Colmena activada.".to_owned(),
            })
        },
        |state, result| {
            settings::update(state, settings::Event::Extensions(Event::Prepared(result)))
        },
    )
}

#[cfg(not(feature = "hive"))]
fn prepare(_: &mut Dashboard, _: &'static str) -> Task<Message> {
    Task::none()
}

#[cfg(feature = "hive")]
fn delete(state: &mut Dashboard, id: &'static str) -> Task<Message> {
    if id != extensions::HIVE {
        state.settings.extensions.busy = false;
        return Task::none();
    }
    let root = state.root.clone();
    jobs::quiet(
        move || {
            crate::features::hive::store::remove(&root)
                .map(|copy| copy.map(|path| path.display().to_string()))
        },
        |state, result| {
            settings::update(state, settings::Event::Extensions(Event::Deleted(result)))
        },
    )
}

#[cfg(not(feature = "hive"))]
fn delete(state: &mut Dashboard, _: &'static str) -> Task<Message> {
    state.settings.extensions.busy = false;
    Task::none()
}

fn pill<'a>(label: &'a str, event: Event, primary: bool) -> iced::widget::Button<'a, Message> {
    button(text(label).size(12))
        .padding([6, 14])
        .style(if primary {
            styles::primary
        } else {
            styles::secondary
        })
        .on_press(send(event))
}

fn about<'a>(label: &'a str, content: &'a str) -> Element<'a, Message> {
    column![
        text(label).size(11).color(design::faint()),
        text(content).size(12).color(design::muted()),
    ]
    .spacing(2)
    .into()
}

fn extension_card<'a>(state: &'a Dashboard, extension: &'static Extension) -> Element<'a, Message> {
    let own = &state.settings.extensions;
    let enabled = state.session.preferences().extension_enabled(extension.id);
    let mut body = column![
        row![
            icons::icon(extension.icon, design::accent(), 18.0),
            text(extension.name).size(14).font(design::UI_BOLD),
            Space::new().width(Fill),
            text(if enabled { "Activada" } else { "Desactivada" })
                .size(11)
                .color(if enabled {
                    design::success()
                } else {
                    design::faint()
                }),
        ]
        .spacing(8)
        .align_y(iced::Center),
        text(extension.tagline).size(12).color(design::muted()),
    ]
    .spacing(8);

    let asking = own.asking == Some(extension.id);
    if asking || enabled {
        body = body
            .push(about("Qué crea", extension.creates))
            .push(about("Qué cuesta", extension.cost))
            .push(about("Cómo quitarla", extension.removal));
    }
    let actions: Element<'a, Message> = if asking {
        row![
            pill("Entiendo, activar", Event::Enable(extension.id), true),
            pill("Cancelar", Event::Cancel, false),
        ]
        .spacing(10)
        .into()
    } else if enabled {
        let delete = if own.deleting == Some(extension.id) {
            row![
                pill(
                    "Sí, borrar los datos de este proyecto",
                    Event::ConfirmDelete(extension.id),
                    true
                ),
                pill("No", Event::Cancel, false),
            ]
            .spacing(10)
        } else {
            row![pill(
                "Borrar datos de este proyecto…",
                Event::AskDelete(extension.id),
                false
            )]
        };
        row![
            pill("Desactivar", Event::Disable(extension.id), false),
            delete
        ]
        .spacing(10)
        .into()
    } else {
        row![pill("Activar…", Event::Ask(extension.id), true)].into()
    };
    body = body.push(actions);

    if enabled && extension.id == extensions::HIVE {
        let on = state.session.session().hive_auto_export;
        body = body.push(
            row![
                column![
                    text("Exportar el tablero a .ai/board.json").size(12),
                    text("Solo este proyecto. Apagado por defecto: en un repositorio público el plan se publicaría.")
                        .size(11)
                        .color(design::muted()),
                ]
                .spacing(2)
                .width(Fill),
                button(text(if on { "Sí" } else { "No" }).size(12))
                    .padding([4, 12])
                    .style(styles::ghost(on))
                    .on_press(send(Event::AutoExport(!on))),
            ]
            .spacing(10)
            .align_y(iced::Center),
        );
    }
    container(body)
        .padding(14)
        .width(Fill)
        .style(styles::card)
        .into()
}

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let own = &state.settings.extensions;
    let mut body = column![].spacing(10);
    for extension in extensions::all() {
        body = body.push(extension_card(state, extension));
    }
    if extensions::all().next().is_none() {
        body = body.push(
            text("Esta versión no incluye extensiones.")
                .size(12)
                .color(design::muted()),
        );
    }
    if own.busy {
        body = body.push(text("Trabajando…").size(11).color(design::muted()));
    }
    if !own.notice.is_empty() {
        body = body.push(text(own.notice.clone()).size(12).color(if own.failed {
            design::warning()
        } else {
            design::muted()
        }));
    }
    body.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn studio() -> Dashboard {
        super::super::super::editor::fixture()
    }

    fn event(state: &mut Dashboard, event: Event) {
        let _ = update(state, event);
    }

    #[test]
    fn everything_starts_off() {
        let state = studio();
        assert!(
            !state
                .session
                .preferences()
                .extension_enabled(extensions::HIVE)
        );
        assert!(
            !state.session.session().hive_auto_export,
            "never exported by surprise"
        );
    }

    #[cfg(feature = "hive")]
    #[test]
    fn asking_does_not_turn_it_on_and_cancel_goes_back() {
        let mut state = studio();
        event(&mut state, Event::Ask(extensions::HIVE));
        assert!(
            !state
                .session
                .preferences()
                .extension_enabled(extensions::HIVE)
        );
        event(&mut state, Event::Cancel);
        assert!(state.settings.extensions.asking.is_none());
    }

    #[cfg(feature = "hive")]
    #[test]
    fn accepting_turns_it_on_remembers_when_and_turning_off_keeps_the_choice_of_data() {
        let mut state = studio();
        // The update returns the task that would create the database; it is not run here.
        event(&mut state, Event::Enable(extensions::HIVE));
        let preferences = state.session.preferences();
        assert!(preferences.extension_enabled(extensions::HIVE));
        assert!(
            preferences.extensions[extensions::HIVE]
                .accepted_at
                .is_some()
        );
        event(&mut state, Event::Disable(extensions::HIVE));
        assert!(
            !state
                .session
                .preferences()
                .extension_enabled(extensions::HIVE)
        );
        assert!(
            state
                .session
                .preferences()
                .extensions
                .contains_key(extensions::HIVE),
            "the entry stays, so the acceptance is remembered"
        );
    }

    #[test]
    fn an_unknown_extension_cannot_be_turned_on() {
        let mut state = studio();
        event(&mut state, Event::Enable("nope"));
        assert!(!state.session.preferences().extension_enabled("nope"));
        assert!(state.session.preferences().extensions.is_empty());
    }

    #[test]
    fn the_card_draws_off_asking_and_on_without_panicking() {
        let mut state = studio();
        let _ = card(&state);
        event(&mut state, Event::Ask(extensions::HIVE));
        let _ = card(&state);
        state.session.edit(|p| {
            p.extensions.insert(
                extensions::HIVE.into(),
                ExtensionPrefs {
                    enabled: true,
                    accepted_at: None,
                },
            );
        });
        let _ = card(&state);
    }
}
