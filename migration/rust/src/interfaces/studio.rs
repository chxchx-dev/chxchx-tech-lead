use crate::features::project_overview::{ProjectOverview, inspect_project};
use crate::features::project_setup::ProjectInitPlan;
use iced::widget::{button, column, container, scrollable, text};
use iced::{Element, Fill, Task};
use std::path::PathBuf;

struct Dashboard {
    root: PathBuf,
    overview: Result<ProjectOverview, String>,
    init_preview: Option<ProjectInitPlan>,
    sync_preview: Option<crate::features::project_sync::ProjectSyncPlan>,
    rollback_preview: Option<crate::features::project_rollback::ProjectRollbackPlan>,
    trust_preview: Option<crate::features::project_trust::ProjectTrustPlan>,
    mcp_preview: Option<crate::features::mcp_setup::McpIntegrationPlan>,
    projects_catalog: Option<crate::features::project_catalog::ProjectCatalog>,
    doctor_report: Option<crate::features::doctor::DoctorReport>,
    init_busy: bool,
    sync_busy: bool,
    mcp_busy: bool,
    message: String,
}

#[derive(Clone, Debug)]
enum Message {
    Refresh,
    PreviewInit { minimal: bool },
    ApplyInit,
    PreviewSync,
    ApplySync,
    PreviewRollback,
    ApplyRollback,
    PreviewTrust { desired: bool },
    ApplyTrust,
    PreviewMcp,
    ApplyMcp,
    Projects,
    InitApplied(Result<crate::features::project_setup::ProjectInitApplication, String>),
    SyncApplied(Result<crate::features::project_sync::ProjectSyncApplication, String>),
    Doctor,
    McpApplied(Result<crate::features::mcp_setup::McpIntegrationResult, String>),
}

pub fn run(path: PathBuf) -> Result<(), String> {
    iced::application(
        move || Dashboard {
            root: path.clone(),
            overview: inspect_project(&path),
            init_preview: None,
            sync_preview: None,
            rollback_preview: None,
            trust_preview: None,
            mcp_preview: None,
            projects_catalog: None,
            doctor_report: None,
            init_busy: false,
            sync_busy: false,
            mcp_busy: false,
            message: String::new(),
        },
        update,
        view,
    )
    .title("ChxChx Studio")
    .run()
    .map_err(|error| format!("No se pudo abrir ChxChx Studio: {error}"))
}

