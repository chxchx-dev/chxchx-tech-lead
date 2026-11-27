//! Colmena in the sidebar (ADR 0018). Only reachable while the extension is on.
//!
//! The board of the open project: decisions waiting for the person, then the cards by status.
//! Every rule (dependencies, open decisions, releasing in chain) lives in `features::hive::board`;
//! this panel only reads a snapshot and calls those functions off the UI thread.
use super::*;
use crate::features::extensions;
use crate::features::hive::{board, export, store};
use iced::Color;
use iced::widget::{column, text_input};

#[path = "studio_hive_board.rs"]
mod kanban;
#[path = "studio_hive_style.rs"]
mod look;
pub(super) use kanban::{board_view, takes_center};

const ACTOR: &str = "persona";
pub(super) const INPUT: &str = "hive-new-card";

#[derive(Default)]
pub(super) struct State {
    cards: Vec<board::Card>,
    decisions: Vec<board::Decision>,
    title: String,
    error: String,
    /// Cards released by the last decision, so the person sees the chain reaction.
    released: Vec<String>,
    /// The board fills the main area (columns) instead of only the sidebar list.
    wide: bool,
    selected: Option<String>,
    dependency: String,
    reason: String,
}

type Snapshot = Result<(Vec<board::Card>, Vec<board::Decision>, board::Released), String>;

#[derive(Clone, Debug)]
pub(super) enum Event {
    Title(String),
    Add,
    Move(String, String),
    Decide(String, Option<String>),
    Wide(bool),
    Select(Option<String>),
    DependencyInput(String),
    AddDependency,
    WaitFor(String),
    ReasonInput(String),
    Block,
    Export,
    Loaded(Box<Snapshot>),
}

/// Whether the rail shows Colmena: the extension must be compiled in and turned on.
pub(super) fn visible(state: &Dashboard) -> bool {
    state
        .session
        .preferences()
        .extension_enabled(extensions::HIVE)
}

fn snapshot(
    root: &Path,
    auto_export: bool,
    change: impl FnOnce(&rusqlite::Connection) -> Result<board::Released, String>,
) -> Snapshot {
    let (db, _) = store::open(root)?;
    let released = change(&db)?;
    if auto_export {
        export::write(&db, root)?;
    }
    Ok((board::cards(&db)?, board::decisions(&db)?, released))
}

fn run(
    state: &mut Dashboard,
    change: impl FnOnce(&rusqlite::Connection) -> Result<board::Released, String> + Send + 'static,
) -> Task<Message> {
    let root = state.root.clone();
    let auto_export = state.session.session().hive_auto_export;
    jobs::quiet(
        move || snapshot(&root, auto_export, change),
        |_, result| Task::done(Message::Hive(Event::Loaded(Box::new(result)))),
    )
}

pub(super) fn refresh(state: &mut Dashboard) -> Task<Message> {
    if !visible(state) {
        return Task::none();
    }
    run(state, |_| Ok(board::Released::default()))
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Title(title) => {
            state.hive.title = title;
            Task::none()
        }
        Event::Add => {
            let title = std::mem::take(&mut state.hive.title);
            if title.trim().is_empty() {
                return Task::none();
            }
            run(state, move |db| {
                board::add_card(db, &title, "", ACTOR).map(|_| board::Released::default())
            })
        }
        Event::Move(key, status) => {
            run(state, move |db| board::set_status(db, &key, &status, ACTOR))
        }
        Event::Decide(key, outcome) => run(state, move |db| {
            board::resolve(db, &key, outcome.as_deref().map(|text| (text, "")), ACTOR)
        }),
        Event::Wide(on) => {
            state.hive.wide = on;
            Task::none()
        }
        Event::Select(key) => {
            state.hive.selected = key;
            state.hive.dependency.clear();
            state.hive.reason.clear();
            Task::none()
        }
        Event::DependencyInput(value) => {
            state.hive.dependency = value;
            Task::none()
        }
        Event::ReasonInput(value) => {
            state.hive.reason = value;
            Task::none()
        }
        Event::AddDependency => {
            let (Some(key), dep) = (
                state.hive.selected.clone(),
                state.hive.dependency.trim().to_uppercase(),
            ) else {
                return Task::none();
            };
            state.hive.dependency.clear();
            run(state, move |db| {
                board::add_dependency(db, &key, &dep).map(|_| board::Released::default())
            })
        }
        Event::WaitFor(decision) => {
            let Some(key) = state.hive.selected.clone() else {
                return Task::none();
            };
            run(state, move |db| {
                board::wait_for(db, &key, &decision, ACTOR).map(|_| board::Released::default())
            })
        }
        Event::Block => {
            let Some(key) = state.hive.selected.clone() else {
                return Task::none();
            };
            let reason = std::mem::take(&mut state.hive.reason);
            run(state, move |db| {
                board::block(db, &key, &reason, ACTOR).map(|_| board::Released::default())
            })
        }
        Event::Export => {
            let root = state.root.clone();
            jobs::quiet(
                move || {
                    let (db, _) = store::open(&root)?;
                    export::write(&db, &root)?;
                    Ok((
                        board::cards(&db)?,
                        board::decisions(&db)?,
                        board::Released::default(),
                    ))
                },
                |_, result| Task::done(Message::Hive(Event::Loaded(Box::new(result)))),
            )
        }
        Event::Loaded(result) => {
            match *result {
                Ok((cards, decisions, released)) => {
                    state.hive.cards = cards;
                    state.hive.decisions = decisions;
                    state.hive.error.clear();
                    state.hive.released = released.cards;
                }
                Err(error) => state.hive.error = error,
            }
            Task::none()
        }
    }
}

