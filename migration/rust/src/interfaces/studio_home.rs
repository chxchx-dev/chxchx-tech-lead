//! Welcome content: brand hero, project summary and detected technologies.
use super::pages::{config_status, eyebrow};
use super::*;
use iced::widget::column;

pub(super) fn hero<'a>() -> Element<'a, Message> {
    container(
        row![
            brand::logo(124.0),
            column![
                container(
                    text(concat!(
                        "ENTORNO DE DESARROLLO  ·  v",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .size(10)
                    .font(design::UI_BOLD)
                )
                .padding([3, 10])
                .style(styles::chip(design::BRAND_CYAN)),
                text("ChxChx Studio").size(30).font(design::UI_BOLD),
                text("Prepara, conecta y controla tus proyectos asistidos por agentes desde un solo lugar.")
                    .size(14)
                    .color(design::MUTED),
            ]
            .spacing(10),
        ]
        .spacing(26)
        .align_y(iced::Center),
    )
    .padding([22, 28])
    .width(Fill)
    .style(styles::hero)
    .into()
}

pub(super) fn project_card<'a>(
    state: &'a Dashboard,
    project: &'a ProjectOverview,
) -> Element<'a, Message> {
    let config = config_status(project);
    let header = row![
        column![
            eyebrow("PROYECTO ACTUAL"),
            text(&project.project.name).size(20).font(design::UI_BOLD),
            text(state.root.display().to_string())
                .size(12)
                .font(design::MONO)
                .color(design::MUTED),
        ]
        .spacing(5),
        Space::new().width(Fill),
        button(
            row![
                icons::icon("refresh", design::MUTED, 14.0),
                text("Actualizar").size(12)
            ]
            .spacing(7)
            .align_y(iced::Center)
        )
        .padding([7, 12])
        .style(styles::secondary)
        .on_press(Message::Refresh),
    ];
    container(
        column![
            header,
            row![
                indicator("Configuración", config, config == "Lista"),
                indicator(
                    "Permisos locales",
                    if project.workspace.trusted {
                        "Confianza concedida"
                    } else {
                        "Pendiente de revisión"
                    },
                    project.workspace.trusted
                ),
                indicator(
                    "Control de versiones",
                    if project.project.git_repository {
                        "Git disponible"
                    } else {
                        "Sin repositorio Git"
                    },
                    project.project.git_repository
                ),
            ]
            .spacing(12),
        ]
        .spacing(16),
    )
    .padding(20)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn indicator<'a>(label: &'a str, value: &'a str, ready: bool) -> Element<'a, Message> {
    let tint = if ready {
        design::SUCCESS
    } else {
        design::WARNING
    };
    container(
        column![
            text(label).size(11).color(design::MUTED),
            row![icons::icon("dot", tint, 11.0), text(value).size(13)]
                .spacing(6)
                .align_y(iced::Center),
        ]
        .spacing(6),
    )
    .padding([10, 14])
    .width(Fill)
    .style(styles::well)
    .into()
}

pub(super) fn technologies(project: &ProjectOverview) -> Element<'_, Message> {
    let detection = &project.detection;
    container(
        column![
            row![
                icons::icon("layers", design::ACCENT, 16.0),
                text("Tecnologías del proyecto")
                    .size(15)
                    .font(design::UI_BOLD),
                Space::new().width(Fill),
                container(text(format!("Perfil · {}", detection.profile)).size(11))
                    .padding([3, 10])
                    .style(styles::chip(design::BRAND_CYAN)),
            ]
            .spacing(9)
            .align_y(iced::Center),
            group("Stacks", &detection.stacks, design::BRAND_BLUE),
            group("Lenguajes", &detection.languages, design::TINT_PURPLE),
            group("Gestores", &detection.package_managers, design::TINT_GREEN),
        ]
        .spacing(12),
    )
    .padding(20)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn group<'a>(label: &'a str, values: &'a [String], tint: iced::Color) -> Element<'a, Message> {
    let mut chips = iced::widget::Row::new().spacing(6);
    if values.is_empty() {
        chips = chips.push(text("Sin detectar").size(12).color(design::FAINT));
    }
    for value in values {
        chips = chips.push(
            container(text(value).size(11))
                .padding([3, 10])
                .style(styles::chip(tint)),
        );
    }
    row![
        text(label).size(12).color(design::MUTED).width(84),
        chips.wrap()
    ]
    .spacing(8)
    .align_y(iced::Center)
    .into()
}

/// Recent files and projects remembered between sessions.
pub(super) fn recents(state: &Dashboard) -> Option<Element<'_, Message>> {
    let files = state.session.recent_files();
    let current = state.root.display().to_string();
    let projects: Vec<&String> = state
        .session
        .recent_projects()
        .iter()
        .filter(|path| **path != current)
        .take(5)
        .collect();
    if files.is_empty() && projects.is_empty() {
        return None;
    }
    let mut file_list = iced::widget::column![eyebrow("ARCHIVOS RECIENTES")].spacing(4);
    for path in files.iter().take(6) {
        let kind = file_types::of(std::path::Path::new(path));
        file_list = file_list.push(entry(
            kind.icon,
            kind.tint,
            path,
            Message::Editor(editor::Event::Open(PathBuf::from(path))),
        ));
    }
    if files.is_empty() {
        file_list = file_list.push(text("Aún no abres archivos.").size(12).color(design::FAINT));
    }
    let mut project_list = iced::widget::column![eyebrow("PROYECTOS RECIENTES")].spacing(4);
    for path in &projects {
        project_list = project_list.push(entry(
            "folder",
            design::BRAND_BLUE,
            path,
            Message::Session(session::Event::OpenProject((*path).clone())),
        ));
    }
    if projects.is_empty() {
        project_list = project_list.push(
            text("Sin otros proyectos todavía.")
                .size(12)
                .color(design::FAINT),
        );
    }
    Some(
        container(row![file_list.width(Fill), project_list.width(Fill)].spacing(24))
            .padding(20)
            .width(Fill)
            .style(styles::card)
            .into(),
    )
}

fn entry<'a>(
    icon: &'static str,
    tint: iced::Color,
    label: &'a str,
    message: Message,
) -> Element<'a, Message> {
    button(
        row![icons::icon(icon, tint, 14.0), text(label).size(12)]
            .spacing(8)
            .align_y(iced::Center),
    )
    .width(Fill)
    .padding([5, 6])
    .style(styles::ghost(false))
    .on_press(message)
    .into()
}
