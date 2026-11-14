//! Bottom dock: tabbed terminal and activity output.
use super::shell::{DockTab, Event};
use super::*;
use iced::widget::{column, stack, tooltip};

pub(super) fn view(state: &Dashboard, current: DockTab) -> Element<'_, Message> {
    let sessions = state.terminal.tabs.len();
    let terminal_label = if state.terminal.opening {
        "Terminal · iniciando".to_owned()
    } else if sessions > 0 {
        format!("Terminal · {sessions}")
    } else {
        "Terminal".to_owned()
    };
    let mut header = row![
        tab(terminal_label, "terminal", current, DockTab::Terminal),
        tab("Actividad".to_owned(), "bell", current, DockTab::Activity),
        Space::new().width(Fill),
    ]
    .spacing(4)
    .align_y(iced::Center);
    if current == DockTab::Terminal {
        header = header.push(terminal::toolbar(state));
    }
    header = header.push(styles::tip(
        button(icons::icon("close", design::MUTED, 13.0))
            .padding(5)
            .style(styles::ghost(false))
            .on_press(Message::Shell(Event::ToggleDock)),
        "Ocultar panel (Ctrl+J)",
        tooltip::Position::Top,
    ));
    let body = match current {
        DockTab::Terminal => terminal::view(state),
        DockTab::Activity => activity(state),
    };
    container(column![container(header).padding([0, 10]), body])
        .width(Fill)
        .height(state.shell.dock_height)
        .style(styles::surface(design::BG_PANEL))
        .into()
}

fn tab<'a>(
    label: String,
    icon: &'static str,
    current: DockTab,
    target: DockTab,
) -> Element<'a, Message> {
    let active = current == target;
    let color = if active {
        design::ACCENT
    } else {
        design::MUTED
    };
    let action = button(
        row![
            icons::icon(icon, color, 14.0),
            text(label).size(11).font(design::UI_BOLD)
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .padding([8, 8])
    .style(styles::plain(active))
    .on_press(Message::Shell(Event::Dock(target)));
    let marker = container(
        container(Space::new())
            .width(Fill)
            .height(2)
            .style(styles::surface(if active {
                design::ACCENT
            } else {
                iced::Color::TRANSPARENT
            })),
    )
    .align_bottom(Fill);
    stack![action, marker].into()
}

fn activity(state: &Dashboard) -> Element<'_, Message> {
    let content: Element<'_, Message> = if state.message.is_empty() {
        text("Sin operaciones recientes.")
            .size(12)
            .color(design::FAINT)
            .into()
    } else {
        row![
            icons::icon("info", design::ACCENT, 14.0),
            text(&state.message).size(12)
        ]
        .spacing(8)
        .into()
    };
    container(styles::vscroll(container(content).padding(12).width(Fill)).height(Fill))
        .padding([0, 4])
        .width(Fill)
        .height(Fill)
        .into()
}
