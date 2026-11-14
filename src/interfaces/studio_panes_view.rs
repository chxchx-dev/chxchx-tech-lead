//! Drawing of the pane grid: a title bar per pane (drag handle, what it shows, split/maximize/close).
use super::super::*;
use super::{Content, Event};
use iced::widget::pane_grid::{self, Axis, PaneGrid, TitleBar};
use iced::widget::tooltip;

pub(in crate::interfaces::studio) fn view(state: &Dashboard) -> Element<'_, Message> {
    let focus = state.panes.focus;
    let grid = PaneGrid::new(&state.panes.grid, move |pane, content, is_maximized| {
        let focused = focus == Some(pane);
        let (icon, tint, title) = describe(state, content);
        let mut controls = row![].spacing(2).align_y(iced::Center);
        if matches!(content, Content::Terminal(_)) {
            controls = controls.push(control(
                "stop",
                "Interrumpir (Ctrl+C)",
                Message::Terminal(terminal::Event::Interrupt),
            ));
        }
        if !is_maximized && !matches!(content, Content::Activity) {
            controls = controls
                .push(control(
                    "columns",
                    "Dividir a la derecha con una terminal",
                    Message::Panes(Event::Split(pane, Axis::Vertical)),
                ))
                .push(control(
                    "panel-bottom",
                    "Dividir abajo con una terminal",
                    Message::Panes(Event::Split(pane, Axis::Horizontal)),
                ));
        }
        controls = controls.push(if is_maximized {
            control("minus", "Restaurar", Message::Panes(Event::Restore))
        } else {
            control("layers", "Maximizar", Message::Panes(Event::Maximize(pane)))
        });
        if !matches!(content, Content::Editor) {
            controls = controls.push(control(
                "close",
                "Cerrar",
                Message::Panes(Event::Close(pane)),
            ));
        }
        let head = row![
            icons::icon(
                "dot",
                if focused {
                    design::ACCENT
                } else {
                    design::BORDER_STRONG
                },
                8.0
            ),
            icons::icon(icon, if focused { tint } else { design::MUTED }, 13.0),
            text(title)
                .size(11)
                .font(design::UI_BOLD)
                .color(if focused { design::TEXT } else { design::MUTED })
                .wrapping(iced::widget::text::Wrapping::None),
        ]
        .spacing(7)
        .align_y(iced::Center);
        pane_grid::Content::new(body(state, content))
            .title_bar(
                TitleBar::new(head)
                    .controls(pane_grid::Controls::new(controls))
                    .always_show_controls()
                    .padding([5, 10])
                    .style(styles::pane_title(focused)),
            )
            .style(styles::pane(focused))
    })
    .spacing(6)
    .min_size(120)
    .on_click(|pane| Message::Panes(Event::Focus(pane)))
    .on_drag(|drag| Message::Panes(Event::Dragged(drag)))
    .on_resize(8, |resize| Message::Panes(Event::Resized(resize)))
    .style(styles::pane_grid);
    container(grid)
        .padding(6)
        .width(Fill)
        .height(Fill)
        .style(styles::surface(design::BG_DEEP))
        .into()
}

fn control<'a>(icon: &'static str, tip: &'static str, message: Message) -> Element<'a, Message> {
    styles::tip(
        button(icons::icon(icon, design::MUTED, 13.0))
            .padding(4)
            .style(styles::ghost(false))
            .on_press(message),
        tip,
        tooltip::Position::Bottom,
    )
}

fn describe<'a>(state: &'a Dashboard, content: &'a Content) -> (&'static str, iced::Color, String) {
    match content {
        Content::Editor => ("code", design::BRAND_CYAN, "Editor".to_owned()),
        Content::Activity => ("bell", design::MUTED, "Actividad".to_owned()),
        Content::Chat(id) => {
            let label = state
                .chat
                .chats
                .iter()
                .find(|c| c.agent.id == *id)
                .map_or_else(|| id.clone(), |c| c.agent.label());
            ("sparkle", design::ACCENT, format!("Chat · {label}"))
        }
        Content::Terminal(Some(id)) => {
            let tab = state.terminal.tabs.iter().find(|t| t.id == *id);
            let title = tab.map_or_else(
                || "Terminal".to_owned(),
                |t| {
                    if t.exited {
                        format!("{} · terminó", t.title)
                    } else {
                        t.title.clone()
                    }
                },
            );
            ("terminal", design::TINT_GREEN, title)
        }
        Content::Terminal(None) => ("terminal", design::TINT_GREEN, "Terminal".to_owned()),
    }
}

fn body<'a>(state: &'a Dashboard, content: &'a Content) -> Element<'a, Message> {
    match content {
        Content::Editor => ide_editor(state),
        Content::Chat(id) => chat::view(state, id),
        Content::Terminal(id) => terminal::view(state, *id),
        Content::Activity => dock::activity(state),
    }
}

/// The editor group: file tabs and the open file (or the home page).
fn ide_editor(state: &Dashboard) -> Element<'_, Message> {
    let content = if state.editor.home {
        layout::view(state)
    } else {
        tabs::editor_area(state)
    };
    iced::widget::column![tabs::tab_bar(state), content]
        .height(Fill)
        .into()
}
