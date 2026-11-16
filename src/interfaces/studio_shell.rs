//! Window layout state: resizable panels, bottom dock and global shortcuts.
use super::keymap::{self, Action};
use super::*;
use iced::widget::mouse_area;
use iced::{Point, Subscription, keyboard, mouse};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Split {
    Explorer,
}

/// What the left sidebar shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Explorer,
    Search,
    Git,
    Agents,
    Workspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DockTab {
    Terminal,
    Activity,
}

pub(super) struct State {
    pub explorer_width: f32,
    pub dock_height: f32,
    pub side: Side,
    drag: Option<(Split, Option<Point>)>,
    last_dock: DockTab,
    /// Resident memory of Studio, read every few seconds.
    pub memory: Option<u64>,
    memory_read: Option<std::time::Instant>,
    memory_trimmed: std::time::Instant,
    /// The window has the focus: timers that only keep the screen fresh pause while it does not.
    pub focused: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            explorer_width: 248.0,
            dock_height: 220.0,
            side: Side::Explorer,
            drag: None,
            last_dock: DockTab::Terminal,
            memory: None,
            memory_read: None,
            memory_trimmed: std::time::Instant::now(),
            focused: true,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    DragStart(Split),
    DragMove(Point),
    DragEnd,
    Dock(DockTab),
    ToggleDock,
    /// Rail button: shows the tab, or hides the dock when it is already the visible one.
    ToggleTab(DockTab),
    /// Shows a sidebar view and focuses its input.
    ShowSide(Side),
    /// Rail button: like `ShowSide`, but hides the sidebar when it is already shown.
    ToggleSide(Side),
    ResetLayout,
    About,
    ClearMessage,
    WindowFocus(bool),
}

/// Whether the panes of this dock tab are on screen.
pub(super) fn dock_visible(state: &Dashboard, tab: DockTab) -> bool {
    match tab {
        DockTab::Terminal => state.panes.terminal_panes() > 0,
        DockTab::Activity => state.panes.has_activity(),
    }
}

/// The dock tab on screen (terminals first), or `None` when there is none.
pub(super) fn dock_tab(state: &Dashboard) -> Option<DockTab> {
    [DockTab::Terminal, DockTab::Activity]
        .into_iter()
        .find(|tab| dock_visible(state, *tab))
}

/// Memory is read at most every 5 s and freed memory is handed back at most every 30 s.
/// Both ride on messages that already happen: an idle Studio has no timer for them.
const MEMORY_READ_EVERY: std::time::Duration = std::time::Duration::from_secs(5);
const MEMORY_TRIM_EVERY: std::time::Duration = std::time::Duration::from_secs(30);

impl State {
    pub(super) fn refresh_memory(&mut self) {
        let due = self
            .memory_read
            .is_none_or(|at| at.elapsed() >= MEMORY_READ_EVERY);
        if !due {
            return;
        }
        self.memory_read = Some(std::time::Instant::now());
        self.memory = crate::integrations::process_memory::resident_bytes();
        if self.memory_trimmed.elapsed() >= MEMORY_TRIM_EVERY {
            self.memory_trimmed = std::time::Instant::now();
            crate::integrations::process_memory::give_back_soon();
        }
    }
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::DragStart(split) => state.shell.drag = Some((split, None)),
        Event::DragMove(point) => {
            let shell = &mut state.shell;
            if let Some((split, last)) = &mut shell.drag
                && let Some(previous) = last.replace(point)
            {
                let dx = point.x - previous.x;
                match split {
                    Split::Explorer => {
                        shell.explorer_width = (shell.explorer_width + dx).clamp(180.0, 520.0);
                    }
                }
            }
        }
        Event::DragEnd => state.shell.drag = None,
        Event::Dock(tab) => show_dock(state, Some(tab)),
        Event::ToggleDock => match dock_tab(state) {
            Some(_) => show_dock(state, None),
            None => show_dock(state, Some(state.shell.last_dock)),
        },
        Event::ToggleTab(tab) => {
            if dock_visible(state, tab) {
                hide_dock(state, tab);
            } else {
                show_dock(state, Some(tab));
            }
        }
        Event::ShowSide(side) => return show_side(state, side),
        Event::ToggleSide(side) => {
            if state.editor.explorer && state.shell.side == side {
                state.editor.explorer = false;
            } else {
                return show_side(state, side);
            }
        }
        Event::ResetLayout => {
            let defaults = State::default();
            state.shell.explorer_width = defaults.explorer_width;
            state.editor.explorer = true;
            show_dock(state, None);
            state.message = "Diseño restablecido.".into();
        }
        Event::About => {
            state.message = format!(
                "ChxChx Studio {} · interfaz nativa en Rust (Iced), sin navegador embebido.",
                env!("CARGO_PKG_VERSION")
            );
        }
        Event::ClearMessage => state.message.clear(),
        Event::WindowFocus(focused) => {
            state.shell.focused = focused;
            if focused && state.git.is_repository() {
                return git::update(state, git::Event::Poll);
            }
        }
    }
    Task::none()
}

