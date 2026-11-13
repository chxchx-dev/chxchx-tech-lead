use super::*;
use iced::widget::column;

pub(super) fn page_actions<'a>(
    state: &'a Dashboard,
    mut body: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    let cards = match state.page {
        Page::Home | Page::Catalog => return body,
        Page::Setup => column![
            card(
                "RECOMENDADO",
                "Preparación completa",
                "Crea configuración y guías. Revisa los archivos antes de aplicar.",
                "Revisar preparación",
                Message::PreviewInit { minimal: false }
            ),
            card(
                "ESENCIAL",
                "Preparación compacta",
                "Empieza con la configuración mínima y amplía después.",
                "Revisar versión compacta",
                Message::PreviewInit { minimal: true }
            ),
            card(
                "PERMISOS",
                "Confianza del proyecto",
                "Revisa la autorización local para las operaciones del proyecto.",
                "Revisar confianza",
                Message::PreviewTrust { desired: true }
            ),
            button(text("Revisar revocación de confianza").size(12))
                .padding([8, 14])
                .style(styles::danger)
                .on_press(Message::PreviewTrust { desired: false }),
            card(
                "RECUPERACIÓN",
                "Restaurar un respaldo",
                "Revisa los archivos que volverán a su estado anterior.",
                "Revisar restauración",
                Message::PreviewRollback
            ),
        ],
        Page::Projects => column![
            card(
                "CATÁLOGO",
                "Proyectos registrados",
                "Consulta las rutas y la disponibilidad de tus proyectos.",
                "Cargar proyectos",
                Message::Projects
            ),
            card(
                "MANTENIMIENTO",
                "Sincronizar proyectos",
                "Revisa las actualizaciones de las guías administradas.",
                "Revisar sincronización",
                Message::PreviewSync
            ),
        ],
        Page::Integrations => column![card(
            "ASISTENTES",
            "Conexiones MCP",
            "Conecta tus clientes con las herramientas del proyecto. Podrás revisar comandos y alcance antes de aplicar.",
            "Revisar conexiones",
            Message::PreviewMcp
        )],
        Page::Diagnostics => column![card(
            "SALUD DEL PROYECTO",
            "Comprueba tu entorno",
            "Consulta herramientas, configuración y conexiones con una revisión de solo lectura.",
            "Ejecutar diagnóstico",
            Message::Doctor
        )],
    };
    body = body.push(cards.spacing(16));
    body
}

pub(super) fn eyebrow<'a>(label: &'a str) -> iced::widget::Text<'a> {
    text(label)
        .size(10)
        .font(design::UI_BOLD)
        .color(design::ACCENT)
}

/// Round tinted badge that holds a section icon.
pub(super) fn badge<'a>(icon: &'static str, size: f32) -> Element<'a, Message> {
    container(icons::icon(icon, design::BRAND_CYAN, size))
        .padding(size * 0.45)
        .style(styles::chip(design::BRAND_BLUE))
        .into()
}

/// Inline message with an icon, for warnings and read errors.
pub(super) fn banner<'a>(
    icon: &'static str,
    tint: iced::Color,
    message: String,
) -> Element<'a, Message> {
    container(
        row![icons::icon(icon, tint, 16.0), text(message).size(13)]
            .spacing(10)
            .align_y(iced::Center),
    )
    .padding([12, 16])
    .width(Fill)
    .style(styles::card)
    .into()
}

pub(super) fn card<'a>(
    eyebrow: &'a str,
    title: &'a str,
    description: &'a str,
    label: &'a str,
    action: Message,
) -> Element<'a, Message> {
    action_card(eyebrow, title, description, label, action, false)
}

/// The highlighted card for the recommended next step.
pub(super) fn feature_card<'a>(
    eyebrow: &'a str,
    title: &'a str,
    description: &'a str,
    label: &'a str,
    action: Message,
) -> Element<'a, Message> {
    action_card(eyebrow, title, description, label, action, true)
}

fn action_card<'a>(
    label_top: &'a str,
    title: &'a str,
    description: &'a str,
    label: &'a str,
    action: Message,
    featured: bool,
) -> Element<'a, Message> {
    let icon = match title {
        "Preparación completa" | "Preparación compacta" => "setup",
        "Confianza del proyecto" | "Revisa los permisos del proyecto" => "shield",
        "Restaurar un respaldo" => "undo",
        "Proyectos registrados" | "Un lugar para tus proyectos" => "projects",
        "Sincronizar proyectos" => "refresh",
        "Conexiones MCP" | "Tus herramientas, juntas" => "integrations",
        "Comprueba tu entorno" | "Comprueba que todo esté listo" => "diagnostics",
        "Dale un buen inicio a tu proyecto" => "sparkle",
        _ => "activity",
    };
    let call_to_action = button(
        row![
            text(label).size(12).font(design::UI_BOLD),
            icons::icon("arrow-right", design::TEXT, 13.0)
        ]
        .spacing(8)
        .align_y(iced::Center),
    )
    .padding([8, 14])
    .on_press(action);
    let content = column![
        row![
            badge(icon, 18.0),
            column![
                eyebrow(label_top),
                text(title).size(16).font(design::UI_BOLD)
            ]
            .spacing(3)
        ]
        .spacing(12)
        .align_y(iced::Center),
        text(description).size(13).color(design::MUTED),
        if featured {
            call_to_action.style(styles::primary)
        } else {
            call_to_action.style(styles::secondary)
        },
    ]
    .spacing(13);
    let surface: fn(&iced::Theme) -> container::Style =
        if featured { styles::hero } else { styles::card };
    container(content)
        .padding(20)
        .width(Fill)
        .style(surface)
        .into()
}

pub(super) fn has_review(state: &Dashboard) -> bool {
    state.init_preview.is_some()
        || state.sync_preview.is_some()
        || state.rollback_preview.is_some()
        || state.trust_preview.is_some()
        || state.mcp_preview.is_some()
        || (state.page == Page::Projects && state.projects_catalog.is_some())
        || (state.page == Page::Diagnostics && state.doctor_report.is_some())
}

pub(super) fn config_status(project: &ProjectOverview) -> &'static str {
    match (
        project.workspace.config_exists,
        project.workspace.config_parseable,
        project.workspace.config_supported,
    ) {
        (false, _, _) => "Por preparar",
        (true, Some(false), _) => "Revisar formato TOML",
        (true, Some(true), Some(false)) => "Revisar compatibilidad",
        (true, Some(true), Some(true)) => "Lista",
        _ => "Pendiente de validar",
    }
}
