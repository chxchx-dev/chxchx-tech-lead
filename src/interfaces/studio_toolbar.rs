//! Toolbar: the everyday commands, enabled only when they apply.
use super::shell::{Event as ShellEvent, Side};
use super::*;
use iced::widget::tooltip;

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let tab = tabs::selected_tab(state);
    let on_home = state.editor.home;
    let side_open = |side: Side| state.editor.explorer && state.shell.side == side;
    let editor = |event: editor::Event| Some(Message::Editor(event));
    let page = |page: Page| on_home && state.page == page;

    let mut bar = row![
        tool(
            "home",
            Some("Inicio"),
            "Inicio",
            page(Page::Home),
            Some(Message::Navigate(Page::Home))
        ),
        divider(),
        tool(
            "save",
            Some("Guardar"),
            "Guardar (Ctrl+S)",
            false,
            tab.is_some_and(|tab| tab.dirty && !state.editor.saving)
                .then_some(Message::Editor(editor::Event::Save)),
        ),
        tool(
            "undo",
            None,
            "Deshacer (Ctrl+Z)",
            false,
            tab.filter(|tab| !tab.undo.is_empty())
                .and_then(|_| editor(editor::Event::Undo)),
        ),
        tool(
            "redo",
            None,
            "Rehacer (Ctrl+Y)",
            false,
            tab.filter(|tab| !tab.redo.is_empty())
                .and_then(|_| editor(editor::Event::Redo)),
        ),
        divider(),
        tool(
            "search",
            None,
            "Buscar en el archivo (Ctrl+F)",
            tab.is_some() && state.editor.search,
            tab.and_then(|_| editor(editor::Event::Search)),
        ),
        tool(
            "folder-search",
            None,
            "Buscar en el proyecto (Ctrl+Shift+F)",
            side_open(Side::Search),
            Some(Message::Shell(ShellEvent::ShowSide(Side::Search))),
        ),
        tool(
            "branch",
            None,
            "Git: cambios, historial y ramas",
            side_open(Side::Git),
            Some(Message::Shell(ShellEvent::ShowSide(Side::Git))),
        ),
        divider(),
        tool(
            "setup",
            Some("Preparar"),
            "Proyecto y permisos",
            page(Page::Setup),
            Some(Message::Navigate(Page::Setup)),
        ),
        tool(
            "layers",
            Some("Catálogo"),
            "Skills, Tech Packs y Mods",
            page(Page::Catalog),
            Some(Message::Navigate(Page::Catalog)),
        ),
        tool(
            "integrations",
            Some("Conectar"),
            "Herramientas y MCP",
            page(Page::Integrations),
            Some(Message::Navigate(Page::Integrations)),
        ),
        tool(
            "diagnostics",
            Some("Salud"),
            "Diagnóstico del proyecto",
            page(Page::Diagnostics),
            Some(Message::Navigate(Page::Diagnostics)),
        ),
    ]
    .spacing(2)
    .align_y(iced::Center);
    bar = bar.push(Space::new().width(Fill));
    if state.pending > 0 {
        bar = bar.push(
            container(
                row![
                    icons::icon("refresh", design::ACCENT, 12.0),
                    text("Procesando…").size(11)
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .padding([3, 10])
            .style(styles::chip(design::BRAND_CYAN)),
        );
    }
    bar = bar.push(tool(
        "refresh",
        None,
        "Actualizar proyecto",
        false,
        Some(Message::Refresh),
    ));
    container(bar)
        .padding([3, 12])
        .width(Fill)
        .style(styles::surface(design::BG_PANEL))
        .into()
}

fn divider<'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(1)
        .height(18)
        .style(styles::surface(design::BORDER))
        .into()
}

/// A toolbar button; `message == None` draws it disabled.
fn tool<'a>(
    icon: &'static str,
    label: Option<&'static str>,
    hint: &'static str,
    active: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let color = match (message.is_some(), active) {
        (false, _) => design::FAINT,
        (true, true) => design::ACCENT,
        (true, false) => design::MUTED,
    };
    let mut content = row![icons::icon(icon, color, 16.0)]
        .spacing(6)
        .align_y(iced::Center);
    if let Some(label) = label {
        content = content.push(text(label).size(11));
    }
    let mut control = button(content).padding([5, 8]).style(styles::ghost(active));
    if let Some(message) = message {
        control = control.on_press(message);
    }
    styles::tip(control, hint, tooltip::Position::Bottom)
}
