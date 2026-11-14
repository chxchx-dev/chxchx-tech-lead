//! Editor group: tab strip, breadcrumbs, in-file search and the code editor.
use super::editor::{Event, Tab};
use super::*;
use iced::widget::{column, stack, text_editor, text_input, tooltip};

const TAB_HEIGHT: f32 = 34.0;

pub(super) fn selected_tab(state: &Dashboard) -> Option<&Tab> {
    state
        .editor
        .selected
        .filter(|_| !state.editor.home)
        .and_then(|index| state.editor.tabs.get(index))
}

pub(super) fn tab_bar(state: &Dashboard) -> Element<'_, Message> {
    let mut tabs = row![tab(
        brand::mark(15.0).into(),
        "Inicio".to_owned(),
        state.editor.home,
        Message::Editor(Event::Home),
        None,
    )];
    for (index, open) in state.editor.tabs.iter().enumerate() {
        let active = !state.editor.home && state.editor.selected == Some(index);
        let kind = file_types::of(&open.document.relative);
        let name = open
            .document
            .relative
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        tabs = tabs.push(tab(
            icons::icon(kind.icon, kind.tint, 14.0).into(),
            name,
            active,
            Message::Editor(Event::Select(index)),
            Some((open.dirty, Message::Editor(Event::Close(index)))),
        ));
    }
    container(styles::hscroll(tabs))
        .width(Fill)
        .height(TAB_HEIGHT)
        .style(styles::surface(design::BG_PANEL))
        .into()
}

fn tab<'a>(
    icon: Element<'a, Message>,
    label: String,
    active: bool,
    select: Message,
    close: Option<(bool, Message)>,
) -> Element<'a, Message> {
    let mut content = row![
        button(
            row![icon, text(label).size(12)]
                .spacing(7)
                .align_y(iced::Center)
        )
        .padding([0, 4])
        .style(styles::plain(active))
        .on_press(select)
    ]
    .spacing(2)
    .align_y(iced::Center);
    if let Some((dirty, message)) = close {
        // A dot marks unsaved changes; the cross is always available to close.
        if dirty {
            content = content.push(icons::icon("dot", design::WARNING, 10.0));
        }
        content = content.push(
            button(icons::icon("close", design::MUTED, 12.0))
                .padding(3)
                .style(styles::ghost(false))
                .on_press(message),
        );
    }
    let body = container(content)
        .padding([0, 10])
        .height(TAB_HEIGHT)
        .align_y(iced::Center)
        .style(styles::surface(if active {
            design::BG_BASE
        } else {
            design::BG_PANEL
        }));
    let marker = container(Space::new())
        .width(Fill)
        .height(2)
        .style(styles::surface(if active {
            design::ACCENT
        } else {
            iced::Color::TRANSPARENT
        }));
    row![
        stack![body, marker],
        container(Space::new())
            .width(1)
            .height(TAB_HEIGHT)
            .style(styles::surface(design::BG_DEEP))
    ]
    .into()
}

pub(super) fn editor_area(state: &Dashboard) -> Element<'_, Message> {
    let Some(tab) = selected_tab(state) else {
        return container(
            column![
                brand::mark(96.0).opacity(0.35_f32),
                text("Abre un archivo desde el explorador")
                    .size(13)
                    .color(design::MUTED),
            ]
            .spacing(16)
            .align_x(iced::Center),
        )
        .center(Fill)
        .style(styles::surface(design::BG_BASE))
        .into();
    };
    let syntax = tab
        .document
        .relative
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("txt");
    let mut area = column![breadcrumbs(state, tab)];
    if state.editor.search {
        area = area.push(search_bar(state));
    }
    area.push(
        text_editor(&tab.content)
            .on_action(|action| Message::Editor(Event::Edit(action)))
            .style(styles::editor)
            .font(design::MONO)
            .size(14)
            .line_height(1.5)
            .padding([10, 18])
            .height(Fill)
            .highlight(syntax, iced::highlighter::Theme::Base16Ocean)
            .key_binding(key_binding),
    )
    .into()
}

fn key_binding(key: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if !key.modifiers.command() {
        return text_editor::Binding::from_key_press(key);
    }
    let event = match key.key.as_ref() {
        iced::keyboard::Key::Character("s") => Some(Event::Save),
        iced::keyboard::Key::Character("z") if key.modifiers.shift() => Some(Event::Redo),
        iced::keyboard::Key::Character("z") => Some(Event::Undo),
        iced::keyboard::Key::Character("y") => Some(Event::Redo),
        iced::keyboard::Key::Character("f") => Some(Event::Search),
        _ => None,
    };
    event
        .map(|event| text_editor::Binding::Custom(Message::Editor(event)))
        .or_else(|| text_editor::Binding::from_key_press(key))
}

fn breadcrumbs<'a>(state: &'a Dashboard, tab: &'a Tab) -> Element<'a, Message> {
    let mut trail = row![].spacing(4).align_y(iced::Center);
    let parts: Vec<String> = tab
        .document
        .relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().to_string())
        .collect();
    let last = parts.len().saturating_sub(1);
    for (index, part) in parts.into_iter().enumerate() {
        if index > 0 {
            trail = trail.push(icons::icon("chevron-right", design::FAINT, 11.0));
        }
        trail = trail.push(text(part).size(12).color(if index == last {
            design::TEXT
        } else {
            design::MUTED
        }));
    }
    let save_tint = if tab.dirty {
        design::ACCENT
    } else {
        design::MUTED
    };
    let save_label = if state.editor.saving {
        "Guardando…"
    } else {
        "Guardar (Ctrl+S)"
    };
    container(
        row![
            trail,
            Space::new().width(Fill),
            tool("save", save_label, save_tint, Event::Save),
            tool("undo", "Deshacer (Ctrl+Z)", design::MUTED, Event::Undo),
            tool("redo", "Rehacer (Ctrl+Y)", design::MUTED, Event::Redo),
            tool(
                "search",
                "Buscar (Ctrl+F)",
                if state.editor.search {
                    design::ACCENT
                } else {
                    design::MUTED
                },
                Event::Search
            ),
        ]
        .spacing(2)
        .align_y(iced::Center),
    )
    .padding([3, 12])
    .width(Fill)
    .style(styles::surface(design::BG_BASE))
    .into()
}

fn tool<'a>(
    icon: &'static str,
    label: &'a str,
    color: iced::Color,
    event: Event,
) -> Element<'a, Message> {
    styles::tip(
        button(icons::icon(icon, color, 15.0))
            .padding(5)
            .style(styles::ghost(false))
            .on_press(Message::Editor(event)),
        label,
        tooltip::Position::Bottom,
    )
}

fn search_bar(state: &Dashboard) -> Element<'_, Message> {
    container(
        row![
            icons::icon("search", design::MUTED, 14.0),
            text_input("Buscar en este archivo…", &state.editor.query)
                .on_input(|query| Message::Editor(Event::Query(query)))
                .on_submit(Message::Editor(Event::FindNext))
                .size(12)
                .padding([5, 8])
                .width(320)
                .style(styles::input),
            button(text("Siguiente").size(12))
                .padding([5, 10])
                .style(styles::secondary)
                .on_press(Message::Editor(Event::FindNext)),
            Space::new().width(Fill),
            button(icons::icon("close", design::MUTED, 13.0))
                .padding(5)
                .style(styles::ghost(false))
                .on_press(Message::Editor(Event::Search)),
        ]
        .spacing(8)
        .align_y(iced::Center),
    )
    .padding([6, 12])
    .width(Fill)
    .style(styles::surface(design::BG_PANEL))
    .into()
}
