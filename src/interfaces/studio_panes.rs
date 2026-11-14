//! The work area as a grid of groups the person arranges. Each group holds tabs (the editor, chats,
//! terminals, the activity log) like the editor groups of VS Code: several things share one place as
//! tabs, and a tab can be moved out to its own pane. Panes can be dragged next to each other (edges
//! split, the middle merges as tabs), resized and maximized.
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

/// The tabs of one pane and which of them shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Group {
    pub tabs: Vec<Content>,
    pub active: usize,
}

impl Group {
    fn single(content: Content) -> Self {
        Self {
            tabs: vec![content],
            active: 0,
        }
    }

    pub fn current(&self) -> Option<&Content> {
        self.tabs.get(self.active)
    }
}

pub(super) struct State {
    pub grid: pane_grid::State<Group>,
    pub focus: Option<Pane>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            grid: pane_grid::State::new(Group::single(Content::Editor)).0,
            focus: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Focus(Pane),
    /// Shows this tab of the group.
    Select(Pane, usize),
    /// Closes a tab (a terminal tab ends its session).
    CloseTab(Pane, usize),
    Maximize(Pane),
    Restore,
    /// A new shell in a new pane next to this one.
    Split(Pane, Axis),
    /// Moves the active tab of a group with several into a pane of its own.
    MoveOut(Pane, Axis),
}

impl State {
    fn locate(&self, wanted: impl Fn(&Content) -> bool) -> Option<(Pane, usize)> {
        self.grid.iter().find_map(|(pane, group)| {
            group
                .tabs
                .iter()
                .position(&wanted)
                .map(|index| (*pane, index))
        })
    }

    pub fn find(&self, wanted: impl Fn(&Content) -> bool) -> Option<Pane> {
        self.locate(wanted).map(|(pane, _)| pane)
    }

