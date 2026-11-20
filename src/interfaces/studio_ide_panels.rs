//! Left sidebar view: the project explorer.
use super::editor::Event;
use super::*;
use iced::widget::{column, mouse_area, text_input, tooltip};
use std::path::Path;

fn heading<'a>(label: &'a str) -> iced::widget::Text<'a> {
    text(label)
        .size(10)
        .font(design::UI_BOLD)
        .color(design::faint())
}

pub(super) fn explorer(state: &Dashboard) -> Element<'_, Message> {
    iced::widget::responsive(move |size| explorer_panel(state, size)).into()
}

fn header_button<'a>(
    icon: &'static str,
    tip: &'static str,
    message: Message,
) -> Element<'a, Message> {
    styles::tip(
        button(icons::icon(icon, design::muted(), 14.0))
            .padding(4)
            .style(styles::ghost(false))
            .on_press(message),
        tip,
        tooltip::Position::Bottom,
    )
}

fn explorer_panel(state: &Dashboard, size: iced::Size) -> Element<'_, Message> {
    use explorer::{Action, Event as Explore, Target};
    let filter = state.editor.filter.to_lowercase();
    let active = tabs::selected_tab(state).map(|tab| tab.document.relative.as_path());
    let mut tree = folder_rows(state, Path::new(""), 0, &filter, active, column![]);
    // A new entry at the top level has no folder row to hang from: its box leads the tree.
    let mut header = row![
        heading("EXPLORADOR"),
        Space::new().width(Fill),
        header_button(
            "file-plus",
            "Nuevo archivo",
            Message::Explorer(Explore::Do(Action::NewFile))
        ),
        header_button(
            "folder-plus",
            "Nueva carpeta",
            Message::Explorer(Explore::Do(Action::NewFolder))
        ),
    ]
    .align_y(iced::Center);
    if explorer::undo_depth(state) > 0 {
        header = header.push(header_button(
            "undo",
            "Deshacer la última eliminación",
            Message::Explorer(Explore::Do(Action::UndoDelete)),
        ));
    }
    header = header
        .push(header_button(
            "minus",
            "Contraer todo",
            Message::Explorer(Explore::Do(Action::CollapseAll)),
        ))
        .push(header_button(
            "refresh",
            "Actualizar árbol",
            Message::Explorer(Explore::Do(Action::Refresh)),
        ));
    let project = mouse_area(
        row![
            icons::icon("chevron-down", design::muted(), 12.0),
            text(chrome::project_name(state).to_uppercase())
                .size(11)
                .font(design::UI_BOLD),
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .on_press(Message::Explorer(Explore::ClickRoot))
    .on_right_press(Message::Explorer(Explore::Context(Target::Root)));
    let empty = state
        .editor
        .folders
        .get(Path::new(""))
        .is_some_and(Vec::is_empty)
        && state.explorer.edit.is_none();
    if empty {
        tree = tree.push(
            container(
                text("La carpeta está vacía. Clic derecho para crear algo.")
                    .size(11)
                    .color(design::faint()),
            )
            .padding([6, 10]),
        );
    }
    let scroll = mouse_area(styles::vscroll(container(tree).padding([0, 6])).height(Fill))
        .on_press(Message::Explorer(Explore::ClickRoot))
        .on_right_press(Message::Explorer(Explore::Context(Target::Root)));
    let mut body = column![
        container(header).padding([0, 12]),
        container(
            text_input("Filtrar archivos…", &state.editor.filter)
                .size(12)
                .padding([5, 8])
                .style(styles::input)
                .on_input(|value| Message::Editor(Event::Filter(value)))
        )
        .padding([0, 10]),
        container(project).padding([2, 10]),
        scroll,
    ]
    .spacing(8);
    if let Some(confirm) = explorer::view::confirm_bar(state) {
        body = body.push(container(confirm).padding([0, 8]));
    }
    let panel = container(body)
        .padding([10, 0])
        .width(state.shell.explorer_width)
        .height(Fill)
        .style(styles::surface(design::bg_panel()));
    let tracked = mouse_area(panel).on_move(|point| Message::Explorer(Explore::Pointer(point)));
    match explorer::view::context_menu(state, size) {
        Some(menu) => iced::widget::stack![tracked, menu].into(),
        None => tracked.into(),
    }
}

fn folder_rows<'a>(
    state: &'a Dashboard,
    folder: &Path,
    depth: u16,
    filter: &str,
    active: Option<&Path>,
    mut rows: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    use explorer::{EditKind, Event as Explore, Target};
    let edit = state.explorer.edit.as_ref();
    // A new file or folder is typed where it will appear: at the top of its folder.
    if let Some(edit) = edit
        && matches!(&edit.kind, EditKind::NewFile(parent) | EditKind::NewFolder(parent) if parent == folder)
    {
        rows = rows.push(explorer::view::name_box(
            edit,
            depth,
            matches!(edit.kind, EditKind::NewFolder(_)),
        ));
    }
    let Some(entries) = state.editor.folders.get(folder).filter(|_| depth <= 20) else {
        return rows;
    };
    for entry in entries {
        let name = entry
            .relative
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let expanded = state.editor.expanded.contains(&entry.relative);
        if entry.directory || name.to_lowercase().contains(filter) {
            if let Some(edit) = edit
                && matches!(&edit.kind, EditKind::Rename(path) if *path == entry.relative)
            {
                rows = rows.push(explorer::view::name_box(edit, depth, entry.directory));
                if entry.directory && expanded {
                    rows = folder_rows(state, &entry.relative, depth + 1, filter, active, rows);
                }
                continue;
            }
            let selected = state.explorer.selection.contains(&entry.relative)
                || (state.explorer.selection.is_empty()
                    && active == Some(entry.relative.as_path()));
            let cut = state
                .explorer
                .clipboard
                .as_ref()
                .is_some_and(|clip| clip.cut && clip.paths.contains(&entry.relative));
            let key = entry.relative.to_string_lossy().replace('\\', "/");
            let badge = state.git.badges.get(&key).copied();
            let changed = entry.directory && state.git.dirty_dirs.contains(&key);
            let tint = badge.map(|letter| match letter {
                'M' => design::warning(),
                'N' => design::success(),
                'D' | '!' => design::danger(),
                _ => design::brand_cyan(),
            });
            let (chevron, icon): (Element<'a, Message>, _) = if entry.directory {
                let (arrow, folder_icon) = if expanded {
                    ("chevron-down", "folder-open")
                } else {
                    ("chevron-right", "folder")
                };
                (
                    icons::icon(arrow, design::faint(), 11.0).into(),
                    icons::icon(folder_icon, design::brand_blue(), 15.0),
                )
            } else {
                let kind = file_types::of(&entry.relative);
                (
                    Space::new().width(11).into(),
                    icons::icon(kind.icon, kind.tint, 15.0),
                )
            };
            let label_color = if cut {
                design::faint()
            } else {
                tint.unwrap_or(design::fg())
            };
            rows = rows.push(
                mouse_area(
                    button(
                        row![
                            Space::new().width(f32::from(depth) * 12.0),
                            chevron,
                            icon,
                            text(name)
                                .size(12)
                                .color(label_color)
                                .wrapping(iced::widget::text::Wrapping::None),
                            Space::new().width(Fill),
                            text(badge.map(String::from).unwrap_or_default())
                                .size(10)
                                .font(design::UI_BOLD)
                                .color(tint.unwrap_or(design::fg())),
                            text(if changed { "●" } else { "" })
                                .size(8)
                                .color(design::warning()),
                        ]
                        .spacing(6)
                        .align_y(iced::Center),
                    )
                    .width(Fill)
                    .padding([4, 6])
                    .style(styles::ghost(selected))
                    .on_press(Message::Explorer(Explore::Click(
                        entry.relative.clone(),
                        entry.directory,
                    ))),
                )
                .on_right_press(Message::Explorer(Explore::Context(
                    Target::Item {
                        path: entry.relative.clone(),
                        directory: entry.directory,
                    },
                ))),
            );
        }
        if entry.directory && expanded {
            rows = folder_rows(state, &entry.relative, depth + 1, filter, active, rows);
        }
    }
    rows
}
