//! Terminal sessions of the dock: shells and agents, each in its own tab.
use super::*;
use crate::features::agents::Agent;
use crate::integrations::terminal::{Session, Snapshot};
use iced::keyboard;
use std::sync::{Arc, Mutex};

#[path = "studio_terminal_view.rs"]
mod view_impl;
pub(in crate::interfaces::studio) use view_impl::{toolbar, view};

/// Identifies the terminal surface for focus operations.
pub(super) const SURFACE: &str = "terminal-surface";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Shell,
    /// The id of the agent running in the tab.
    Agent(String),
}

pub(super) struct Tab {
    pub kind: Kind,
    pub title: String,
    pub session: Arc<Mutex<Session>>,
    pub snapshot: Snapshot,
    pub exited: bool,
}

#[derive(Default)]
pub(super) struct State {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub opening: bool,
    pub visible: bool,
    pub focused: bool,
    generation: u64,
    rows: u16,
    cols: u16,
}

impl State {
    pub fn has_sessions(&self) -> bool {
        !self.tabs.is_empty()
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    /// The tab an agent is already running in, if any.
    pub fn agent_tab(&self, id: &str) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| matches!(&tab.kind, Kind::Agent(agent) if agent == id) && !tab.exited)
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    /// A new shell tab.
    Open,
    /// An agent in a tab of its own (or back to the one it already has).
    OpenAgent(Agent),
    Opened(u64, Kind, String, Result<Arc<Mutex<Session>>, String>),
    Select(usize),
    CloseTab(usize),
    Key(keyboard::Event),
    Pasted(Option<String>),
    Scroll(i32),
    Focus(bool),
    Resized(iced::Size),
    Tick,
    Interrupt,
}

/// Font and cell metrics: the grid is sized from them, so the program gets the rows and columns that fit.
pub(super) const FONT_SIZE: f32 = 13.0;
pub(super) const LINE_HEIGHT: f32 = 17.0;
#[cfg(windows)]
pub(super) const CELL_WIDTH: f32 = FONT_SIZE * 0.55;
#[cfg(not(windows))]
pub(super) const CELL_WIDTH: f32 = FONT_SIZE * 0.602;
pub(super) const PADDING: f32 = 8.0;

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Open => return open(state, Kind::Shell, "Shell".to_owned(), None),
        Event::OpenAgent(agent) => {
            if let Some(index) = state.terminal.agent_tab(&agent.id) {
                state.terminal.visible = true;
                state.terminal.active = index;
                return focus_surface();
            }
            let title = agent.label();
            return open(state, Kind::Agent(agent.id.clone()), title, Some(agent));
        }
        Event::Opened(generation, kind, title, result) => {
            let terminal = &mut state.terminal;
            if generation != terminal.generation {
                return Task::none();
            }
            terminal.opening = false;
            match result {
                Ok(session) => {
                    let (rows, cols) = (terminal.rows.max(24), terminal.cols.max(80));
                    if let Ok(mut locked) = session.lock() {
                        let _ = locked.resize(rows, cols);
                    }
                    let title = unique_title(&terminal.tabs, title);
                    let snapshot = session.lock().map(|s| s.snapshot()).unwrap_or_default();
                    terminal.tabs.push(Tab {
                        kind,
                        title,
                        session,
                        snapshot,
                        exited: false,
                    });
                    terminal.active = terminal.tabs.len() - 1;
                    terminal.focused = true;
                    return focus_surface();
                }
                Err(error) => state.message = error,
            }
        }
        Event::Select(index) => {
            let terminal = &mut state.terminal;
            if index < terminal.tabs.len() {
                terminal.active = index;
                refresh(&mut terminal.tabs[index]);
                return focus_surface();
            }
        }
        Event::CloseTab(index) => {
            let terminal = &mut state.terminal;
            if index < terminal.tabs.len() {
                terminal.tabs.remove(index);
                terminal.active = terminal.active.min(terminal.tabs.len().saturating_sub(1));
                if index < terminal.active {
                    terminal.active -= 1;
                }
                terminal.generation += 1;
                terminal.opening = false;
            }
        }
        Event::Key(key) => {
            if let keyboard::Event::KeyPressed { key, modifiers, .. } = &key
                && terminal_keys::is_paste(key, *modifiers)
            {
                return paste_request();
            }
            forward_key(state, &key);
        }
        Event::Pasted(text) => {
            if let (Some(text), Some(tab)) = (text, state.terminal.active_tab()) {
                let mut bytes = text.replace("\r\n", "\n").replace('\n', "\r").into_bytes();
                if tab.snapshot.bracketed_paste {
                    bytes = [b"\x1b[200~".as_slice(), &bytes, b"\x1b[201~"].concat();
                }
                send(state, &bytes);
            }
        }
        Event::Scroll(lines) => {
            if let Some(tab) = state.terminal.tabs.get_mut(state.terminal.active) {
                if let Ok(mut session) = tab.session.lock() {
                    session.scroll(lines);
                }
                refresh(tab);
            }
        }
        Event::Focus(focused) => state.terminal.focused = focused,
        Event::Resized(size) => resize(&mut state.terminal, size),
        Event::Tick => tick(&mut state.terminal),
        Event::Interrupt => send(state, b"\x03"),
    }
    Task::none()
}