    /// Terminal tabs on screen, hidden or not.
    pub fn terminal_panes(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|(_, group)| &group.tabs)
            .filter(|c| matches!(c, Content::Terminal(_)))
            .count()
    }

    pub fn has_activity(&self) -> bool {
        self.locate(|c| *c == Content::Activity).is_some()
    }

    /// Whether this chat is the tab being shown (a hidden tab accepts no dropped files).
    pub fn shows_chat(&self, id: &str) -> bool {
        self.grid
            .iter()
            .any(|(_, group)| matches!(group.current(), Some(Content::Chat(agent)) if agent == id))
    }

    fn editor_or_first(&self) -> Pane {
        self.find(|c| *c == Content::Editor)
            .or_else(|| self.grid.iter().next().map(|(pane, _)| *pane))
            .expect("the grid is never empty")
    }

    fn split(&mut self, anchor: Pane, axis: Axis, group: Group, ratio: f32) -> Pane {
        match self.grid.split(axis, anchor, group) {
            Some((pane, split)) => {
                self.grid.resize(split, ratio);
                self.focus = Some(pane);
                pane
            }
            None => anchor,
        }
    }

    fn add_tab(&mut self, pane: Pane, content: Content) {
        if let Some(group) = self.grid.get_mut(pane) {
            group.tabs.push(content);
            group.active = group.tabs.len() - 1;
        }
        self.focus = Some(pane);
    }

    /// Shows `content`: its tab when it already has one, otherwise a new tab. Terminals gather in
    /// one group under the editor; everything else opens in the group the person is working in.
    pub fn show(&mut self, content: Content) -> Pane {
        if let Some((pane, index)) = self.locate(|c| *c == content) {
            if let Some(group) = self.grid.get_mut(pane) {
                group.active = index;
            }
            self.focus = Some(pane);
            return pane;
        }
        if matches!(content, Content::Terminal(_)) {
            let terminals = self.find(|c| matches!(c, Content::Terminal(_)));
            return match terminals {
                Some(pane) => {
                    self.add_tab(pane, content);
                    pane
                }
                None => {
                    let anchor = self.editor_or_first();
                    self.split(anchor, Axis::Horizontal, Group::single(content), 0.68)
                }
            };
        }
        let pane = self
            .focus
            .filter(|pane| self.grid.get(*pane).is_some())
            .unwrap_or_else(|| self.editor_or_first());
        self.add_tab(pane, content);
        pane
    }

    /// A terminal that just opened takes the placeholder tab, or a new one.
    pub fn attach_terminal(&mut self, id: u64) {
        if let Some((pane, index)) = self.locate(|c| *c == Content::Terminal(None))
            && let Some(group) = self.grid.get_mut(pane)
        {
            group.tabs[index] = Content::Terminal(Some(id));
            group.active = index;
            self.focus = Some(pane);
            return;
        }
        self.show(Content::Terminal(Some(id)));
    }

    fn drop_empty(&mut self, pane: Pane) {
        let empty = self.grid.get(pane).is_some_and(|g| g.tabs.is_empty());
        if empty && self.grid.len() > 1 {
            self.grid.close(pane);
        }
        if self.focus.is_some_and(|p| self.grid.get(p).is_none()) {
            self.focus = None;
        }
    }

    fn remove_tab(&mut self, pane: Pane, index: usize) {
        if let Some(group) = self.grid.get_mut(pane)
            && index < group.tabs.len()
        {
            group.tabs.remove(index);
            if group.active > index || group.active >= group.tabs.len() {
                group.active = group
                    .active
                    .saturating_sub(1)
                    .min(group.tabs.len().saturating_sub(1));
            }
        }
        self.drop_empty(pane);
    }

    /// Removes the tabs matching `wanted` (the editor stays).
    pub fn hide(&mut self, wanted: impl Fn(&Content) -> bool) {
        let panes: Vec<Pane> = self.grid.iter().map(|(pane, _)| *pane).collect();
        for pane in panes {
            while let Some(index) = self.grid.get(pane).and_then(|g| {
                g.tabs
                    .iter()
                    .position(|c| *c != Content::Editor && wanted(c))
            }) {
                self.remove_tab(pane, index);
                if self.grid.get(pane).is_none() {
                    break;
                }
            }
        }
    }

    /// The active tab of a group with several goes to a pane of its own.
    pub fn move_out(&mut self, pane: Pane, axis: Axis) {
        let Some(group) = self.grid.get_mut(pane) else {
            return;
        };
        if group.tabs.len() < 2 {
            return;
        }
        let content = group.tabs.remove(group.active);
        group.active = group.active.min(group.tabs.len() - 1);
        self.split(pane, axis, Group::single(content), 0.5);
    }

    /// All the tabs of `from` join `into`.
    fn merge(&mut self, from: Pane, into: Pane) {
        if from == into {
            return;
        }
        let Some(moved) = self.grid.get_mut(from).map(|g| std::mem::take(&mut g.tabs)) else {
            return;
        };
        if let Some(target) = self.grid.get_mut(into) {
            target.active = target.tabs.len();
            target.tabs.extend(moved);
        }
        self.grid.close(from);
        self.focus = Some(into);
    }
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => match target {
            // The middle of another pane: the dragged one becomes tabs there (like dropping a tab on a group).
            pane_grid::Target::Pane(other, pane_grid::Region::Center) => {
                state.panes.merge(pane, other)
            }
            target => {
                state.panes.grid.drop(pane, target);
                state.panes.focus = Some(pane);
            }
        },
        Event::Dragged(_) => {}
        Event::Resized(resize) => state.panes.grid.resize(resize.split, resize.ratio),
        Event::Focus(pane) => return focus(state, pane),
        Event::Select(pane, index) => {
            if let Some(group) = state.panes.grid.get_mut(pane)
                && index < group.tabs.len()
            {
                group.active = index;
            }
            return focus(state, pane);
        }
        Event::CloseTab(pane, index) => return close_tab(state, pane, index),
        Event::Maximize(pane) => {
            state.panes.grid.maximize(pane);
            state.panes.focus = Some(pane);
        }
        Event::Restore => state.panes.grid.restore(),
        Event::Split(pane, axis) => {
            if state.panes.grid.get(pane).is_some() {
                state
                    .panes
                    .split(pane, axis, Group::single(Content::Terminal(None)), 0.5);
                return terminal::update(state, terminal::Event::Open);
            }
        }
        Event::MoveOut(pane, axis) => state.panes.move_out(pane, axis),
    }
    Task::none()
}

fn focus(state: &mut Dashboard, pane: Pane) -> Task<Message> {
    state.panes.focus = Some(pane);
    match state
        .panes
        .grid
        .get(pane)
        .and_then(|g| g.current())
        .cloned()
    {
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

fn close_tab(state: &mut Dashboard, pane: Pane, index: usize) -> Task<Message> {
    match state
        .panes
        .grid
        .get(pane)
        .and_then(|g| g.tabs.get(index))
        .cloned()
    {
        Some(Content::Terminal(Some(id))) => terminal::update(state, terminal::Event::CloseTab(id)),
        Some(Content::Editor) | None => Task::none(),
        Some(_) => {
            state.panes.remove_tab(pane, index);
            Task::none()
        }
    }
}

/// Opens a chat as a tab of the group in use.
pub(super) fn show_chat(state: &mut Dashboard, id: &str) {
    state.panes.show(Content::Chat(id.to_owned()));
}

#[cfg(test)]
#[path = "studio_panes_tests.rs"]
mod tests;
