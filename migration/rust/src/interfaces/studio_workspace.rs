//! "Espacio de trabajo": the sections of the project, grouped, in the left sidebar.
use super::editor::Event;
use super::*;
use iced::widget::column;

/// One entry of a group: icon, label and the page it opens.
type Entry = (&'static str, &'static str, Page);

/// Sections shown in the sidebar, by group.
const GROUPS: &[(&str, &[Entry])] = &[
    (
        "PROYECTO",
        &[
            ("home", "Inicio", Page::Home),
            ("setup", "Proyecto y permisos", Page::Setup),
            ("projects", "Proyectos", Page::Projects),
        ],
    ),
    (
        "AGENTES Y HERRAMIENTAS",
        &[
            ("layers", "Catálogo: skills, packs, mods", Page::Catalog),
            ("integrations", "Herramientas / MCP", Page::Integrations),
        ],
    ),
    (
        "SALUD",
        &[("diagnostics", "Diagnóstico", Page::Diagnostics)],
    ),
];

const DOCUMENTS: &[(&str, &str)] = &[
    ("Estado del proyecto", ".ai/CURRENT_STATE.md"),
    ("Handoff", ".ai/HANDOFF.md"),
    ("Reglas del proyecto", "AGENTS.md"),
    ("Guía del producto", "README.md"),
];

fn heading<'a>(label: &'a str) -> Element<'a, Message> {
    container(
        text(label)
            .size(10)
            .font(design::UI_BOLD)
            .color(design::FAINT),
    )
    .padding([8, 8])
    .into()
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let mut list = column![project_card(state)].spacing(1);
    for (group, entries) in GROUPS {
        list = list.push(heading(group));
        for (icon, label, page) in *entries {
            let active = state.page == *page && state.editor.home;
            let tint = if active {
                design::ACCENT
            } else {
                design::MUTED
            };
            list = list.push(link(icon, tint, label, active, Message::Navigate(*page)));
        }
    }
    list = list.push(heading("CONTINUIDAD"));
    for (label, path) in DOCUMENTS {
        list = list.push(link(
            "file-text",
            design::BRAND_CYAN,
            label,
            false,
            Message::Editor(Event::Open(PathBuf::from(path))),
        ));
    }
    container(
        column![
            container(
                text("ESPACIO DE TRABAJO")
                    .size(10)
                    .font(design::UI_BOLD)
                    .color(design::FAINT)
            )
            .padding([0, 12]),
            styles::vscroll(container(list).padding([0, 6])).height(Fill),
            container(pending()).padding([0, 10]),
        ]
        .spacing(8),
    )
    .padding([10, 0])
    .width(state.shell.explorer_width)
    .height(Fill)
    .style(styles::surface(design::BG_PANEL))
    .into()
}

/// Project name, profile and the two facts that gate everything else.
fn project_card(state: &Dashboard) -> Element<'_, Message> {
    let mut card = column![
        row![
            icons::icon("folder", design::BRAND_BLUE, 15.0),
            text(chrome::project_name(state))
                .size(13)
                .font(design::UI_BOLD)
        ]
        .spacing(8)
        .align_y(iced::Center),
    ]
    .spacing(8);
    if let Ok(overview) = &state.overview {
        let trusted = overview.workspace.trusted;
        let fact = |icon: &'static str, tint: iced::Color, label: &'static str| {
            row![
                icons::icon(icon, tint, 12.0),
                text(label).size(11).color(design::MUTED)
            ]
            .spacing(6)
            .align_y(iced::Center)
        };
        card = card
            .push(
                text(format!("Perfil {}", overview.detection.profile))
                    .size(11)
                    .color(design::MUTED),
            )
            .push(if trusted {
                fact("shield", design::SUCCESS, "Proyecto de confianza")
            } else {
                fact("warning", design::WARNING, "Permisos por revisar")
            })
            .push(if overview.project.git_repository {
                fact("branch", design::MUTED, "Repositorio Git")
            } else {
                fact("branch", design::FAINT, "Sin repositorio Git")
            });
    } else if state.overview_loading {
        card = card.push(text("Leyendo el proyecto…").size(11).color(design::MUTED));
    }
    container(card)
        .padding(12)
        .width(Fill)
        .style(styles::card)
        .into()
}

fn pending<'a>() -> Element<'a, Message> {
    container(
        row![
            icons::icon("sparkle", design::ACCENT, 13.0),
            text("Agentes y Git: port pendiente")
                .size(11)
                .color(design::MUTED)
        ]
        .spacing(7)
        .align_y(iced::Center),
    )
    .padding([8, 10])
    .width(Fill)
    .style(styles::well)
    .into()
}

fn link<'a>(
    icon: &'static str,
    tint: iced::Color,
    label: &'a str,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    button(
        row![icons::icon(icon, tint, 15.0), text(label).size(12)]
            .spacing(9)
            .align_y(iced::Center),
    )
    .width(Fill)
    .padding([6, 8])
    .style(styles::ghost(active))
    .on_press(message)
    .into()
}
