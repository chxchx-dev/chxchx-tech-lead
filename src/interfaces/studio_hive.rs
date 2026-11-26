//! Colmena in the sidebar (ADR 0018). Only reachable while the extension is on.
//!
//! The board of the open project: decisions waiting for the person, then the cards by status.
//! Every rule (dependencies, open decisions, releasing in chain) lives in `features::hive::board`;
//! this panel only reads a snapshot and calls those functions off the UI thread.
use super::*;
use crate::features::extensions;
use crate::features::hive::{board, store};
use iced::widget::{column, text_input};

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
}

type Snapshot = Result<(Vec<board::Card>, Vec<board::Decision>, board::Released), String>;

#[derive(Clone, Debug)]
pub(super) enum Event {
    Title(String),
    Add,
    Move(String, String),
    Decide(String, Option<String>),
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
    change: impl FnOnce(&rusqlite::Connection) -> Result<board::Released, String>,
) -> Snapshot {
    let (db, _) = store::open(root)?;
    let released = change(&db)?;
    Ok((board::cards(&db)?, board::decisions(&db)?, released))
}

fn run(
    state: &mut Dashboard,
    change: impl FnOnce(&rusqlite::Connection) -> Result<board::Released, String> + Send + 'static,
) -> Task<Message> {
    let root = state.root.clone();
    jobs::quiet(
        move || snapshot(&root, change),
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

fn small<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    button(text(label).size(11))
        .padding([3, 8])
        .style(styles::secondary)
        .on_press(message)
        .into()
}

fn decision_card(item: &board::Decision) -> Element<'_, Message> {
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
            text(format!("{} · {}", item.key, item.question)).size(12),
            choices.wrap(),
        ]
        .spacing(6),
    )
    .padding(10)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn card_row(item: &board::Card) -> Element<'_, Message> {
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
    let mut body = column![text(format!("{} · {}", item.key, item.title)).size(12)].spacing(4);
    if !item.blocker.is_empty() {
        body = body.push(text(&item.blocker).size(11).color(design::muted()));
    } else if !item.depends_on.is_empty() {
        body = body.push(
            text(format!("Depende de {}", item.depends_on.join(", ")))
                .size(11)
                .color(design::muted()),
        );
    }
    container(
        row![body.width(Fill), actions]
            .spacing(6)
            .align_y(iced::Center),
    )
    .padding([6, 10])
    .width(Fill)
    .style(styles::card)
    .into()
}

fn label(name: &str, count: usize) -> Element<'static, Message> {
    text(format!("{} ({count})", name.to_uppercase()))
        .size(10)
        .font(design::UI_BOLD)
        .color(design::faint())
        .into()
}

fn status_name(status: &str) -> &str {
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
    let mut body = column![
        row![
            icons::icon("hive", design::accent(), 18.0),
            text("Colmena").size(15).font(design::UI_BOLD),
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
            body = body.push(card_row(item));
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
}