pub(super) fn small<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    button(text(label).size(11))
        .padding([3, 8])
        .style(styles::secondary)
        .on_press(message)
        .into()
}

pub(super) fn decision_card(item: &board::Decision) -> Element<'_, Message> {
    let mut choices = row![].spacing(4);
    let options: Vec<String> = if item.options.is_empty() {
        vec!["Aprobada".into()]
    } else {
        item.options.clone()
    };
    for option in options {
        let message = Message::Hive(Event::Decide(item.key.clone(), Some(option.clone())));
        choices = choices.push(
            button(text(option).size(11))
                .padding([3, 8])
                .style(styles::primary)
                .on_press(message),
        );
    }
    choices = choices.push(small(
        "Descartar",
        Message::Hive(Event::Decide(item.key.clone(), None)),
    ));
    container(
        column![
            key_chip(&item.key, design::warning()),
            text(&item.question)
                .size(12)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            choices.wrap(),
        ]
        .spacing(6),
    )
    .padding(10)
    .width(Fill)
    .style(look::panel(design::warning()))
    .into()
}

fn card_row(item: &board::Card, narrow: bool) -> Element<'_, Message> {
    let at = board::STATUSES
        .iter()
        .position(|s| *s == item.status)
        .unwrap_or(0);
    let mut actions = row![].spacing(4);
    if at > 0 {
        actions = actions.push(small(
            "◀",
            Message::Hive(Event::Move(
                item.key.clone(),
                board::STATUSES[at - 1].into(),
            )),
        ));
    }
    if at + 1 < board::STATUSES.len() {
        actions = actions.push(small(
            "▶",
            Message::Hive(Event::Move(
                item.key.clone(),
                board::STATUSES[at + 1].into(),
            )),
        ));
    }
    let tint = look::status_color(&item.status);
    let mut body = column![
        key_chip(&item.key, tint),
        text(&item.title)
            .size(12)
            .color(design::fg())
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    ]
    .spacing(4);
    if !item.blocker.is_empty() {
        body = body.push(text(&item.blocker).size(11).color(design::danger()));
    } else if !item.depends_on.is_empty() {
        body = body.push(
            text(format!("Depende de {}", item.depends_on.join(", ")))
                .size(11)
                .color(design::muted()),
        );
    }
    let inner: Element<'_, Message> = if narrow {
        column![body, actions].spacing(6).into()
    } else {
        row![body.width(Fill), actions]
            .spacing(6)
            .align_y(iced::Center)
            .into()
    };
    container(inner)
        .padding([8, 10])
        .width(Fill)
        .style(look::panel(tint))
        .into()
}

/// A key such as `C-004` as a small tinted chip.
fn key_chip<'a>(key: &'a str, tint: Color) -> Element<'a, Message> {
    container(text(key).size(10).font(design::MONO).color(tint))
        .padding([1, 6])
        .style(look::chip(tint))
        .into()
}

fn label(name: &str, count: usize) -> Element<'static, Message> {
    let tint = look::status_color(status_key(name));
    row![
        text("●").size(9).color(tint),
        text(name.to_uppercase())
            .size(10)
            .font(design::UI_BOLD)
            .color(design::muted()),
        Space::new().width(Fill),
        container(text(count.to_string()).size(10).color(tint))
            .padding([1, 7])
            .style(look::chip(tint)),
    ]
    .spacing(6)
    .align_y(iced::Center)
    .into()
}

/// Maps the visible name back to its status so the label can take its color.
fn status_key(name: &str) -> &'static str {
    board::STATUSES
        .iter()
        .copied()
        .find(|s| status_name(s) == name)
        .unwrap_or("backlog")
}

