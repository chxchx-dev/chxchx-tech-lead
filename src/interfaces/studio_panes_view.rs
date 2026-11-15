//! Drawing of the pane grid: each pane shows a strip of tabs (like an editor group) and the active one.
use super::super::*;
use super::{Content, Event, Group};
use iced::widget::pane_grid::{self, Axis, PaneGrid, TitleBar};
use iced::widget::tooltip;

pub(in crate::interfaces::studio) fn view(state: &Dashboard) -> Element<'_, Message> {
    let focus = state.panes.focus;
    let grid = PaneGrid::new(&state.panes.grid, move |pane, group, is_maximized| {
        let focused = focus == Some(pane);
        let current = group.current();
        let mut controls = row![].spacing(2).align_y(iced::Center);
        if matches!(current, Some(Content::Terminal(_))) {
            controls = controls.push(control(
                "stop",
                "Interrumpir (Ctrl+C)",
                Message::Terminal(terminal::Event::Interrupt),
            ));
        }
        if !is_maximized {
            controls = if group.tabs.len() > 1 {
                controls
                    .push(control(
                        "columns",
                        "Sacar esta pestaña a un panel a la derecha",
                        Message::Panes(Event::MoveOut(pane, Axis::Vertical)),
                    ))
                    .push(control(
                        "panel-bottom",
                        "Sacar esta pestaña a un panel de abajo",
                        Message::Panes(Event::MoveOut(pane, Axis::Horizontal)),
                    ))
            } else {
                controls
                    .push(control(
                        "columns",
                        "Dividir a la derecha con una terminal",
                        Message::Panes(Event::Split(pane, Axis::Vertical)),
                    ))
                    .push(control(
                        "panel-bottom",
                        "Dividir abajo con una terminal",
                        Message::Panes(Event::Split(pane, Axis::Horizontal)),
                    ))
            };
        }
        controls = controls.push(if is_maximized {
            control("minus", "Restaurar", Message::Panes(Event::Restore))
        } else {
            control("layers", "Maximizar", Message::Panes(Event::Maximize(pane)))
        });
        let body = match current {
            Some(content) => body(state, content),
            None => Space::new().into(),
        };
        pane_grid::Content::new(body)
            .title_bar(
                TitleBar::new(tab_strip(state, pane, group, focused))
                    .controls(pane_grid::Controls::new(controls))
                    .always_show_controls()
                    .padding([2, 8])
                    .style(styles::pane_title(focused)),
            )
            .style(styles::pane(focused))
    })
    .spacing(8)
    .min_size(120)
    .on_click(|pane| Message::Panes(Event::Focus(pane)))
    .on_drag(|drag| Message::Panes(Event::Dragged(drag)))
    .on_resize(8, |resize| Message::Panes(Event::Resized(resize)))
    .style(styles::pane_grid);
    container(grid)
        .padding(8)
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
        Content::Local => ("cpu", design::TINT_GREEN, "Modelos locales".to_owned()),
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

/// One tab per item of the group; the pane stays draggable by the space around them.
fn tab_strip<'a>(
    state: &'a Dashboard,
    pane: pane_grid::Pane,
    group: &'a Group,
    focused: bool,
) -> Element<'a, Message> {
    let mut strip = row![].spacing(3).align_y(iced::Center);
    for (index, content) in group.tabs.iter().enumerate() {
        let active = index == group.active;
        let (icon, tint, title) = describe(state, content);
        let mut tab = row![
            icons::icon(
                icon,
                if active && focused {
                    design::ACCENT
                } else if active {
                    tint
                } else {
                    design::FAINT
                },
                13.0,
            ),
            text(title)
                .size(11)
                .font(design::UI_BOLD)
                .color(if active && focused {
                    design::TEXT
                } else if active {
                    design::MUTED
                } else {
                    design::FAINT
                })
                .wrapping(iced::widget::text::Wrapping::None),
        ]
        .spacing(6)
        .align_y(iced::Center);
        if *content != Content::Editor {
            tab = tab.push(
                button(icons::icon("close", design::FAINT, 10.0))
                    .padding(2)
                    .style(styles::ghost(false))
                    .on_press(Message::Panes(Event::CloseTab(pane, index))),
            );
        }
        strip = strip.push(
            button(tab)
                .padding([5, 9])
                .style(styles::ghost(active))
                .on_press(Message::Panes(Event::Select(pane, index))),
        );
    }
    strip.into()
}

fn body<'a>(state: &'a Dashboard, content: &'a Content) -> Element<'a, Message> {
    match content {
        Content::Editor => ide_editor(state),
        Content::Chat(id) => chat::view(state, id),
        Content::Terminal(id) => terminal::view(state, *id),
        Content::Activity => dock::activity(state),
        Content::Local => local::view(state),
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
