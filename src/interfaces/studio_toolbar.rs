//! Toolbar: where you are (the sections, as tabs) and the everyday commands, enabled only when
//! they apply.
use super::keymap::{self, Action};
use super::shell::{Event as ShellEvent, Side};
use super::*;
use iced::widget::tooltip;

const HEIGHT: f32 = 36.0;

/// "Guardar (Ctrl+S)": the hint of a command with the shortcut it has right now.
fn with_key(label: &str, action: Action) -> String {
    let binding = keymap::binding(action);
    if binding.is_empty() {
        label.to_owned()
    } else {
        format!("{label} ({})", keymap::display(&binding))
    }
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let tab = tabs::selected_tab(state);
    let on_home = state.editor.home;
    let side_open = |side: Side| state.editor.explorer && state.shell.side == side;
    let editor = |event: editor::Event| Some(Message::Editor(event));
    let page = |page: Page| on_home && state.page == page;
    let section = |icon, label, hint: &'static str, target: Page| {
        tool(
            icon,
            Some(label),
            hint.to_owned(),
            page(target),
            Some(Message::Navigate(target)),
        )
    };

    // Where you are: the sections of the workspace, as tabs.
    let sections = row![
        section("home", "Inicio", "Inicio", Page::Home),
        section("setup", "Preparar", "Proyecto y permisos", Page::Setup),
        section(
            "layers",
            "Catálogo",
            "Skills, Tech Packs y Mods",
            Page::Catalog
        ),
        section(
            "integrations",
            "Conectar",
            "Herramientas y MCP",
            Page::Integrations
        ),
        section(
            "diagnostics",
            "Salud",
            "Diagnóstico del proyecto",
            Page::Diagnostics
        ),
    ]
    .spacing(2)
    .align_y(iced::Center);

    // What you do to the open file.
    let file = row![
        tool(
            "save",
            None,
            with_key("Guardar", Action::Save),
            false,
            tab.is_some_and(|tab| tab.dirty && !state.editor.saving)
                .then_some(Message::Editor(editor::Event::Save)),
        ),
        tool(
            "undo",
            None,
            "Deshacer (Ctrl+Z)".to_owned(),
            false,
            tab.filter(|tab| !tab.undo.is_empty())
                .and_then(|_| editor(editor::Event::Undo)),
        ),
        tool(
            "redo",
            None,
            "Rehacer (Ctrl+Y)".to_owned(),
            false,
            tab.filter(|tab| !tab.redo.is_empty())
                .and_then(|_| editor(editor::Event::Redo)),
        ),
    ]
    .spacing(2)
    .align_y(iced::Center);

    let find = row![
        tool(
            "search",
            None,
            "Buscar en el archivo (Ctrl+F)".to_owned(),
            tab.is_some() && state.editor.search,
            tab.and_then(|_| editor(editor::Event::Search)),
        ),
        tool(
            "folder-search",
            None,
            with_key("Buscar en el proyecto", Action::SearchProject),
            side_open(Side::Search),
            Some(Message::Shell(ShellEvent::ShowSide(Side::Search))),
        ),
        tool(
            "branch",
            None,
            "Git: cambios, historial y ramas".to_owned(),
            side_open(Side::Git),
            Some(Message::Shell(ShellEvent::ShowSide(Side::Git))),
        ),
    ]
    .spacing(2)
    .align_y(iced::Center);

    let mut bar = row![sections, divider(), file, divider(), find]
        .spacing(8)
        .align_y(iced::Center);
    bar = bar.push(Space::new().width(Fill));
    if state.pending > 0 {
        bar = bar.push(
            container(
                row![
                    icons::icon("refresh", design::accent(), 12.0),
                    text("Procesando…").size(11)
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .padding([3, 10])
            .style(styles::chip(design::brand_cyan())),
        );
    }
    bar = bar
        .push(tool(
            "refresh",
            None,
            "Actualizar proyecto".to_owned(),
            false,
            Some(Message::Refresh),
        ))
        .push(tool(
            "settings",
            None,
            with_key("Ajustes: temas, colores y atajos", Action::OpenSettings),
            page(Page::Settings),
            Some(Message::Navigate(Page::Settings)),
        ));
    container(bar)
        .padding([0, 12])
        .height(HEIGHT)
        .align_y(iced::Center)
        .width(Fill)
        .style(styles::surface(design::bg_panel()))
        .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(1)
        .height(16)
        .style(styles::surface(design::border_strong()))
        .into()
}

/// A toolbar button; `message == None` draws it disabled. The active one is marked by a line
/// under it, like a tab, instead of a filled block.
fn tool<'a>(
    icon: &'static str,
    label: Option<&'static str>,
    hint: String,
    active: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let enabled = message.is_some();
    let color = match (enabled, active) {
        (false, _) => design::faint(),
        (true, true) => design::accent(),
        (true, false) => design::muted(),
    };
    let mut content = row![icons::icon(icon, color, 15.0)]
        .spacing(6)
        .align_y(iced::Center);
    if let Some(label) = label {
        content = content.push(
            text(label)
                .size(12)
                .wrapping(iced::widget::text::Wrapping::None),
        );
    }
    let control = button(content)
        .padding([5, 8])
        .style(styles::tool(active))
        .on_press_maybe(message);
    let marker = container(Space::new())
        .width(Fill)
        .height(2)
        .style(styles::surface(if active {
            design::accent()
        } else {
            iced::Color::TRANSPARENT
        }));
    let stacked = iced::widget::column![
        Space::new().height(2),
        control,
        container(marker).padding([0, 6]),
    ]
    .width(iced::Shrink)
    .align_x(iced::Center);
    styles::tip(stacked, hint, tooltip::Position::Bottom)
}
