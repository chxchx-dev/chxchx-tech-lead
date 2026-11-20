//! What the explorer draws for its interaction: the context menu, the inline name box and the
//! delete confirmation. The menu's rows are data (`rows`) so what each target offers is tested.
use super::{Action, Edit, EditKind, Event, Target};
use crate::interfaces::studio::keymap::{self, Action as Key};
use crate::interfaces::studio::*;
use iced::widget::{column, mouse_area, opaque, pin, stack, text_input};
use iced::{Point, Size};

pub(in crate::interfaces::studio) const MENU_WIDTH: f32 = 236.0;
const ROW_HEIGHT: f32 = 27.0;
const SEPARATOR_HEIGHT: f32 = 9.0;
const PADDING: f32 = 8.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::interfaces::studio) enum Row {
    Item {
        label: String,
        hint: String,
        action: Action,
        enabled: bool,
    },
    Separator,
}

fn item(label: &str, action: Action, enabled: bool, key: Option<Key>) -> Row {
    Row::Item {
        label: label.to_owned(),
        hint: key
            .map(|key| keymap::display(&keymap::binding(key)))
            .unwrap_or_default(),
        action,
        enabled,
    }
}

/// What the menu offers for this target and the current state of the selection and clipboard.
pub(in crate::interfaces::studio) fn rows(state: &Dashboard, target: &Target) -> Vec<Row> {
    let can_paste = state.explorer.clipboard.is_some();
    let count = match target {
        Target::Root => 0,
        Target::Item { path, .. } => {
            if state.explorer.selection.contains(path) {
                state.explorer.selection.len()
            } else {
                1
            }
        }
    };
    let mut rows = Vec::new();
    match target {
        Target::Root => {
            rows.extend([
                item(
                    "Nuevo archivo",
                    Action::NewFile,
                    true,
                    Some(Key::ExplorerNewFile),
                ),
                item("Nueva carpeta", Action::NewFolder, true, None),
                Row::Separator,
                item("Pegar", Action::Paste, can_paste, Some(Key::ExplorerPaste)),
                Row::Separator,
                item("Copiar la ruta del proyecto", Action::CopyPath, true, None),
                item(
                    "Mostrar en el explorador de archivos",
                    Action::Reveal,
                    true,
                    None,
                ),
                Row::Separator,
                item("Actualizar", Action::Refresh, true, None),
                item("Contraer todo", Action::CollapseAll, true, None),
            ]);
            if super::undo_depth(state) > 0 {
                rows.push(item(
                    "Deshacer la última eliminación",
                    Action::UndoDelete,
                    true,
                    None,
                ));
            }
        }
        Target::Item { path, directory } => {
            if count == 1 {
                if *directory {
                    rows.extend([
                        item(
                            "Nuevo archivo",
                            Action::NewFile,
                            true,
                            Some(Key::ExplorerNewFile),
                        ),
                        item("Nueva carpeta", Action::NewFolder, true, None),
                        Row::Separator,
                        item(
                            if state.editor.expanded.contains(path) {
                                "Contraer"
                            } else {
                                "Expandir"
                            },
                            Action::Open,
                            true,
                            None,
                        ),
                    ]);
                } else {
                    rows.push(item("Abrir", Action::Open, true, None));
                }
                rows.push(Row::Separator);
            }
            rows.extend([
                item(
                    "Renombrar",
                    Action::Rename,
                    count == 1,
                    Some(Key::ExplorerRename),
                ),
                item("Duplicar", Action::Duplicate, true, None),
                Row::Separator,
                item("Copiar", Action::Copy, true, Some(Key::ExplorerCopy)),
                item("Cortar", Action::Cut, true, Some(Key::ExplorerCut)),
                item("Pegar", Action::Paste, can_paste, Some(Key::ExplorerPaste)),
                Row::Separator,
                item("Copiar nombre", Action::CopyName, true, None),
                item("Copiar ruta", Action::CopyPath, true, None),
                item("Copiar ruta relativa", Action::CopyRelative, true, None),
                Row::Separator,
                item(
                    "Mostrar en el explorador de archivos",
                    Action::Reveal,
                    count == 1,
                    None,
                ),
                Row::Separator,
                item(
                    &if count > 1 {
                        format!("Eliminar {count} elementos")
                    } else {
                        "Eliminar".to_owned()
                    },
                    Action::Delete,
                    true,
                    Some(Key::ExplorerDelete),
                ),
            ]);
        }
    }
    rows
}

fn menu_height(rows: &[Row]) -> f32 {
    PADDING
        + rows
            .iter()
            .map(|row| match row {
                Row::Item { .. } => ROW_HEIGHT,
                Row::Separator => SEPARATOR_HEIGHT,
            })
            .sum::<f32>()
}

/// Top-left corner of the menu: where the pointer was, moved to stay inside the panel.
pub(in crate::interfaces::studio) fn place(at: Point, size: Size, height: f32) -> Point {
    let x = at.x.min(size.width - MENU_WIDTH - 4.0).max(4.0);
    let y = if at.y + height > size.height - 4.0 {
        (at.y - height)
            .max(4.0)
            .min((size.height - height - 4.0).max(4.0))
    } else {
        at.y
    };
    Point::new(x, y)
}

