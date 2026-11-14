//! Window chrome: title bar with the menu bar, and the activity rail.
use super::shell::{DockTab, Event as ShellEvent, Side};
use super::*;
use iced::widget::{column, tooltip};

pub(super) fn project_name(state: &Dashboard) -> String {
    state
        .overview
        .as_ref()
        .map(|overview| overview.project.name.clone())
        .ok()
        .or_else(|| {
            state
                .root
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| state.root.display().to_string())
}

pub(super) fn title_bar(state: &Dashboard) -> Element<'_, Message> {
    let mut workspace = row![
        icons::icon("folder", design::BRAND_BLUE, 14.0),
        text(project_name(state)).size(12).font(design::UI_BOLD),
    ]
    .spacing(7)
    .align_y(iced::Center);
    if let Ok(overview) = &state.overview {
        workspace = workspace.push(
            text(format!("·  {}", overview.detection.profile))
                .size(11)
                .color(design::MUTED),
        );
    }
    let brand = container(
        row![
            brand::mark(20.0),
            text("ChxChx").size(13).font(design::UI_BOLD)
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .width(menu::BRAND_WIDTH);
    let bar = row![
        brand,
        menu::bar(state),
        Space::new().width(Fill),
        container(workspace).padding([4, 14]).style(styles::well),
        Space::new().width(Fill),
        layout_toggle(
            "panel-left",
            "Barra lateral (Ctrl+B)",
            state.editor.explorer,
            Message::Shell(ShellEvent::ToggleSide(state.shell.side)),
        ),
        layout_toggle(
            "panel-bottom",
            "Panel inferior (Ctrl+J)",
            shell::dock_tab(state).is_some(),
            Message::Shell(ShellEvent::ToggleDock),
        ),
    ]
    .spacing(6)
    .align_y(iced::Center);
    container(bar)
        .padding([0, 12])
        .height(menu::BAR_HEIGHT)
        .align_y(iced::Center)
        .width(Fill)
        .style(styles::surface(design::BG_DEEP))
        .into()
}

fn layout_toggle<'a>(
    icon: &'static str,
    label: &'a str,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    let color = if active {
        design::ACCENT
    } else {
        design::MUTED
    };
    styles::tip(
        button(icons::icon(icon, color, 16.0))
            .padding(5)
            .style(styles::ghost(false))
            .on_press(message),
        label,
        tooltip::Position::Bottom,
    )
}

/// Left rail: one choice for the sidebar (explorer, search or workspace) and one for the bottom
/// panel. At most one item per group is marked, and clicking it again hides the panel.
pub(super) fn activity_rail(state: &Dashboard) -> Element<'_, Message> {
    let side = |side: Side| state.editor.explorer && state.shell.side == side;
    let dock = shell::dock_tab(state);
    let rail = column![
        rail_action(
            "folder",
            "Explorador (Ctrl+B)",
            side(Side::Explorer),
            Message::Shell(ShellEvent::ToggleSide(Side::Explorer)),
        ),
        rail_action(
            "search",
            "Buscar en el proyecto (Ctrl+Shift+F)",
            side(Side::Search),
            Message::Shell(ShellEvent::ToggleSide(Side::Search)),
        ),
        git_action(state),
        agents_action(state),
        rail_action(
            "projects",
            "Espacio de trabajo: secciones del proyecto",
            side(Side::Workspace),
            Message::Shell(ShellEvent::ToggleSide(Side::Workspace)),
        ),
        Space::new().height(Fill),
        rail_action(
            "terminal",
            "Terminal (Ctrl+J)",
            dock == Some(DockTab::Terminal),
            Message::Shell(ShellEvent::ToggleTab(DockTab::Terminal)),
        ),
        rail_action(
            "bell",
            "Actividad",
            dock == Some(DockTab::Activity),
            Message::Shell(ShellEvent::ToggleTab(DockTab::Activity)),
        ),
    ]
    .spacing(2);
    container(rail)
        .padding([8, 0])
        .width(46)
        .height(Fill)
        .style(styles::surface(design::BG_DEEP))
        .into()
}

fn rail_action<'a>(
    icon: &'static str,
    label: &'a str,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    let color = if active { design::TEXT } else { design::FAINT };
    let marker = container(Space::new())
        .width(2)
        .height(22)
        .style(styles::surface(if active {
            design::ACCENT
        } else {
            iced::Color::TRANSPARENT
        }));
    let action = button(container(icons::icon(icon, color, 20.0)).center_x(Fill))
        .width(Fill)
        .padding([8, 0])
        .style(styles::ghost(false))
        .on_press(message);
    styles::tip(
        row![marker, action, Space::new().width(2)].align_y(iced::Center),
        label,
        tooltip::Position::Right,
    )
}

/// Rail button for the agents; a dot says one is running in the terminal.
fn agents_action(state: &Dashboard) -> Element<'_, Message> {
    let active = state.editor.explorer && state.shell.side == Side::Agents;
    let base = rail_action(
        "sparkle",
        "Agentes: abrir Claude Code, Codex u OpenCode",
        active,
        Message::Shell(ShellEvent::ToggleSide(Side::Agents)),
    );
    let running = state
        .terminal
        .tabs
        .iter()
        .any(|tab| matches!(tab.kind, terminal::Kind::Agent(_)) && !tab.exited);
    if !running {
        return base;
    }
    iced::widget::stack![
        base,
        container(icons::icon("dot", design::SUCCESS, 10.0))
            .align_right(Fill)
            .align_top(Fill)
            .padding([6, 8])
    ]
    .into()
}

/// Rail button for Git, with the number of changed files on it.
fn git_action(state: &Dashboard) -> Element<'_, Message> {
    let active = state.editor.explorer && state.shell.side == Side::Git;
    let count = state.git.change_count();
    let base = rail_action(
        "branch",
        "Git: cambios, historial y ramas",
        active,
        Message::Shell(ShellEvent::ToggleSide(Side::Git)),
    );
    if count == 0 {
        return base;
    }
    let badge = container(
        text(count.min(99).to_string())
            .size(8)
            .font(design::UI_BOLD),
    )
    .padding([0, 4])
    .style(styles::brand_fill);
    iced::widget::stack![
        base,
        container(badge)
            .align_right(Fill)
            .align_top(Fill)
            .padding([4, 3])
    ]
    .into()
}
