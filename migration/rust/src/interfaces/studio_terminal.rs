use super::*;
use crate::integrations::terminal::Session;
use iced::widget::{column, text_input, tooltip};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(super) struct State {
    pub session: Option<Arc<Mutex<Session>>>,
    pub opening: bool,
    pub visible: bool,
    generation: u64,
    input: String,
    output: String,
    rows: u16,
    cols: u16,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Open,
    Opened(u64, Result<Arc<Mutex<Session>>, String>),
    Input(String),
    Send,
    Sent(Result<(), String>),
    Tick,
    Interrupt,
    Close,
    Resize(u16, u16),
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    let terminal = &mut state.terminal;
    match event {
        Event::Open => {
            terminal.visible = true;
            if terminal.opening || terminal.session.is_some() {
                return Task::none();
            }
            terminal.opening = true;
            terminal.generation += 1;
            let generation = terminal.generation;
            let root = state.root.clone();
            return Task::perform(
                async move {
                    jobs::offload(move || {
                        crate::features::workspace_terminal::open(&root)
                            .map(|session| Arc::new(Mutex::new(session)))
                    })
                    .await
                    .unwrap_or_else(|| Err("No se pudo iniciar la terminal".into()))
                },
                move |result| Message::Terminal(Event::Opened(generation, result)),
            );
        }
        Event::Opened(generation, result) => {
            if generation != terminal.generation {
                return Task::none();
            }
            terminal.opening = false;
            match result {
                Ok(session) => {
                    terminal.session = Some(session);
                    terminal.rows = 24;
                    terminal.cols = 100;
                }
                Err(error) => state.message = error,
            }
        }
        Event::Input(input) => terminal.input = input,
        Event::Send | Event::Interrupt => {
            if let Some(session) = terminal.session.clone() {
                let input = if matches!(event, Event::Interrupt) {
                    "\x03".into()
                } else {
                    format!("{}\r", std::mem::take(&mut terminal.input))
                };
                return Task::perform(
                    async move {
                        jobs::offload(move || {
                            session
                                .lock()
                                .map_err(|_| "La terminal no responde".to_owned())?
                                .send(input.as_bytes())
                        })
                        .await
                        .unwrap_or_else(|| Err("Falló el envío a la terminal".into()))
                    },
                    |result| Message::Terminal(Event::Sent(result)),
                );
            }
        }
        Event::Sent(result) => {
            if let Err(error) = result {
                state.message = error;
            }
        }
        Event::Tick => {
            if let Some(session) = &terminal.session
                && let Ok(mut session) = session.try_lock()
            {
                match session.poll_changed() {
                    Ok(Some(output)) => terminal.output = output,
                    Ok(None) => {}
                    Err(error) => state.message = error,
                }
            }
        }
        Event::Close => {
            terminal.generation += 1;
            terminal.opening = false;
            terminal.session = None;
            terminal.output.clear();
        }
        Event::Resize(rows, cols) => {
            terminal.rows = rows;
            terminal.cols = cols;
            if let Some(session) = terminal.session.clone() {
                return Task::perform(
                    async move {
                        jobs::offload(move || {
                            session
                                .lock()
                                .map_err(|_| "La terminal no responde".to_owned())?
                                .resize(rows, cols)
                        })
                        .await
                        .unwrap_or_else(|| Err("Falló el cambio de tamaño de terminal".into()))
                    },
                    |result| Message::Terminal(Event::Sent(result)),
                );
            }
        }
    }
    Task::none()
}

/// Session controls shown in the dock header while the terminal tab is active.
pub(super) fn toolbar(state: &Dashboard) -> Element<'_, Message> {
    if state.terminal.session.is_none() {
        return Space::new().into();
    }
    let action = |icon: &'static str, label: &'static str, event: Event| {
        styles::tip(
            button(icons::icon(icon, design::MUTED, 13.0))
                .padding(5)
                .style(styles::ghost(false))
                .on_press(Message::Terminal(event)),
            label,
            tooltip::Position::Top,
        )
    };
    let columns = |label: &'static str, cols: u16| {
        let active = state.terminal.cols == cols;
        button(text(label).size(10).font(design::MONO))
            .padding([4, 7])
            .style(styles::ghost(active))
            .on_press(Message::Terminal(Event::Resize(
                state.terminal.rows.max(24),
                cols,
            )))
    };
    row![
        columns("80", 80),
        columns("100", 100),
        columns("120", 120),
        action("stop", "Interrumpir (Ctrl+C)", Event::Interrupt),
        action("close", "Terminar sesión", Event::Close),
    ]
    .spacing(2)
    .align_y(iced::Center)
    .into()
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let terminal = &state.terminal;
    if terminal.session.is_none() {
        let hint = if terminal.opening {
            "Iniciando shell…"
        } else {
            "Abre un shell del proyecto. Se requiere confianza local."
        };
        let mut empty = column![
            icons::icon("terminal", design::FAINT, 28.0),
            text(hint).size(12).color(design::MUTED),
        ]
        .spacing(10)
        .align_x(iced::Center);
        if !terminal.opening {
            empty = empty.push(
                button(text("Abrir terminal").size(12))
                    .padding([7, 14])
                    .style(styles::primary)
                    .on_press(Message::Terminal(Event::Open)),
            );
        }
        return container(empty).center(Fill).into();
    }
    let output = container(
        styles::vscroll(
            container(text(&terminal.output).font(design::MONO).size(12))
                .padding([8, 12])
                .width(Fill),
        )
        .anchor_bottom()
        .height(Fill),
    )
    .height(Fill)
    .style(styles::well);
    let prompt = row![
        icons::icon("chevron-right", design::ACCENT, 13.0),
        text_input("Enviar una línea al shell…", &terminal.input)
            .size(12)
            .font(design::MONO)
            .padding([5, 8])
            .style(styles::input)
            .on_input(|line| Message::Terminal(Event::Input(line)))
            .on_submit(Message::Terminal(Event::Send)),
    ]
    .spacing(6)
    .align_y(iced::Center);
    container(column![output, prompt].spacing(6))
        .padding(iced::Padding {
            top: 0.0,
            right: 10.0,
            bottom: 8.0,
            left: 10.0,
        })
        .width(Fill)
        .height(Fill)
        .into()
}