fn row_view(row: &Row) -> Element<'static, Message> {
    match row {
        Row::Separator => container(Space::new())
            .width(Fill)
            .height(1)
            .style(styles::surface(design::border()))
            .into(),
        Row::Item {
            label,
            hint,
            action,
            enabled,
        } => {
            let content = row![
                text(label.clone()).size(12),
                Space::new().width(Fill),
                text(hint.clone()).size(11).color(design::faint())
            ]
            .align_y(iced::Center);
            let mut button = button(content)
                .width(Fill)
                .padding([5, 10])
                .style(styles::menu_item);
            if *enabled {
                button = button.on_press(Message::Explorer(Event::Do(*action)));
            }
            button.into()
        }
    }
}

/// The open context menu as a layer over the explorer panel of this size, or nothing.
pub(in crate::interfaces::studio) fn context_menu(
    state: &Dashboard,
    size: Size,
) -> Option<Element<'_, Message>> {
    let (target, at) = state.explorer.menu.as_ref()?;
    let rows = rows(state, target);
    let corner = place(*at, size, menu_height(&rows));
    let mut list = column![].spacing(1);
    for row in &rows {
        list = list.push(row_view(row));
    }
    let panel = opaque(
        container(list.padding(4))
            .width(MENU_WIDTH)
            .style(styles::menu_panel),
    );
    // A click anywhere else in the panel closes the menu and does what it would have done.
    let catcher = mouse_area(container(Space::new()).width(Fill).height(Fill))
        .on_press(Message::Explorer(Event::Escape))
        .on_right_press(Message::Explorer(Event::Escape));
    Some(stack![catcher, pin(panel).x(corner.x).y(corner.y)].into())
}

/// The name box of a rename or of a new entry, drawn where the entry will be.
pub(in crate::interfaces::studio) fn name_box<'a>(
    edit: &'a Edit,
    depth: u16,
    directory: bool,
) -> Element<'a, Message> {
    let icon = match &edit.kind {
        EditKind::NewFolder(_) => icons::icon("folder", design::brand_blue(), 15.0),
        EditKind::NewFile(_) => icons::icon("file", design::muted(), 15.0),
        EditKind::Rename(_) if directory => icons::icon("folder", design::brand_blue(), 15.0),
        EditKind::Rename(_) => icons::icon("file", design::muted(), 15.0),
    };
    let input = text_input("Nombre…", &edit.text)
        .id(super::INPUT)
        .size(12)
        .padding([3, 6])
        .style(styles::input)
        .on_input(|value| Message::Explorer(Event::EditText(value)))
        .on_submit(Message::Explorer(Event::EditSubmit));
    let mut cell = column![
        row![
            Space::new().width(f32::from(depth) * 12.0 + 17.0),
            icon,
            input
        ]
        .spacing(6)
        .align_y(iced::Center)
    ]
    .spacing(2);
    if let Some(error) = &edit.error {
        cell = cell.push(row![
            Space::new().width(f32::from(depth) * 12.0 + 38.0),
            text(error.clone()).size(11).color(design::danger())
        ]);
    }
    container(cell).padding([2, 6]).into()
}

/// The question before deleting: what goes, and that it can be undone.
pub(in crate::interfaces::studio) fn confirm_bar(
    state: &Dashboard,
) -> Option<Element<'_, Message>> {
    let items = state.explorer.confirm.as_ref()?;
    let what = match items.as_slice() {
        [one] => format!("«{}»", super::name_of(one)),
        many => format!("{} elementos", many.len()),
    };
    let unsaved = state
        .editor
        .tabs
        .iter()
        .filter(|tab| {
            tab.dirty
                && items
                    .iter()
                    .any(|item| tab.document.relative.starts_with(item))
        })
        .count();
    let mut body = column![
        text(format!("¿Eliminar {what}?"))
            .size(12)
            .font(design::UI_BOLD),
        text("Se mueve a la papelera de ChxChx y puedes deshacerlo desde el explorador.")
            .size(11)
            .color(design::muted()),
    ]
    .spacing(4);
    if unsaved > 0 {
        body = body.push(
            text(format!(
                "Hay {unsaved} archivo(s) abierto(s) con cambios sin guardar; se perderán."
            ))
            .size(11)
            .color(design::warning()),
        );
    }
    let buttons = row![
        Space::new().width(Fill),
        button(text("Cancelar").size(12))
            .padding([5, 12])
            .style(styles::secondary)
            .on_press(Message::Explorer(Event::Escape)),
        button(text("Eliminar").size(12))
            .padding([5, 12])
            .style(styles::primary)
            .on_press(Message::Explorer(Event::ConfirmDelete)),
    ]
    .spacing(8);
    Some(
        container(body.push(buttons).spacing(8))
            .padding(10)
            .width(Fill)
            .style(styles::menu_panel)
            .into(),
    )
}

#[cfg(test)]
#[path = "studio_explorer_view_tests.rs"]
mod tests;
