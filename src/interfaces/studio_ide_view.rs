//! IDE shell composition: chrome, side panels, editor group, dock and dialogs.
use super::editor::Event;
use super::shell::Split;
use super::*;
use iced::widget::{column, opaque, stack};

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let center = panes::view(state);
    let mut work = row![chrome::activity_rail(state)].height(Fill);
    if state.editor.explorer {
        work = work
            .push(match state.shell.side {
                shell::Side::Explorer => panels::explorer(state),
                shell::Side::Search => search::view(state),
                shell::Side::Git => git_view::view(state),
                shell::Side::Agents => agents::view(state),
                shell::Side::Workspace => workspace::view(state),
                shell::Side::Hive => hive::view(state),
            })
            .push(shell::splitter(state, Split::Explorer));
    }
    work = work.push(container(center).width(Fill).height(Fill));
    let mut window = column![chrome::title_bar(state), toolbar::view(state), work];
    if !state.message.is_empty() {
        window = window.push(notice(state));
    }
    let window = window.push(status::status_bar(state));
    let mut layers = stack![window];
    if let Some(menu) = menu::overlay(state) {
        layers = layers.push(menu);
    }
    if state.editor.closing.is_some() || state.editor.exiting {
        layers = layers.push(confirm(state));
    }
    if let Some(dialog) = update::dialog(state) {
        layers = layers.push(dialog);
    }
    layers.into()
}

/// Latest operation message, pinned above the status bar until dismissed.
fn notice(state: &Dashboard) -> Element<'_, Message> {
    container(
        row![
            icons::icon("info", design::accent(), 14.0),
            styles::vscroll(text(&state.message).size(12))
                .height(18)
                .width(Fill),
            button(text("Ver actividad").size(11))
                .padding([3, 8])
                .style(styles::ghost(false))
                .on_press(Message::Shell(shell::Event::Dock(shell::DockTab::Activity))),
            button(icons::icon("close", design::muted(), 12.0))
                .padding(4)
                .style(styles::ghost(false))
                .on_press(Message::Shell(shell::Event::ClearMessage)),
        ]
        .spacing(8)
        .align_y(iced::Center),
    )
    .padding([4, 12])
    .width(Fill)
    .style(styles::surface(design::bg_raised()))
    .into()
}

/// Modal confirmation for discarding unsaved work or leaving Studio.
fn confirm(state: &Dashboard) -> Element<'_, Message> {
    let (title, detail, label, event) = if state.editor.exiting {
        (
            "Cerrar Studio",
            "Guarda tus archivos antes de salir. La terminal local se cerrará al terminar Studio.",
            "Salir y descartar",
            Event::ExitDiscard,
        )
    } else {
        (
            "Hay cambios sin guardar",
            "Guarda el archivo o confirma que deseas descartar los cambios.",
            "Descartar y cerrar",
            Event::Discard,
        )
    };
    let dialog = container(
        column![
            row![
                icons::icon("warning", design::warning(), 20.0),
                text(title).size(16).font(design::UI_BOLD)
            ]
            .spacing(10)
            .align_y(iced::Center),
            text(detail).size(13).color(design::muted()),
            row![
                Space::new().width(Fill),
                button(text("Volver al editor").size(12))
                    .padding([7, 14])
                    .style(styles::secondary)
                    .on_press(Message::Editor(Event::Cancel)),
                button(text(label).size(12))
                    .padding([7, 14])
                    .style(styles::danger)
                    .on_press(Message::Editor(event)),
            ]
            .spacing(10),
        ]
        .spacing(16),
    )
    .padding(22)
    .width(440)
    .style(styles::modal);
    opaque(
        container(dialog)
            .center(Fill)
            .style(styles::surface(design::alpha(design::bg_deep(), 0.72))),
    )
}
