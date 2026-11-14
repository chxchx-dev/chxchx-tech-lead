//! Home tab: welcome page and the operational sections of the workspace.
use super::pages::*;
use super::*;
use iced::widget::column;

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let mut body = column![].spacing(16).max_width(960);
    body = body.push(match state.page {
        Page::Home => home::hero(),
        Page::Catalog => catalog::header(),
        page => page_header(page),
    });
    if state.page == Page::Catalog {
        body = body.push(catalog::view(state));
    }
    match &state.overview {
        _ if state.page == Page::Catalog => {}
        Ok(project) => {
            body = body.push(home::project_card(state, project));
            if let Some(error) = &project.workspace.config_validation_error {
                body = body.push(banner(
                    "warning",
                    design::WARNING,
                    format!("Revisa la configuración: {error}"),
                ));
            }
            if state.page == Page::Home {
                body = body
                    .push(next_step(project))
                    .push(agents::home_card(state))
                    .push(
                        row![
                            card(
                                "01 / ORGANIZA",
                                "Un lugar para tus proyectos",
                                "Consulta los proyectos que ya tienes registrados.",
                                "Ver proyectos",
                                Message::Navigate(Page::Projects)
                            ),
                            card(
                                "02 / CONECTA",
                                "Tus herramientas, juntas",
                                "Prepara las conexiones de tus asistentes.",
                                "Ver integraciones",
                                Message::Navigate(Page::Integrations)
                            ),
                        ]
                        .spacing(16),
                    )
                    .push(home::technologies(project));
                if let Some(recents) = home::recents(state) {
                    body = body.push(recents);
                }
            }
        }
        Err(_) if state.overview_loading => {
            body = body.push(banner(
                "refresh",
                design::ACCENT,
                "Leyendo el proyecto…".to_owned(),
            ));
        }
        Err(error) => {
            body = body.push(banner(
                "warning",
                design::DANGER,
                format!("No se pudo leer el proyecto: {error}"),
            ));
        }
    }
    body = page_actions(state, body);
    if has_review(state) {
        body = body.push(
            container(
                column![
                    row![
                        icons::icon("shield", design::ACCENT, 18.0),
                        text("Revisa antes de continuar")
                            .size(16)
                            .font(design::UI_BOLD),
                        Space::new().width(Fill),
                        button(text("Cerrar revisión").size(12))
                            .padding([6, 12])
                            .style(styles::secondary)
                            .on_press(Message::Dismiss)
                    ]
                    .spacing(9)
                    .align_y(iced::Center),
                    review::view(state),
                ]
                .spacing(16),
            )
            .padding(20)
            .width(Fill)
            .style(styles::card),
        );
    }
    container(styles::vscroll(container(body).center_x(Fill).padding([22, 28])).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(styles::surface(design::BG_BASE))
        .into()
}

fn page_header<'a>(page: Page) -> Element<'a, Message> {
    let (icon, title, subtitle) = match page {
        Page::Home => ("home", "Tu espacio de trabajo", ""),
        Page::Setup => (
            "setup",
            "Prepara tu proyecto",
            "Configura tu espacio con cambios que puedes revisar.",
        ),
        Page::Catalog => ("layers", "Catálogo del proyecto", ""),
        Page::Projects => (
            "projects",
            "Tus proyectos",
            "Consulta y mantén tus proyectos registrados.",
        ),
        Page::Integrations => (
            "integrations",
            "Conecta tus herramientas",
            "Configura la conexión con tus asistentes de desarrollo.",
        ),
        Page::Diagnostics => (
            "diagnostics",
            "Diagnóstico",
            "Comprueba qué está listo y qué necesita atención.",
        ),
    };
    row![
        badge(icon, 22.0),
        column![
            text(title).size(22).font(design::UI_BOLD),
            text(subtitle).size(13).color(design::MUTED)
        ]
        .spacing(4),
    ]
    .spacing(14)
    .align_y(iced::Center)
    .into()
}

fn next_step(project: &ProjectOverview) -> Element<'_, Message> {
    let (heading, description, label, action) = if !project.workspace.config_exists {
        (
            "Dale un buen inicio a tu proyecto",
            "Prepara la configuración y las guías para trabajar con tus asistentes.",
            "Preparar proyecto",
            Message::PreviewInit { minimal: false },
        )
    } else if !project.workspace.trusted {
        (
            "Revisa los permisos del proyecto",
            "Concede confianza local cuando hayas revisado la configuración.",
            "Revisar confianza",
            Message::PreviewTrust { desired: true },
        )
    } else {
        (
            "Comprueba que todo esté listo",
            "Revisa las herramientas y conexiones antes de empezar a trabajar.",
            "Comprobar proyecto",
            Message::Doctor,
        )
    };
    feature_card("SIGUIENTE PASO", heading, description, label, action)
}
