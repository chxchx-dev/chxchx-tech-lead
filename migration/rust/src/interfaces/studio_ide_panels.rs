//! Left sidebar view: the project explorer.
use super::editor::Event;
use super::*;
use iced::widget::{column, text_input, tooltip};
use std::path::Path;

fn heading<'a>(label: &'a str) -> iced::widget::Text<'a> {
    text(label)
        .size(10)
        .font(design::UI_BOLD)
        .color(design::FAINT)
}

pub(super) fn explorer(state: &Dashboard) -> Element<'_, Message> {
    let filter = state.editor.filter.to_lowercase();
    let active = tabs::selected_tab(state).map(|tab| tab.document.relative.as_path());
    let tree = folder_rows(state, Path::new(""), 0, &filter, active, column![]);
    let header = row![
        heading("EXPLORADOR"),
        Space::new().width(Fill),
        styles::tip(
            button(icons::icon("refresh", design::MUTED, 14.0))
                .padding(4)
                .style(styles::ghost(false))
                .on_press(Message::Editor(Event::RefreshTree)),
            "Actualizar árbol",
            tooltip::Position::Bottom,
        ),
    ]
    .align_y(iced::Center);
    let project = row![
        icons::icon("chevron-down", design::MUTED, 12.0),
        text(chrome::project_name(state).to_uppercase())
            .size(11)
            .font(design::UI_BOLD),
    ]
    .spacing(6)
    .align_y(iced::Center);
    container(
        column![
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
            styles::vscroll(container(tree).padding([0, 6])).height(Fill),
        ]
        .spacing(8),
    )
    .padding([10, 0])
    .width(state.shell.explorer_width)
    .height(Fill)
    .style(styles::surface(design::BG_PANEL))
    .into()
}

fn folder_rows<'a>(
    state: &'a Dashboard,
    folder: &Path,
    depth: u16,
    filter: &str,
    active: Option<&Path>,
    mut rows: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
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
            let selected = active == Some(entry.relative.as_path());
            let key = entry.relative.to_string_lossy().replace('\\', "/");
            let badge = state.git.badges.get(&key).copied();
            let changed = entry.directory && state.git.dirty_dirs.contains(&key);
            let tint = badge.map(|letter| match letter {
                'M' => design::WARNING,
                'N' => design::SUCCESS,
                'D' | '!' => design::DANGER,
                _ => design::BRAND_CYAN,
            });
            let (chevron, icon): (Element<'a, Message>, _) = if entry.directory {
                let (arrow, folder_icon) = if expanded {
                    ("chevron-down", "folder-open")
                } else {
                    ("chevron-right", "folder")
                };
                (
                    icons::icon(arrow, design::FAINT, 11.0).into(),
                    icons::icon(folder_icon, design::BRAND_BLUE, 15.0),
                )
            } else {
                let kind = file_types::of(&entry.relative);
                (
                    Space::new().width(11).into(),
                    icons::icon(kind.icon, kind.tint, 15.0),
                )
            };
            rows = rows.push(
                button(
                    row![
                        Space::new().width(f32::from(depth) * 12.0),
                        chevron,
                        icon,
                        text(name)
                            .size(12)
                            .color(tint.unwrap_or(design::TEXT))
                            .wrapping(iced::widget::text::Wrapping::None),
                        Space::new().width(Fill),
                        text(badge.map(String::from).unwrap_or_default())
                            .size(10)
                            .font(design::UI_BOLD)
                            .color(tint.unwrap_or(design::TEXT)),
                        text(if changed { "●" } else { "" })
                            .size(8)
                            .color(design::WARNING),
                    ]
                    .spacing(6)
                    .align_y(iced::Center),
                )
                .width(Fill)
                .padding([4, 6])
                .style(styles::ghost(selected))
                .on_press(Message::Editor(if entry.directory {
                    Event::Folder(entry.relative.clone())
                } else {
                    Event::Open(entry.relative.clone())
                })),
            );
        }
        if entry.directory && expanded {
            rows = folder_rows(state, &entry.relative, depth + 1, filter, active, rows);
        }
    }
    rows
}