pub(super) fn status_name(status: &str) -> &str {
    match status {
        "backlog" => "Pendiente",
        "ready" => "Lista",
        "doing" => "En curso",
        "review" => "En revisión",
        "done" => "Hecha",
        "blocked" => "Bloqueada",
        other => other,
    }
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let hive = &state.hive;
    let open: Vec<&board::Decision> = hive
        .decisions
        .iter()
        .filter(|d| d.status == "proposed")
        .collect();
    let narrow = state.shell.explorer_width < 260.0;
    let mut body = column![
        row![
            icons::icon("hive", design::accent(), 18.0),
            text("Colmena").size(15).font(design::UI_BOLD),
            Space::new().width(Fill),
            small(
                match (hive.wide, narrow) {
                    (true, true) => "Cerrar",
                    (true, false) => "Cerrar tablero",
                    (false, true) => "Tablero",
                    (false, false) => "Tablero completo",
                },
                Message::Hive(Event::Wide(!hive.wide)),
            ),
        ]
        .spacing(8)
        .align_y(iced::Center),
        row![
            text_input("Nueva tarjeta…", &hive.title)
                .id(INPUT)
                .size(12)
                .on_input(|value| Message::Hive(Event::Title(value)))
                .on_submit(Message::Hive(Event::Add)),
            small("Añadir", Message::Hive(Event::Add)),
        ]
        .spacing(6),
    ]
    .spacing(10);
    if !hive.error.is_empty() {
        body = body.push(text(&hive.error).size(11).color(design::danger()));
    }
    if !hive.released.is_empty() {
        body = body.push(
            text(format!("Liberó: {}", hive.released.join(", ")))
                .size(11)
                .color(design::muted()),
        );
    }
    if !open.is_empty() {
        body = body.push(label("Esperan por ti", open.len()));
        for item in open {
            body = body.push(decision_card(item));
        }
    }
    for status in board::STATUSES {
        let cards: Vec<&board::Card> = hive.cards.iter().filter(|c| c.status == status).collect();
        if cards.is_empty() {
            continue;
        }
        body = body.push(label(status_name(status), cards.len()));
        for item in cards {
            body = body.push(card_row(item, narrow));
        }
    }
    if hive.cards.is_empty() && hive.decisions.is_empty() && hive.error.is_empty() {
        body = body.push(
            text("Aún no hay tarjetas. Añade una, o importa un tablero con `chxchx-tech hive import-vyro`.")
                .size(11)
                .color(design::muted()),
        );
    }
    container(
        column![
            row![
                text("COLMENA")
                    .size(10)
                    .font(design::UI_BOLD)
                    .color(design::faint()),
            ]
            .padding([0, 12]),
            styles::vscroll(container(body).padding([0, 12]).width(Fill)).height(Fill),
        ]
        .spacing(8),
    )
    .padding([10, 0])
    .width(state.shell.explorer_width)
    .height(Fill)
    .style(styles::surface(design::bg_panel()))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::studio_preferences::ExtensionPrefs;

    #[test]
    fn it_is_hidden_until_the_extension_is_on() {
        let mut state = super::super::editor::fixture();
        assert!(!visible(&state));
        state.session.edit(|preferences| {
            preferences.extensions.insert(
                extensions::HIVE.into(),
                ExtensionPrefs {
                    enabled: true,
                    accepted_at: None,
                },
            );
        });
        assert_eq!(visible(&state), cfg!(feature = "hive"));
        let _ = view(&state);
    }

    #[test]
    fn a_snapshot_fills_the_board_and_an_error_is_shown() {
        let mut state = super::super::editor::fixture();
        let (db, _) = crate::features::hive::store::open_at(
            &std::env::temp_dir().join(format!("hive-panel-{}.db", std::process::id())),
        )
        .unwrap();
        board::add_card(&db, "Primera", "", ACTOR).unwrap();
        let snap = Ok((
            board::cards(&db).unwrap(),
            board::decisions(&db).unwrap(),
            board::Released::default(),
        ));
        let _ = update(&mut state, Event::Loaded(Box::new(snap)));
        assert_eq!(state.hive.cards.len(), 1);
        let _ = view(&state);
        let _ = update(&mut state, Event::Loaded(Box::new(Err("boom".into()))));
        assert_eq!(state.hive.error, "boom");
        assert_eq!(state.hive.cards.len(), 1, "an error keeps the last board");
    }

    #[test]
    fn the_wide_board_selects_a_card_and_takes_the_main_area() {
        let mut state = super::super::editor::fixture();
        state.session.edit(|preferences| {
            preferences.extensions.insert(
                extensions::HIVE.into(),
                ExtensionPrefs {
                    enabled: true,
                    accepted_at: None,
                },
            );
        });
        state.shell.side = shell::Side::Hive;
        assert!(!takes_center(&state));
        let _ = update(&mut state, Event::Wide(true));
        assert!(takes_center(&state));
        let (db, _) = crate::features::hive::store::open_at(
            &std::env::temp_dir().join(format!("hive-wide-{}.db", std::process::id())),
        )
        .unwrap();
        board::add_card(&db, "Primera", "", ACTOR).unwrap();
        let snap = Ok((
            board::cards(&db).unwrap(),
            board::decisions(&db).unwrap(),
            board::Released::default(),
        ));
        let _ = update(&mut state, Event::Loaded(Box::new(snap)));
        let _ = update(&mut state, Event::Select(Some("C-001".into())));
        let _ = update(&mut state, Event::ReasonInput("sin acceso".into()));
        let _ = board_view(&state);
        let _ = update(&mut state, Event::Select(None));
        assert!(state.hive.selected.is_none() && state.hive.reason.is_empty());
        let _ = update(&mut state, Event::Wide(false));
        assert!(!takes_center(&state));
    }
}
