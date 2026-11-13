//! Window layout state: resizable panels, bottom dock and global shortcuts.
use super::*;
use iced::widget::mouse_area;
use iced::{Point, Subscription, keyboard, mouse};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Split {
    Explorer,
    Dock,
}

/// What the left sidebar shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Explorer,
    Search,
    Git,
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            explorer_width: 248.0,
            dock_height: 220.0,
            side: Side::Explorer,
            drag: None,
            last_dock: DockTab::Terminal,
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
}

/// The tab shown in the bottom dock, or `None` when the dock is hidden.
pub(super) fn dock_tab(state: &Dashboard) -> Option<DockTab> {
    if state.terminal.visible {
        Some(DockTab::Terminal)
    } else if state.editor.activity {
        Some(DockTab::Activity)
    } else {
        None
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
                let (dx, dy) = (point.x - previous.x, point.y - previous.y);
                match split {
                    Split::Explorer => {
                        shell.explorer_width = (shell.explorer_width + dx).clamp(180.0, 520.0);
                    }
                    Split::Dock => {
                        shell.dock_height = (shell.dock_height - dy).clamp(110.0, 560.0);
                    }
                }
            }
        }
        Event::DragEnd => state.shell.drag = None,
        Event::Dock(tab) => show_dock(state, Some(tab)),
        Event::ToggleDock => {
            let next = match dock_tab(state) {
                Some(_) => None,
                None => Some(state.shell.last_dock),
            };
            show_dock(state, next);
        }
        Event::ToggleTab(tab) => {
            let next = (dock_tab(state) != Some(tab)).then_some(tab);
            show_dock(state, next);
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
            state.shell.dock_height = defaults.dock_height;
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
    }
    Task::none()
}

fn show_side(state: &mut Dashboard, side: Side) -> Task<Message> {
    state.shell.side = side;
    state.editor.explorer = true;
    match side {
        Side::Search => iced::widget::operation::focus(search::INPUT),
        Side::Git => git::refresh(state),
        _ => Task::none(),
    }
}

/// Applies a saved dock choice at startup without touching the remembered tab.
pub(super) fn restore_dock(state: &mut Dashboard, tab: Option<DockTab>) {
    show_dock(state, tab);
}

fn show_dock(state: &mut Dashboard, tab: Option<DockTab>) {
    if let Some(tab) = tab {
        state.shell.last_dock = tab;
    }
    state.terminal.visible = tab == Some(DockTab::Terminal);
    state.editor.activity = tab == Some(DockTab::Activity);
}

/// A draggable gutter between two panels.
pub(super) fn splitter<'a>(state: &Dashboard, split: Split) -> Element<'a, Message> {
    let active = matches!(state.shell.drag, Some((current, _)) if current == split);
    let bar = container(Space::new()).style(styles::splitter(active));
    let (bar, interaction) = if split == Split::Dock {
        (
            bar.width(Fill).height(4),
            mouse::Interaction::ResizingVertically,
        )
    } else {
        (
            bar.width(4).height(Fill),
            mouse::Interaction::ResizingHorizontally,
        )
    };
    mouse_area(bar)
        .on_press(Message::Shell(Event::DragStart(split)))
        .interaction(interaction)
        .into()
}

pub(super) fn subscription(state: &Dashboard) -> Subscription<Message> {
    let shortcuts = iced::event::listen_with(shortcut);
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
        assert!(!state.terminal.visible);
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
        assert_eq!(dock_tab(&state), Some(DockTab::Activity));
        let _ = update(&mut state, Event::ToggleTab(DockTab::Activity));
        assert_eq!(dock_tab(&state), None);

        state.shell.explorer_width = 400.0;
        let _ = update(&mut state, Event::ResetLayout);
        assert_eq!(state.shell.explorer_width, 248.0);
    }
}
