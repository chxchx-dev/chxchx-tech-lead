//! Terminal sessions of the dock: shells and agents, each in its own tab.
use super::*;
use crate::features::agents::Agent;
use crate::integrations::terminal::{Session, Snapshot};
use iced::keyboard;
use std::sync::{Arc, Mutex};

#[path = "studio_terminal_view.rs"]
mod view_impl;
pub(in crate::interfaces::studio) use view_impl::view;

/// Identifies the surface of a session for focus operations.
pub(super) fn surface_id(id: u64) -> iced::widget::Id {
    iced::widget::Id::from(format!("terminal-{id}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Shell,
    /// The id of the agent running in the tab.
    Agent(String),
}

pub(super) struct Tab {
    pub id: u64,
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
    pub focused: bool,
    generation: u64,
    next_id: u64,
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
    /// Like `OpenAgent`, with its own tab title (an agent started with a local model).
    OpenAgentAs(Agent, String),
    Opened(u64, Kind, String, Result<Arc<Mutex<Session>>, String>),
    CloseTab(u64),
    Key(u64, keyboard::Event),
    Pasted(Option<String>),
    Scroll(u64, i32),
    Focus(u64, bool),
    Resized(u64, iced::Size),
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
        Event::Open => {
            let title = crate::features::workspace_terminal::shell_name();
            return open(state, Kind::Shell, title, None);
        }
        Event::OpenAgent(agent) => {
            let title = agent.label();
            return update(state, Event::OpenAgentAs(agent, title));
        }
        Event::OpenAgentAs(agent, title) => {
            if let Some(index) = state.terminal.agent_tab(&agent.id) {
                let id = state.terminal.tabs[index].id;
                state.terminal.active = index;
                state.panes.show(panes::Content::Terminal(Some(id)));
                return focus_surface(id);
            }
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
                    let id = terminal.next_id;
                    terminal.next_id += 1;
                    terminal.tabs.push(Tab {
                        id,
                        kind,
                        title,
                        session,
                        snapshot,
                        exited: false,
                    });
                    terminal.active = terminal.tabs.len() - 1;
                    terminal.focused = true;
                    state.panes.attach_terminal(id);
                    return focus_surface(id);
                }
                Err(error) => state.message = error,
            }
        }
        Event::CloseTab(id) => {
            let terminal = &mut state.terminal;
            if let Some(index) = terminal.tabs.iter().position(|tab| tab.id == id) {
                terminal.tabs.remove(index);
                terminal.active = terminal.active.min(terminal.tabs.len().saturating_sub(1));
                if index < terminal.active {
                    terminal.active -= 1;
                }
                terminal.generation += 1;
                terminal.opening = false;
                state
                    .panes
                    .hide(|c| *c == panes::Content::Terminal(Some(id)));
                crate::integrations::process_memory::give_back_soon();
            }
        }
        Event::Key(id, key) => {
            select(&mut state.terminal, id);
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
        Event::Scroll(id, lines) => {
            if let Some(tab) = state.terminal.tabs.iter_mut().find(|tab| tab.id == id) {
                if let Ok(mut session) = tab.session.lock() {
                    session.scroll(lines);
                }
                refresh(tab);
            }
        }
        Event::Focus(id, focused) => {
            if focused {
                select(&mut state.terminal, id);
            } else if state.terminal.active_tab().is_some_and(|tab| tab.id == id) {
                state.terminal.focused = false;
            }
        }
        Event::Resized(id, size) => resize(&mut state.terminal, id, size),
        Event::Tick => tick(&mut state.terminal),
        Event::Interrupt => send(state, b"\x03"),
    }
    Task::none()
}

fn focus_surface(id: u64) -> Task<Message> {
    iced::widget::operation::focus(surface_id(id))
}

/// The session the keyboard goes to: the one whose surface has the focus.
fn select(terminal: &mut State, id: u64) {
    if let Some(index) = terminal.tabs.iter().position(|tab| tab.id == id) {
        terminal.active = index;
        terminal.focused = true;
    }
}

/// A click on the pane of a session: its surface takes the keyboard.
pub(super) fn focus_tab(state: &mut Dashboard, id: u64) -> Task<Message> {
    select(&mut state.terminal, id);
    focus_surface(id)
}

fn open(state: &mut Dashboard, kind: Kind, title: String, agent: Option<Agent>) -> Task<Message> {
    state.panes.show(panes::Content::Terminal(None));
    let terminal = &mut state.terminal;
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
        let changed = matches!(session.poll_dirty(), Ok(true));
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

fn resize(terminal: &mut State, id: u64, size: iced::Size) {
    let (rows, cols) = grid_for(size);
    (terminal.rows, terminal.cols) = (rows, cols);
    if let Some(tab) = terminal.tabs.iter_mut().find(|tab| tab.id == id)
        && let Ok(mut session) = tab.session.lock()
    {
        let _ = session.resize(rows, cols);
        tab.snapshot = session.snapshot();
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