fn update(state: &mut Dashboard, message: Message) -> Task<Message> {
    if (state.init_busy && !matches!(&message, Message::InitApplied(_)))
        || (state.sync_busy && !matches!(&message, Message::SyncApplied(_)))
        || (state.mcp_busy && !matches!(&message, Message::McpApplied(_)))
    {
        return Task::none();
    }
    match message {
        Message::Refresh => state.overview = inspect_project(&state.root),
        Message::PreviewInit { minimal } => {
            state.sync_preview = None;
            state.rollback_preview = None;
            state.trust_preview = None;
            state.mcp_preview = None;
            match crate::features::project_setup::plan_project_init(&state.root, minimal) {
                Ok(plan) => {
                    state.message = if plan.blocked_reason.is_some() {
                        "La configuración existente requiere atención.".to_owned()
                    } else {
                        "Revisa las acciones antes de aplicarlas.".to_owned()
                    };
                    state.init_preview = Some(plan);
                }
                Err(error) => state.message = error,
            }
        }
        Message::ApplyInit => {
            if let Some(plan) = state.init_preview.clone() {
                state.init_busy = true;
                state.message = "Aplicando init en segundo plano…".to_owned();
                return Task::perform(
                    async move {
                        std::thread::spawn(move || {
                            crate::features::project_setup::apply_project_init(&plan)
                        })
                        .join()
                        .unwrap_or_else(|_| Err("Falló el worker de init".to_owned()))
                    },
                    Message::InitApplied,
                );
            }
        }
        Message::PreviewSync => {
            state.init_preview = None;
            state.rollback_preview = None;
            state.trust_preview = None;
            state.mcp_preview = None;
            match crate::features::project_sync::plan_projects_sync() {
                Ok(plan) => {
                    state.message =
                        "Revisa los bloques administrados antes de sincronizar.".to_owned();
                    state.sync_preview = Some(plan);
                }
                Err(error) => state.message = error,
            }
        }
        Message::ApplySync => {
            if let Some(plan) = state.sync_preview.clone() {
                state.sync_busy = true;
                state.message = "Sincronizando proyectos en segundo plano…".to_owned();
                return Task::perform(
                    async move {
                        std::thread::spawn(move || {
                            crate::features::project_sync::apply_projects_sync(&plan)
                        })
                        .join()
                        .unwrap_or_else(|_| Err("Falló el worker de sincronización".to_owned()))
                    },
                    Message::SyncApplied,
                );
            }
        }
        Message::SyncApplied(result) => {
            state.sync_busy = false;
            state.sync_preview = None;
            match result {
                Ok(result) => {
                    state.message = result
                        .outcomes
                        .iter()
                        .map(|outcome| {
                            if outcome.changed_files.is_empty() {
                                format!("{} · sin cambios", outcome.project_root)
                            } else {
                                format!("{} · sincronizado", outcome.project_root)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" · ");
                    if state.message.is_empty() {
                        state.message = "No hubo proyectos sincronizados.".to_owned();
                    }
                    let backups = result
                        .outcomes
                        .iter()
                        .filter_map(|outcome| outcome.backup.as_deref())
                        .collect::<Vec<_>>();
                    if !backups.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Backups: {}", backups.join(", ")));
                    }
                    if !result.warnings.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                    }
                }
                Err(error) => state.message = error,
            }
        }
        Message::InitApplied(result) => {
            state.init_busy = false;
            match result {
                Ok(result) => {
                    state.message = result.completed_actions.join(" · ");
                    if !result.backups.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Backup: {}", result.backups.join(", ")));
                    }
                    if !result.warnings.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                    }
                    state.init_preview = None;
                    state.overview = inspect_project(&state.root);
                }
                Err(error) => state.message = error,
            }
        }
        Message::PreviewRollback => {
            state.init_preview = None;
            state.sync_preview = None;
            state.trust_preview = None;
            state.mcp_preview = None;
            match crate::features::project_rollback::plan_project_rollback(&state.root) {
                Ok(plan) => {
                    state.message = "Revisa los archivos que se restaurarán.".to_owned();
                    state.rollback_preview = Some(plan);
                }
                Err(error) => state.message = error,
            }
        }
        Message::ApplyRollback => {
            if let Some(plan) = state.rollback_preview.as_ref() {
                match crate::features::project_rollback::apply_project_rollback(plan) {
                    Ok((actions, safety_backup)) => {
                        state.message = actions.join(" · ");
                        if let Some(backup) = safety_backup {
                            state
                                .message
                                .push_str(&format!(" · Backup de seguridad: {backup}"));
                        }
                        state.rollback_preview = None;
                        state.overview = inspect_project(&state.root);
                    }
                    Err(error) => state.message = error,
                }
            }
        }
        Message::PreviewTrust { desired } => {
            state.init_preview = None;
            state.sync_preview = None;
            state.rollback_preview = None;
            state.mcp_preview = None;
            match crate::features::project_trust::plan_project_trust(&state.root, desired) {
                Ok(plan) => {
                    state.message = "Revisa el cambio de confianza antes de guardarlo.".to_owned();
                    state.trust_preview = Some(plan);
                }
                Err(error) => state.message = error,
            }
        }
        Message::ApplyTrust => {
            if let Some(plan) = state.trust_preview.as_ref() {
                let changed = plan.changes_state;
                match crate::features::project_trust::apply_project_trust(plan) {
                    Ok(backup) => {
                        state.message = if !changed {
                            "Sin cambios; la confianza ya estaba en ese estado.".to_owned()
                        } else if plan.desired_trust {
                            "Confianza local guardada.".to_owned()
                        } else {
                            "Confianza local revocada.".to_owned()
                        };
                        if let Some(backup) = backup {
                            state.message.push_str(&format!(" · Backup: {backup}"));
                        }
                        state.trust_preview = None;
                        state.overview = inspect_project(&state.root);
                    }
                    Err(error) => state.message = error,
                }
            }
        }
        Message::PreviewMcp => {
            state.init_preview = None;
            state.sync_preview = None;
            state.rollback_preview = None;
            state.trust_preview = None;
            match crate::features::mcp_setup::McpClient::parse("all").and_then(|clients| {
                crate::features::mcp_setup::plan_mcp_integration(&state.root, &clients, false)
            }) {
                Ok(plan) => {
                    state.message = "Revisa el alcance de MCP antes de aplicar.".to_owned();
                    state.mcp_preview = Some(plan);
                }
                Err(error) => state.message = error,
            }
        }
        Message::Doctor => match crate::features::doctor::diagnose_project(&state.root) {
            Ok(report) => {
                state.message = "Diagnóstico solo lectura completado.".to_owned();
                state.doctor_report = Some(report);
            }
            Err(error) => state.message = error,
        },
        Message::Projects => {
            match crate::features::project_catalog::inspect_registered_projects() {
                Ok(catalog) => {
                    state.projects_catalog = Some(catalog);
                    state.message = "Lista de proyectos actualizada.".to_owned();
                }
                Err(error) => state.message = error,
            }
        }
        Message::ApplyMcp => {
            if state.mcp_busy || state.init_busy {
                return Task::none();
            }
            if let Some(plan) = state.mcp_preview.clone() {
                state.mcp_busy = true;
                state.message = "Configurando MCP en segundo plano…".to_owned();
                return Task::perform(
                    async move {
                        std::thread::spawn(move || {
                            crate::features::mcp_setup::apply_mcp_integration(&plan)
                        })
                        .join()
                        .unwrap_or_else(|_| Err("Falló el worker de integración MCP".to_owned()))
                    },
                    Message::McpApplied,
                );
            }
        }
        Message::McpApplied(result) => {
            state.mcp_busy = false;
            state.mcp_preview = None;
            match result {
                Ok(result) => {
                    state.message = result.completed.join(" · ");
                    if !result.warnings.is_empty() {
                        state
                            .message
                            .push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                    }
                }
                Err(error) => state.message = error,
            }
        }
    }
    Task::none()
}

