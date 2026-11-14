//! Window layout state: resizable panels, bottom dock and global shortcuts.
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
    memory_ticks: u32,
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
            memory_ticks: 0,
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
    /// Reads the memory Studio holds and, now and then, hands freed memory back to the system.
    Memory,
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

/// Memory is read every 5 s; every 6th reading (30 s) the freed memory is handed back.
const MEMORY_TRIM_EVERY: u32 = 6;

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
        Event::Memory => {
            let shell = &mut state.shell;
            shell.memory = crate::integrations::process_memory::resident_bytes();
            shell.memory_ticks += 1;
            if shell.memory_ticks.is_multiple_of(MEMORY_TRIM_EVERY) {
                crate::integrations::process_memory::give_back_soon();
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
        iced::time::every(std::time::Duration::from_secs(5)).map(|_| Message::Shell(Event::Memory)),
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
    if !modifiers.command() || modifiers.alt() {
        return None;
    }
    // The terminal owns Ctrl+<letter> while it has the focus (Ctrl+C, Ctrl+R…); only Studio's reserved ones remain.
    if super::terminal_surface::capturing() && !super::terminal_keys::reserved(&key, modifiers) {
        return None;
    }
    // Keys the focused editor or input already handled must not fire twice.
    let free = status == iced::event::Status::Ignored;
    let character = match key.as_ref() {
        keyboard::Key::Character(character) => character.to_ascii_lowercase(),
        _ => return None,
    };
    match (character.as_str(), modifiers.shift()) {
        ("b", false) => Some(Message::Shell(Event::ToggleSide(Side::Explorer))),
        ("j", false) => Some(Message::Shell(Event::ToggleDock)),
        ("f", true) => Some(Message::Shell(Event::ShowSide(Side::Search))),
        ("s", false) if free => Some(Message::Editor(editor::Event::Save)),
        ("w", false) if free => Some(Message::Editor(editor::Event::CloseCurrent)),
        _ => None,
    }
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
}