fn focus_surface() -> Task<Message> {
    iced::widget::operation::focus(SURFACE)
}

fn open(state: &mut Dashboard, kind: Kind, title: String, agent: Option<Agent>) -> Task<Message> {
    let terminal = &mut state.terminal;
    terminal.visible = true;
    if terminal.opening {
        return Task::none();
    }
    terminal.opening = true;
    terminal.generation += 1;
    let generation = terminal.generation;
    let root = state.root.clone();
    let reported = (kind.clone(), title);
    Task::perform(
        async move {
            jobs::offload(move || {
                match &agent {
                    Some(agent) => crate::features::workspace_terminal::open_agent(&root, agent),
                    None => crate::features::workspace_terminal::open(&root),
                }
                .map(|session| Arc::new(Mutex::new(session)))
            })
            .await
            .unwrap_or_else(|| Err("No se pudo iniciar la terminal".into()))
        },
        move |result| Message::Terminal(Event::Opened(generation, reported.0, reported.1, result)),
    )
}

/// "claude", then "claude 2" when there is already one.
fn unique_title(tabs: &[Tab], title: String) -> String {
    let taken = |candidate: &str| tabs.iter().any(|tab| tab.title == candidate);
    if !taken(&title) {
        return title;
    }
    (2..)
        .map(|n| format!("{title} {n}"))
        .find(|c| !taken(c))
        .unwrap_or(title)
}

fn refresh(tab: &mut Tab) {
    if let Ok(session) = tab.session.lock() {
        tab.snapshot = session.snapshot();
    }
}

fn tick(terminal: &mut State) {
    let active = terminal.active;
    for (index, tab) in terminal.tabs.iter_mut().enumerate() {
        let Ok(mut session) = tab.session.try_lock() else {
            continue;
        };
        // Every session is read so none of them stalls; only the visible one is redrawn.
        let changed = matches!(session.poll_changed(), Ok(Some(_)));
        tab.exited = session.exited;
        if changed && index == active {
            tab.snapshot = session.snapshot();
        }
    }
}

fn send(state: &mut Dashboard, bytes: &[u8]) {
    let Some(tab) = state.terminal.active_tab() else {
        return;
    };
    if tab.exited {
        return;
    }
    let result = match tab.session.lock() {
        Ok(mut session) => session.send(bytes),
        Err(_) => Err("La terminal no responde".to_owned()),
    };
    if let Err(error) = result {
        state.message = error;
    }
}

fn forward_key(state: &mut Dashboard, event: &keyboard::Event) {
    let keyboard::Event::KeyPressed {
        key,
        modifiers,
        text,
        ..
    } = event
    else {
        return;
    };
    let application_cursor = state
        .terminal
        .active_tab()
        .is_some_and(|tab| tab.snapshot.application_cursor);
    if let Some(bytes) = terminal_keys::encode(key, *modifiers, text.as_deref(), application_cursor)
    {
        send(state, &bytes);
    }
}

/// Rows and columns that fit a surface of this size.
pub(super) fn grid_for(size: iced::Size) -> (u16, u16) {
    let cols = ((size.width - 2.0 * PADDING) / CELL_WIDTH).floor();
    let rows = ((size.height - 2.0 * PADDING) / LINE_HEIGHT).floor();
    (
        rows.clamp(4.0, 100.0) as u16,
        cols.clamp(20.0, 240.0) as u16,
    )
}

fn resize(terminal: &mut State, size: iced::Size) {
    let (rows, cols) = grid_for(size);
    if (rows, cols) == (terminal.rows, terminal.cols) {
        return;
    }
    terminal.rows = rows;
    terminal.cols = cols;
    for tab in &mut terminal.tabs {
        if let Ok(mut session) = tab.session.lock() {
            let _ = session.resize(rows, cols);
            tab.snapshot = session.snapshot();
        }
    }
}

/// The key press that asks to paste: Studio reads the clipboard and sends it as typed.
pub(super) fn paste_request() -> Task<Message> {
    iced::clipboard::read().map(|text| Message::Terminal(Event::Pasted(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_follows_the_size_and_stays_within_sane_limits() {
        let (rows, cols) = grid_for(iced::Size::new(800.0, 300.0));
        assert!(cols > 90 && cols < 110, "{cols}");
        assert!(rows > 14 && rows < 20, "{rows}");
        assert_eq!(grid_for(iced::Size::new(10.0, 10.0)), (4, 20));
        assert_eq!(grid_for(iced::Size::new(9000.0, 9000.0)), (100, 240));
    }
}
