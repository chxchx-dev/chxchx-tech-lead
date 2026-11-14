use super::*;
use iced::widget::column;

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let mut content = column![
        text("Comprueba el alcance antes de confirmar.")
            .size(13)
            .color(design::MUTED)
    ]
    .spacing(9);
    if let Some(catalog) = state
        .projects_catalog
        .as_ref()
        .filter(|_| state.page == Page::Projects)
    {
        content = content.push(title("Proyectos registrados"));
        if catalog.projects.is_empty() {
            content = content.push(note("No hay proyectos registrados."));
        } else {
            for project in &catalog.projects {
                let status = match (project.available, project.is_current) {
                    (true, true) => "disponible · último",
                    (true, false) => "disponible",
                    (false, _) => "no encontrado",
                };
                let tint = if project.available {
                    design::SUCCESS
                } else {
                    design::WARNING
                };
                let stacks = if project.stacks.is_empty() {
                    "ninguno".to_owned()
                } else {
                    project.stacks.join(", ")
                };
                content = content.push(
                    container(
                        column![
                            row![
                                icons::icon("folder", design::BRAND_BLUE, 15.0),
                                text(&project.alias).size(13).font(design::UI_BOLD),
                                text(format!("{} · {} · {stacks}", project.name, project.profile))
                                    .size(12)
                                    .color(design::MUTED),
                                Space::new().width(Fill),
                                container(text(status).size(11))
                                    .padding([2, 9])
                                    .style(styles::chip(tint)),
                            ]
                            .spacing(9)
                            .align_y(iced::Center),
                            text(project.path.as_deref().unwrap_or("<ruta ausente>"))
                                .size(11)
                                .font(design::MONO)
                                .color(design::FAINT),
                        ]
                        .spacing(5),
                    )
                    .padding([10, 14])
                    .width(Fill)
                    .style(styles::well),
                );
            }
        }
        if let Some(current) = &catalog.current {
            content = content.push(note_owned(format!(
                "Último proyecto: {} · {}",
                current.alias, current.path
            )));
        }
    }
    if let Some(plan) = &state.init_preview {
        content = content.push(title(format!("Vista previa para {}", plan.project_root)));
        if let Some(reason) = &plan.blocked_reason {
            content = content.push(line(
                "warning",
                design::DANGER,
                format!("No se puede aplicar: {reason}"),
            ));
        } else {
            for action in &plan.actions {
                content = content.push(line("check", design::SUCCESS, action));
            }
            for warning in &plan.warnings {
                content = content.push(line(
                    "warning",
                    design::WARNING,
                    format!("Aviso: {warning}"),
                ));
            }
            if plan
                .actions
                .iter()
                .any(|action| action != "todo está actualizado")
            {
                if state.init_busy {
                    content = content.push(note("Aplicando init en segundo plano…"));
                } else {
                    content = content.push(confirm("Confirmar preparación", Message::ApplyInit));
                }
            }
        }
    }
    if let Some(plan) = &state.sync_preview {
        content = content.push(title("Vista previa de sincronización de proyectos"));
        for entry in &plan.entries {
            content = content.push(title(format!("{} · {}", entry.alias, entry.project_root)));
            if let Some(reason) = &entry.blocked_reason {
                content = content.push(line(
                    "warning",
                    design::WARNING,
                    format!("Omitido: {reason}"),
                ));
            } else if entry.changes.is_empty() {
                content = content.push(note("Sin cambios."));
            } else {
                for action in &entry.changes {
                    content = content.push(line("check", design::SUCCESS, action));
                }
            }
        }
        for warning in &plan.warnings {
            content = content.push(line(
                "warning",
                design::WARNING,
                format!("Aviso: {warning}"),
            ));
        }
        let has_changes = plan.entries.iter().any(|entry| !entry.changes.is_empty());
        if has_changes && state.sync_busy {
            content = content.push(note("Sincronizando en segundo plano…"));
        } else if has_changes {
            content = content.push(confirm("Confirmar sincronización", Message::ApplySync));
        }
    }
    if let Some(plan) = &state.rollback_preview {
        content = content.push(title(format!("Rollback desde {}", plan.backup_path)));
        if let Some(reason) = &plan.blocked_reason {
            content = content.push(line(
                "warning",
                design::DANGER,
                format!("No se puede aplicar: {reason}"),
            ));
        } else {
            for action in &plan.actions {
                content = content.push(line("check", design::SUCCESS, action));
            }
            if plan.changes_state {
                content = content.push(confirm("Restaurar desde backup", Message::ApplyRollback));
            }
        }
    }
    if let Some(plan) = &state.trust_preview {
        content = content.push(title(format!("Confianza de {}", plan.project_root)));
        content = content.push(note_owned(format!(
            "Estado actual: {}",
            if plan.currently_trusted {
                "confiable"
            } else {
                "no confiable"
            }
        )));
        for action in &plan.actions {
            content = content.push(line("check", design::SUCCESS, action));
        }
        content = content.push(note("Guardar confianza no ejecuta procesos."));
        if plan.changes_state {
            content = content.push(confirm(
                "Guardar decisión de confianza",
                Message::ApplyTrust,
            ));
        }
    }
    if let Some(plan) = &state.mcp_preview {
        content = content.push(title(format!(
            "Integración MCP para {}",
            plan.client_labels.join(", ")
        )));
        if let Some(reason) = &plan.blocked_reason {
            content = content.push(line(
                "warning",
                design::DANGER,
                format!("No se puede aplicar: {reason}"),
            ));
        } else {
            for action in &plan.actions {
                content = content.push(line("check", design::SUCCESS, action));
            }
            for warning in &plan.warnings {
                content = content.push(line(
                    "warning",
                    design::WARNING,
                    format!("Aviso: {warning}"),
                ));
            }
            if state.init_busy {
                content = content.push(note("Otra operación está en curso…"));
            } else if state.mcp_busy {
                content = content.push(note("Configuración en curso…"));
            } else {
                content = content.push(confirm("Aplicar integración MCP", Message::ApplyMcp));
            }
        }
    }
    if let Some(report) = state
        .doctor_report
        .as_ref()
        .filter(|_| state.page == Page::Diagnostics)
    {
        content = content.push(
            container(text(report.render()).font(design::MONO).size(12))
                .padding(14)
                .width(Fill)
                .style(styles::well),
        );
    }
    content.into()
}

fn title<'a>(value: impl text::IntoFragment<'a>) -> Element<'a, Message> {
    text(value).size(13).font(design::UI_BOLD).into()
}

fn note<'a>(value: &'a str) -> Element<'a, Message> {
    text(value).size(12).color(design::MUTED).into()
}

fn note_owned<'a>(value: String) -> Element<'a, Message> {
    text(value).size(12).color(design::MUTED).into()
}

fn line<'a>(
    icon: &'static str,
    tint: iced::Color,
    value: impl text::IntoFragment<'a>,
) -> Element<'a, Message> {
    row![icons::icon(icon, tint, 14.0), text(value).size(13)]
        .spacing(9)
        .align_y(iced::Center)
        .into()
}

fn confirm<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    button(text(label).size(12).font(design::UI_BOLD))
        .padding([8, 16])
        .style(styles::primary)
        .on_press(message)
        .into()
}