fn show_side(state: &mut Dashboard, side: Side) -> Task<Message> {
    state.shell.side = side;
    state.editor.explorer = true;
    match side {
        Side::Search => iced::widget::operation::focus(search::INPUT),
        Side::Git => git::refresh(state),
        Side::Agents => agents::refresh(state),
        _ => Task::none(),
    }
}

/// Applies a saved dock choice at startup without touching the remembered tab.
pub(super) fn restore_dock(state: &mut Dashboard, tab: Option<DockTab>) {
    show_dock(state, tab);
}

fn hide_dock(state: &mut Dashboard, tab: DockTab) {
    state.shell.last_dock = tab;
    match tab {
        DockTab::Terminal => state
            .panes
            .hide(|c| matches!(c, panes::Content::Terminal(_))),
        DockTab::Activity => state.panes.hide(|c| *c == panes::Content::Activity),
    }
}

/// Shows a dock tab (its panes are added to the grid) or, with `None`, hides all of them.
fn show_dock(state: &mut Dashboard, tab: Option<DockTab>) {
    let Some(tab) = tab else {
        hide_dock(state, DockTab::Terminal);
        hide_dock(state, DockTab::Activity);
        return;
    };
    state.shell.last_dock = tab;
    match tab {
        DockTab::Terminal => {
            let ids: Vec<u64> = state.terminal.tabs.iter().map(|t| t.id).collect();
            if ids.is_empty() {
                state.panes.show(panes::Content::Terminal(None));
            }
            for id in ids {
                state.panes.show(panes::Content::Terminal(Some(id)));
            }
        }
        DockTab::Activity => {
            state.panes.show(panes::Content::Activity);
        }
    }
}

/// A draggable gutter between the sidebar and the work area.
pub(super) fn splitter<'a>(state: &Dashboard, split: Split) -> Element<'a, Message> {
    let active = matches!(state.shell.drag, Some((current, _)) if current == split);
    mouse_area(
        container(Space::new())
            .style(styles::splitter(active))
            .width(4)
            .height(Fill),
    )
    .on_press(Message::Shell(Event::DragStart(split)))
    .interaction(mouse::Interaction::ResizingHorizontally)
    .into()
}

pub(super) fn subscription(state: &Dashboard) -> Subscription<Message> {
    let shortcuts = Subscription::batch([
        iced::event::listen_with(shortcut),
        iced::event::listen_with(|event, _, _| match event {
            iced::Event::Window(iced::window::Event::Focused) => {
                Some(Message::Shell(Event::WindowFocus(true)))
            }
            iced::Event::Window(iced::window::Event::Unfocused) => {
                Some(Message::Shell(Event::WindowFocus(false)))
            }
            _ => None,
        }),
    ]);
    if state.shell.drag.is_some() {
        Subscription::batch([shortcuts, iced::event::listen_with(drag)])
    } else {
        shortcuts
    }
}

fn drag(event: iced::Event, _: iced::event::Status, _: iced::window::Id) -> Option<Message> {
    match event {
        iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(Message::Shell(Event::DragMove(position)))
        }
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            Some(Message::Shell(Event::DragEnd))
        }
        _ => None,
    }
}

