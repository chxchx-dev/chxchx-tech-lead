//! Project search: the "Buscar" view of the left sidebar.
use super::*;
use crate::features::project_search::{SearchHit, SearchResults, search};
use iced::widget::{column, text_input};
use std::path::Path;

pub(super) const INPUT: &str = "project-search";

pub(super) struct State {
    pub query: String,
    /// Search file contents (true) or only file names (false).
    pub content: bool,
    pub results: Option<SearchResults>,
    pub running: bool,
    /// Query the current results belong to, shown in the summary.
    searched: String,
    generation: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            query: String::new(),
            content: true,
            results: None,
            running: false,
            searched: String::new(),
            generation: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Query(String),
    Run,
    Mode(bool),
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Query(query) => {
            if query.trim().is_empty() {
                state.search.results = None;
                state.search.running = false;
                state.search.generation += 1;
            }
            state.search.query = query;
        }
        Event::Mode(content) => {
            state.search.content = content;
            if !state.search.query.trim().is_empty() {
                return update(state, Event::Run);
            }
        }
        Event::Run => {
            let query = state.search.query.trim().to_owned();
            if query.is_empty() {
                return Task::none();
            }
            state.search.generation += 1;
            let generation = state.search.generation;
            state.search.running = true;
            let (root, content) = (state.root.clone(), state.search.content);
            let wanted = query.clone();
            return jobs::run(
                state,
                move || search(&root, &query, content),
                move |state, result| {
                    // A newer search or a cleared box makes this answer obsolete.
                    if state.search.generation != generation {
                        return Task::none();
                    }
                    state.search.running = false;
                    state.search.searched = wanted;
                    match result {
                        Ok(results) => state.search.results = Some(results),
                        Err(error) => state.message = error,
                    }
                    Task::none()
                },
            );
        }
    }
    Task::none()
}

fn heading<'a>(label: &'a str) -> iced::widget::Text<'a> {
    text(label)
        .size(10)
        .font(design::UI_BOLD)
        .color(design::FAINT)
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let search = &state.search;
    let mode = |label: &'static str, content: bool| {
        button(text(label).size(11))
            .padding([4, 10])
            .style(styles::ghost(search.content == content))
            .on_press(Message::Search(Event::Mode(content)))
    };
    let header = column![
        container(heading("BUSCAR")).padding([0, 12]),
        container(
            text_input("Buscar en el proyecto…", &search.query)
                .id(INPUT)
                .size(12)
                .padding([6, 8])
                .style(styles::input)
                .on_input(|query| Message::Search(Event::Query(query)))
                .on_submit(Message::Search(Event::Run)),
        )
        .padding([0, 10]),
        container(row![mode("Contenido", true), mode("Nombres", false)].spacing(4))
            .padding([0, 10]),
    ]
    .spacing(8);
    container(
        column![
            header,
            container(summary(search)).padding([0, 12]),
            styles::vscroll(container(results(search)).padding([0, 6])).height(Fill),
        ]
        .spacing(8),
    )
    .padding([10, 0])
    .width(state.shell.explorer_width)
    .height(Fill)
    .style(styles::surface(design::BG_PANEL))
    .into()
}

fn summary(search: &State) -> Element<'_, Message> {
    let line = if search.running {
        "Buscando…".to_owned()
    } else if let Some(results) = &search.results {
        if results.hits.is_empty() {
            format!("Sin resultados para «{}»", search.searched)
        } else {
            let files = results
                .hits
                .iter()
                .map(|hit| hit.relative.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            let mut line = format!("{} coincidencias en {files} archivos", results.hits.len());
            if results.truncated {
                line.push_str(" · hay más; afina la búsqueda");
            }
            line
        }
    } else {
        "Escribe y pulsa Intro. Se omiten carpetas de dependencias y archivos .env.".to_owned()
    };
    text(line).size(11).color(design::MUTED).into()
}

fn results(search: &State) -> Element<'_, Message> {
    let Some(results) = &search.results else {
        return Space::new().into();
    };
    let mut list = column![].spacing(1);
    let mut last: Option<&str> = None;
    for hit in &results.hits {
        let open = |hit: &SearchHit| {
            Message::Editor(editor::Event::OpenAt(
                PathBuf::from(&hit.relative),
                hit.line.unwrap_or(0),
                hit.column,
                hit.length,
            ))
        };
        let path = Path::new(&hit.relative);
        if hit.line.is_none() {
            list = list.push(file_row(path, &hit.relative, open(hit)));
            continue;
        }
        if last != Some(hit.relative.as_str()) {
            list = list.push(file_row(path, &hit.relative, open(hit)));
            last = Some(hit.relative.as_str());
        }
        list = list.push(
            button(
                row![
                    text(hit.line.unwrap_or(0).to_string())
                        .size(10)
                        .font(design::MONO)
                        .color(design::FAINT)
                        .width(34)
                        .align_x(iced::alignment::Horizontal::Right),
                    text(&hit.preview)
                        .size(11)
                        .font(design::MONO)
                        .wrapping(iced::widget::text::Wrapping::None),
                ]
                .spacing(8),
            )
            .width(Fill)
            .clip(true)
            .padding([2, 8])
            .style(styles::ghost(false))
            .on_press(open(hit)),
        );
    }
    list.into()
}

fn file_row<'a>(path: &Path, label: &'a str, message: Message) -> Element<'a, Message> {
    let kind = file_types::of(path);
    button(
        row![
            icons::icon(kind.icon, kind.tint, 14.0),
            text(label).size(12).font(design::UI_BOLD)
        ]
        .spacing(7)
        .align_y(iced::Center),
    )
    .width(Fill)
    .padding([4, 6])
    .style(styles::ghost(false))
    .on_press(message)
    .into()
}