fn view(state: &Dashboard) -> Element<'_, Message> {
    let mut content = column![
        text("ChxChx Studio").size(32),
        text("Un producto. Capacidades compartidas."),
        text(state.root.display().to_string()),
    ]
    .spacing(16);

    match &state.overview {
        Ok(project) => {
            content = content.push(text(format!("Proyecto: {}", project.project.name)));
            content = content.push(text(format!(
                "Git: {}  ·  Configuración: {}  ·  Confianza: {}",
                mark(project.project.git_repository),
                config_status(project),
                mark(project.workspace.trusted)
            )));
            if let Some(error) = &project.workspace.config_validation_error {
                content = content.push(text(format!("Configuración: {error}")));
            }
            let markers = if project.project.detected_markers.is_empty() {
                "Sin marcadores conocidos".to_owned()
            } else {
                project.project.detected_markers.join(", ")
            };
            content = content.push(text(format!("Tecnologías: {markers}")));
            content = content.push(text(format!(
                "Stacks: {}",
                display_list(&project.detection.stacks)
            )));
            content = content.push(text(format!(
                "Lenguajes: {}  ·  Gestores: {}",
                display_list(&project.detection.languages),
                display_list(&project.detection.package_managers)
            )));
            content = content.push(text(format!(
                "Infraestructura: {}",
                display_list(&project.detection.infrastructure)
            )));
            content = content.push(text(format!("Perfil: {}", project.detection.profile)));
            for rule in &project.detection.profile_rules {
                content = content.push(text(format!("Regla del perfil: {rule}")));
            }
        }
        Err(error) => content = content.push(text(error.clone())),
    }

    content = content
        .push(button("Actualizar estado").on_press(Message::Refresh))
        .push(
            button("Previsualizar init completo").on_press(Message::PreviewInit { minimal: false }),
        )
        .push(
            button("Previsualizar init compacto").on_press(Message::PreviewInit { minimal: true }),
        )
        .push(button("Previsualizar sync de proyectos").on_press(Message::PreviewSync))
        .push(button("Previsualizar rollback").on_press(Message::PreviewRollback))
        .push(button("Previsualizar trust").on_press(Message::PreviewTrust { desired: true }))
        .push(button("Previsualizar revocación").on_press(Message::PreviewTrust { desired: false }))
        .push(button("Previsualizar MCP").on_press(Message::PreviewMcp))
        .push(button("Proyectos registrados").on_press(Message::Projects))
        .push(button("Ejecutar diagnóstico").on_press(Message::Doctor));
    if let Some(catalog) = &state.projects_catalog {
        content = content.push(text("Proyectos registrados"));
        if catalog.projects.is_empty() {
            content = content.push(text("No hay proyectos registrados."));
        } else {
            for project in &catalog.projects {
                let status = match (project.available, project.is_current) {
                    (true, true) => "disponible · último",
                    (true, false) => "disponible",
                    (false, _) => "no encontrado",
                };
                content = content.push(text(format!(
                    "{} · {} · {} · {} · {} · {}",
                    project.alias,
                    project.name,
                    project.profile,
                    if project.stacks.is_empty() {
                        "ninguno".to_owned()
                    } else {
                        project.stacks.join(", ")
                    },
                    status,
                    project.path.as_deref().unwrap_or("<ruta ausente>")
                )));
            }
        }
        if let Some(current) = &catalog.current {
            content = content.push(text(format!(
                "Último proyecto: {} · {}",
                current.alias, current.path
            )));
        }
    }
    if let Some(plan) = &state.init_preview {
        content = content.push(text(format!("Vista previa para {}", plan.project_root)));
        if let Some(reason) = &plan.blocked_reason {
            content = content.push(text(format!("No se puede aplicar: {reason}")));
        } else {
            for action in &plan.actions {
                content = content.push(text(format!("• {action}")));
            }
            for warning in &plan.warnings {
                content = content.push(text(format!("Aviso: {warning}")));
            }
            if plan
                .actions
                .iter()
                .any(|action| action != "todo está actualizado")
            {
                if state.init_busy {
                    content = content.push(text("Aplicando init en segundo plano…"));
                } else {
                    content =
                        content.push(button("Aplicar vista previa").on_press(Message::ApplyInit));
                }
            }
        }
    }
    if let Some(plan) = &state.sync_preview {
        content = content.push(text("Vista previa de sincronización de proyectos"));
        for entry in &plan.entries {
            content = content.push(text(format!("{} · {}", entry.alias, entry.project_root)));
            if let Some(reason) = &entry.blocked_reason {
                content = content.push(text(format!("Omitido: {reason}")));
            } else if entry.changes.is_empty() {
                content = content.push(text("Sin cambios."));
            } else {
                for action in &entry.changes {
                    content = content.push(text(format!("• {action}")));
                }
            }
        }
        for warning in &plan.warnings {
            content = content.push(text(format!("Aviso: {warning}")));
        }
        let has_changes = plan.entries.iter().any(|entry| !entry.changes.is_empty());
        if has_changes && state.sync_busy {
            content = content.push(text("Sincronizando en segundo plano…"));
        } else if has_changes {
            content =
                content.push(button("Aplicar sync de proyectos").on_press(Message::ApplySync));
        }
    }
    if let Some(plan) = &state.rollback_preview {
        content = content.push(text(format!("Rollback desde {}", plan.backup_path)));
        if let Some(reason) = &plan.blocked_reason {
            content = content.push(text(format!("No se puede aplicar: {reason}")));
        } else {
            for action in &plan.actions {
                content = content.push(text(format!("• {action}")));
            }
            if plan.changes_state {
                content =
                    content.push(button("Restaurar desde backup").on_press(Message::ApplyRollback));
            }
        }
    }
    if let Some(plan) = &state.trust_preview {
        content = content.push(text(format!("Confianza de {}", plan.project_root)));
        content = content.push(text(format!(
            "Estado actual: {}",
            if plan.currently_trusted {
                "confiable"
            } else {
                "no confiable"
            }
        )));
        for action in &plan.actions {
            content = content.push(text(format!("• {action}")));
        }
        content = content.push(text("Guardar confianza no ejecuta procesos."));
        if plan.changes_state {
            content =
                content.push(button("Guardar decisión de confianza").on_press(Message::ApplyTrust));
        }
    }
    if let Some(plan) = &state.mcp_preview {
        content = content.push(text(format!(
            "Integración MCP para {}",
            plan.client_labels.join(", ")
        )));
        if let Some(reason) = &plan.blocked_reason {
            content = content.push(text(format!("No se puede aplicar: {reason}")));
        } else {
            for action in &plan.actions {
                content = content.push(text(format!("• {action}")));
            }
            for warning in &plan.warnings {
                content = content.push(text(format!("Aviso: {warning}")));
            }
            if state.init_busy {
                content = content.push(text("Otra operación está en curso…"));
            } else if state.mcp_busy {
                content = content.push(text("Configuración en curso…"));
            } else {
                content =
                    content.push(button("Aplicar integración MCP").on_press(Message::ApplyMcp));
            }
        }
    }
    if let Some(report) = &state.doctor_report {
        content = content.push(text(report.render()));
    }
    if !state.message.is_empty() {
        content = content.push(text(state.message.clone()));
    }
    container(scrollable(content.padding(28)).width(Fill).height(Fill)).into()
}

fn config_status(project: &ProjectOverview) -> &'static str {
    match (
        project.workspace.config_exists,
        project.workspace.config_parseable,
        project.workspace.config_supported,
    ) {
        (false, _, _) => "sin inicializar",
        (true, Some(false), _) => "TOML mal formado",
        (true, Some(true), Some(false)) => "configuración no compatible",
        (true, Some(true), Some(true)) => "configuración compatible",
        (true, Some(true), None) => "sin validar",
        (true, None, _) => "no se pudo leer",
    }
}

fn mark(value: bool) -> &'static str {
    if value { "sí" } else { "no" }
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() {
        "ninguno".to_owned()
    } else {
        values.join(", ")
    }
}