fn shortcut(
    event: iced::Event,
    status: iced::event::Status,
    _: iced::window::Id,
) -> Option<Message> {
    let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    if matches!(
        key.as_ref(),
        keyboard::Key::Named(keyboard::key::Named::Escape)
    ) {
        return Some(Message::Menu(menu::Event::Close));
    }
    if keymap::capturing() {
        return None;
    }
    let chord = keymap::chord(&key, modifiers)?;
    let action = keymap::lookup(&chord)?;
    // The terminal owns Ctrl+<letter> while it has the focus (Ctrl+C, Ctrl+R…); only Studio's reserved ones remain.
    if super::terminal_surface::capturing() && !super::terminal_keys::reserved(&key, modifiers) {
        return None;
    }
    // Keys the focused editor or input already handled must not fire twice.
    if action.needs_free_key() && status != iced::event::Status::Ignored {
        return None;
    }
    Some(match action {
        Action::ToggleSidebar => Message::Shell(Event::ToggleSide(Side::Explorer)),
        Action::ToggleDock => Message::Shell(Event::ToggleDock),
        Action::SearchProject => Message::Shell(Event::ShowSide(Side::Search)),
        Action::Save => Message::Editor(editor::Event::Save),
        Action::CloseTab => Message::Editor(editor::Event::CloseCurrent),
        Action::OpenSettings => Message::Navigate(Page::Settings),
        Action::NewChat => Message::Chat(chat::Event::NewChat),
        Action::CopyLastAnswer => Message::Chat(chat::Event::CopyLast),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragging_resizes_within_limits_and_dock_tabs_are_exclusive() {
        let mut state = editor::fixture();
        let _ = update(&mut state, Event::DragStart(Split::Explorer));
        let _ = update(&mut state, Event::DragMove(Point::new(100.0, 0.0)));
        let _ = update(&mut state, Event::DragMove(Point::new(5000.0, 0.0)));
        assert_eq!(state.shell.explorer_width, 520.0);
        let _ = update(&mut state, Event::DragEnd);
        let _ = update(&mut state, Event::DragMove(Point::new(0.0, 0.0)));
        assert_eq!(state.shell.explorer_width, 520.0);

        let _ = update(&mut state, Event::Dock(DockTab::Activity));
        assert_eq!(dock_tab(&state), Some(DockTab::Activity));
        let _ = update(&mut state, Event::ToggleDock);
        assert_eq!(dock_tab(&state), None);
        let _ = update(&mut state, Event::ToggleDock);
        assert_eq!(dock_tab(&state), Some(DockTab::Activity));
        assert!(!dock_visible(&state, DockTab::Terminal));
    }

    #[test]
    fn rail_clicks_toggle_the_sidebar_and_the_dock_without_stacking_marks() {
        let mut state = editor::fixture();
        assert!(state.editor.explorer && state.shell.side == Side::Explorer);
        let _ = update(&mut state, Event::ToggleSide(Side::Explorer));
        assert!(!state.editor.explorer, "same view again hides the sidebar");
        let _ = update(&mut state, Event::ToggleSide(Side::Search));
        assert!(state.editor.explorer && state.shell.side == Side::Search);
        let _ = update(&mut state, Event::ToggleSide(Side::Workspace));
        assert!(state.editor.explorer && state.shell.side == Side::Workspace);
        let _ = update(&mut state, Event::ToggleSide(Side::Explorer));
        assert!(state.editor.explorer && state.shell.side == Side::Explorer);

        let _ = update(&mut state, Event::ToggleTab(DockTab::Terminal));
        assert_eq!(dock_tab(&state), Some(DockTab::Terminal));
        let _ = update(&mut state, Event::ToggleTab(DockTab::Activity));
        assert!(dock_visible(&state, DockTab::Terminal) && dock_visible(&state, DockTab::Activity));
        let _ = update(&mut state, Event::ToggleTab(DockTab::Activity));
        let _ = update(&mut state, Event::ToggleTab(DockTab::Terminal));
        assert_eq!(dock_tab(&state), None);

        state.shell.explorer_width = 400.0;
        let _ = update(&mut state, Event::ResetLayout);
        assert_eq!(state.shell.explorer_width, 248.0);
    }

    #[test]
    fn memory_is_read_at_most_every_few_seconds_and_without_a_timer() {
        let mut shell = State::default();
        shell.refresh_memory();
        let first = shell.memory_read.expect("the first call reads it");
        shell.refresh_memory();
        assert_eq!(
            shell.memory_read,
            Some(first),
            "an immediate second call does nothing"
        );
        shell.memory_read = Some(std::time::Instant::now() - std::time::Duration::from_secs(6));
        shell.refresh_memory();
        assert!(shell.memory_read.unwrap() > first);
    }
}
