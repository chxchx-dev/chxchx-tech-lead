//! The work area as a grid of panes the person arranges: the editor, chats, terminals and the
//! activity log can sit side by side or stacked, be dragged next to each other, resized and maximized.
use super::*;
use iced::widget::pane_grid::{self, Axis, Pane};

#[path = "studio_panes_view.rs"]
mod view_impl;
pub(in crate::interfaces::studio) use view_impl::view;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Content {
    Editor,
    /// The chat with the agent of this id.
    Chat(String),
    /// A terminal session by tab id; `None` is the placeholder before one exists.
    Terminal(Option<u64>),
    Activity,
}

pub(super) struct State {
    pub grid: pane_grid::State<Content>,
    pub focus: Option<Pane>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            grid: pane_grid::State::new(Content::Editor).0,
            focus: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Focus(Pane),
    Close(Pane),
    Maximize(Pane),
    Restore,
    /// A new shell next to this pane, splitting along the axis.
    Split(Pane, Axis),
}

impl State {
    pub fn find(&self, wanted: impl Fn(&Content) -> bool) -> Option<Pane> {
        self.grid
            .iter()
            .find(|(_, content)| wanted(content))
            .map(|(pane, _)| *pane)
    }

    pub fn terminal_panes(&self) -> usize {
        self.grid
            .iter()
            .filter(|(_, c)| matches!(c, Content::Terminal(_)))
            .count()
    }

    pub fn has_activity(&self) -> bool {
        self.find(|c| *c == Content::Activity).is_some()
    }

    pub fn has_chat(&self, id: &str) -> bool {
        self.find(|c| matches!(c, Content::Chat(agent) if agent == id))
            .is_some()
    }

    fn split(&mut self, anchor: Pane, axis: Axis, content: Content, ratio: f32) -> Pane {
        match self.grid.split(axis, anchor, content.clone()) {
            Some((pane, split)) => {
                self.grid.resize(split, ratio);
                self.focus = Some(pane);
                pane
            }
            None => anchor,
        }
    }

    /// Shows `content`, reusing its pane when it already has one.
    pub fn show(&mut self, content: Content) -> Pane {
        if let Some(existing) = self.find(|c| *c == content) {
            self.focus = Some(existing);
            return existing;
        }
        let editor = self.find(|c| *c == Content::Editor);
        let last_terminal = self
            .grid
            .iter()
            .filter(|(_, c)| matches!(c, Content::Terminal(_)))
            .map(|(pane, _)| *pane)
            .last();
        let first = self.grid.iter().next().map(|(pane, _)| *pane);
        match content {
            Content::Chat(_) | Content::Editor => {
                let anchor = editor.or(first).expect("the grid is never empty");
                self.split(anchor, Axis::Vertical, content, 0.52)
            }
            Content::Terminal(_) | Content::Activity => match last_terminal {
                Some(anchor) if matches!(content, Content::Terminal(_)) => {
                    self.split(anchor, Axis::Vertical, content, 0.5)
                }
                _ => {
                    let anchor = editor.or(first).expect("the grid is never empty");
                    self.split(anchor, Axis::Horizontal, content, 0.68)
                }
            },
        }
    }

    /// A terminal that just opened takes the placeholder pane, or a new one.
    pub fn attach_terminal(&mut self, id: u64) {
        if let Some(placeholder) = self.find(|c| *c == Content::Terminal(None))
            && let Some(content) = self.grid.get_mut(placeholder)
        {
            *content = Content::Terminal(Some(id));
            self.focus = Some(placeholder);
            return;
        }
        self.show(Content::Terminal(Some(id)));
    }

    /// Removes the pane of this content (the editor pane always stays).
    pub fn hide(&mut self, wanted: impl Fn(&Content) -> bool) {
        let panes: Vec<Pane> = self
            .grid
            .iter()
            .filter(|(_, c)| **c != Content::Editor && wanted(c))
            .map(|(pane, _)| *pane)
            .collect();
        for pane in panes {
            self.grid.close(pane);
        }
        if self.focus.is_some_and(|p| self.grid.get(p).is_none()) {
            self.focus = None;
        }
    }
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
            state.panes.grid.drop(pane, target);
            state.panes.focus = Some(pane);
        }
        Event::Dragged(_) => {}
        Event::Resized(resize) => state.panes.grid.resize(resize.split, resize.ratio),
        Event::Focus(pane) => return focus(state, pane),
        Event::Close(pane) => return close(state, pane),
        Event::Maximize(pane) => {
            state.panes.grid.maximize(pane);
            state.panes.focus = Some(pane);
        }
        Event::Restore => state.panes.grid.restore(),
        Event::Split(pane, axis) => {
            let anchor = state.panes.grid.get(pane).cloned();
            if anchor.is_some() {
                state.panes.focus = Some(pane);
                let target = state.panes.split(pane, axis, Content::Terminal(None), 0.5);
                let _ = target;
                return terminal::update(state, terminal::Event::Open);
            }
        }
    }
    Task::none()
}

fn focus(state: &mut Dashboard, pane: Pane) -> Task<Message> {
    state.panes.focus = Some(pane);
    match state.panes.grid.get(pane).cloned() {
        Some(Content::Chat(id)) => {
            if let Some(index) = state.chat.chats.iter().position(|c| c.agent.id == id) {
                state.chat.active = index;
            }
            Task::none()
        }
        Some(Content::Terminal(Some(id))) => terminal::focus_tab(state, id),
        _ => Task::none(),
    }
}

fn close(state: &mut Dashboard, pane: Pane) -> Task<Message> {
    match state.panes.grid.get(pane).cloned() {
        Some(Content::Terminal(Some(id))) => terminal::update(state, terminal::Event::CloseTab(id)),
        Some(Content::Editor) | None => Task::none(),
        Some(_) => {
            state.panes.grid.close(pane);
            if state.panes.focus == Some(pane) {
                state.panes.focus = None;
            }
            Task::none()
        }
    }
}

/// Opens a chat beside the editor.
pub(super) fn show_chat(state: &mut Dashboard, id: &str) {
    state.panes.show(Content::Chat(id.to_owned()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chat_opens_beside_the_editor_and_a_second_request_reuses_its_pane() {
        let mut state = State::default();
        let first = state.show(Content::Chat("claude".into()));
        assert_eq!(state.grid.len(), 2);
        assert_eq!(state.show(Content::Chat("claude".into())), first);
        assert_eq!(state.grid.len(), 2);
        assert!(state.has_chat("claude") && !state.has_chat("codex"));
    }

    #[test]
    fn terminals_go_below_the_editor_and_then_side_by_side() {
        let mut state = State::default();
        state.show(Content::Terminal(None));
        state.attach_terminal(1);
        assert_eq!(state.grid.len(), 2, "the placeholder became the session");
        state.attach_terminal(2);
        assert_eq!((state.grid.len(), state.terminal_panes()), (3, 2));
        state.show(Content::Activity);
        assert!(state.has_activity());
    }

    #[test]
    fn hiding_removes_panes_but_never_the_editor() {
        let mut state = State::default();
        state.attach_terminal(1);
        state.show(Content::Chat("codex".into()));
        state.hide(|_| true);
        assert_eq!(state.grid.len(), 1);
        assert!(state.find(|c| *c == Content::Editor).is_some());
        assert_eq!(state.focus, None);
    }
}
