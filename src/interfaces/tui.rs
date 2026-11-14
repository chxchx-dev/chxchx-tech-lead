use crate::features::project_overview::{ProjectOverview, inspect_project};
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

struct ViewState<'a> {
    project: &'a ProjectOverview,
    init_preview: Option<&'a crate::features::project_setup::ProjectInitPlan>,
    sync_preview: Option<&'a crate::features::project_sync::ProjectSyncPlan>,
    rollback_preview: Option<&'a crate::features::project_rollback::ProjectRollbackPlan>,
    trust_preview: Option<&'a crate::features::project_trust::ProjectTrustPlan>,
    mcp_preview: Option<&'a crate::features::mcp_setup::McpIntegrationPlan>,
    projects_catalog: Option<&'a crate::features::project_catalog::ProjectCatalog>,
    init_pending: bool,
    sync_pending: bool,
    mcp_pending: bool,
    project_scroll: u16,
    sync_scroll: u16,
    doctor_report: Option<&'a str>,
    doctor_scroll: u16,
    message: &'a str,
}

pub fn run(path: PathBuf) -> Result<(), String> {
    let mut project = inspect_project(&path)?;
    let mut init_preview = None;
    let mut sync_preview = None;
    let mut rollback_preview = None;
    let mut trust_preview = None;
    let mut mcp_preview = None;
    let mut projects_catalog = None;
    let mut init_pending = false;
    let mut sync_pending = false;
    let mut mcp_pending = false;
    let mut doctor_report = None;
    let mut doctor_scroll = 0_u16;
    let mut project_scroll = 0_u16;
    let mut sync_scroll = 0_u16;
    let mut message = String::new();
    let (mcp_sender, mcp_receiver) =
        mpsc::channel::<Result<crate::features::mcp_setup::McpIntegrationResult, String>>();
    let (init_sender, init_receiver) =
        mpsc::channel::<Result<crate::features::project_setup::ProjectInitApplication, String>>();
    let (sync_sender, sync_receiver) =
        mpsc::channel::<Result<crate::features::project_sync::ProjectSyncApplication, String>>();
    ratatui::run(|terminal| -> io::Result<()> {
        loop {
            if let Ok(result) = init_receiver.try_recv() {
                init_pending = false;
                match result {
                    Ok(result) => {
                        message = result.completed_actions.join(" · ");
                        if !result.backups.is_empty() {
                            message.push_str(&format!(" · Backup: {}", result.backups.join(", ")));
                        }
                        if !result.warnings.is_empty() {
                            message.push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                        }
                        init_preview = None;
                        if let Ok(updated) = inspect_project(&path) {
                            project = updated;
                        }
                    }
                    Err(error) => message = error,
                }
            }
            if let Ok(result) = mcp_receiver.try_recv() {
                mcp_pending = false;
                mcp_preview = None;
                match result {
                    Ok(result) => {
                        message = result.completed.join(" · ");
                        if !result.warnings.is_empty() {
                            message.push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                        }
                    }
                    Err(error) => message = error,
                }
            }
            if let Ok(result) = sync_receiver.try_recv() {
                sync_pending = false;
                sync_preview = None;
                match result {
                    Ok(result) => {
                        message = result
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
                        if message.is_empty() {
                            message = "No hubo proyectos sincronizados.".to_owned();
                        }
                        let backups = result
                            .outcomes
                            .iter()
                            .filter_map(|outcome| outcome.backup.as_deref())
                            .collect::<Vec<_>>();
                        if !backups.is_empty() {
                            message.push_str(&format!(" · Backups: {}", backups.join(", ")));
                        }
                        if !result.warnings.is_empty() {
                            message.push_str(&format!(" · Avisos: {}", result.warnings.join("; ")));
                        }
                    }
                    Err(error) => message = error,
                }
            }
            terminal.draw(|frame| {
                render(
                    frame,
                    &ViewState {
                        project: &project,
                        init_preview: init_preview.as_ref(),
                        sync_preview: sync_preview.as_ref(),
                        rollback_preview: rollback_preview.as_ref(),
                        trust_preview: trust_preview.as_ref(),
                        mcp_preview: mcp_preview.as_ref(),
                        projects_catalog: projects_catalog.as_ref(),
                        init_pending,
                        sync_pending,
                        mcp_pending,
                        project_scroll,
                        sync_scroll,
                        doctor_report: doctor_report.as_deref(),
                        doctor_scroll,
                        message: &message,
                    },
                )
            })?;
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let event = event::read()?;
            let Event::Key(key) = event else { continue };
            if init_pending || sync_pending {
                continue;
            }
            if mcp_pending && key.code == KeyCode::Char('y') {
                continue;
            }
            if projects_catalog.is_some() {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc | KeyCode::Char('p') => projects_catalog = None,
                    KeyCode::Up | KeyCode::Char('k') => {
                        project_scroll = project_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        project_scroll = project_scroll.saturating_add(1);
                    }
                    _ => {}
                }
                continue;
            }
            if sync_preview.is_some() {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => sync_preview = None,
                    KeyCode::Up | KeyCode::Char('k') => {
                        sync_scroll = sync_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        sync_scroll = sync_scroll.saturating_add(1);
                    }
                    KeyCode::Char('y') => {
                        if let Some(plan) = sync_preview.as_ref() {
                            if plan.entries.iter().any(|entry| {
                                !entry.changes.is_empty() && entry.blocked_reason.is_none()
                            }) {
                                let plan = plan.clone();
                                let sender = sync_sender.clone();
                                sync_pending = true;
                                message = "Sincronizando proyectos en segundo plano…".to_owned();
                                thread::spawn(move || {
                                    let result =
                                        crate::features::project_sync::apply_projects_sync(&plan);
                                    let _ = sender.send(result);
                                });
                            } else {
                                sync_preview = None;
                                message = "No hay cambios aplicables.".to_owned();
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if doctor_report.is_some() {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => doctor_report = None,
                    KeyCode::Up | KeyCode::Char('k') => {
                        doctor_scroll = doctor_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        doctor_scroll = doctor_scroll.saturating_add(1);
                    }
                    _ => {}
                }
                continue;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('i' | 'm') => {
                    let minimal = key.code == KeyCode::Char('m');
                    match crate::features::project_setup::plan_project_init(&path, minimal) {
                        Ok(plan) => {
                            rollback_preview = None;
                            sync_preview = None;
                            trust_preview = None;
                            mcp_preview = None;
                            message = if plan.blocked_reason.is_some() {
                                "Init necesita corregir la configuración existente.".to_owned()
                            } else {
                                "Vista previa lista; y aplica solo lo mostrado.".to_owned()
                            };
                            init_preview = Some(plan);
                        }
                        Err(error) => message = error,
                    }
                }
                KeyCode::Char('y') => {
                    if let Some(plan) = trust_preview.as_ref() {
                        let changed = plan.changes_state;
                        match crate::features::project_trust::apply_project_trust(plan) {
                            Ok(backup) => {
                                message = if !changed {
                                    "Sin cambios; la confianza ya estaba en ese estado.".to_owned()
                                } else if plan.desired_trust {
                                    "Confianza local guardada.".to_owned()
                                } else {
                                    "Confianza local revocada.".to_owned()
                                };
                                if let Some(backup) = backup {
                                    message.push_str(&format!(" · Backup: {backup}"));
                                }
                                trust_preview = None;
                                if let Ok(updated) = inspect_project(&path) {
                                    project = updated;
                                }
                            }
                            Err(error) => message = error,
                        }
                    } else if let Some(plan) = init_preview.as_ref() {
                        let plan = plan.clone();
                        let sender = init_sender.clone();
                        init_pending = true;
                        message = "Aplicando init en segundo plano…".to_owned();
                        thread::spawn(move || {
                            let result = crate::features::project_setup::apply_project_init(&plan);
                            let _ = sender.send(result);
                        });
                    } else if let Some(plan) = mcp_preview.clone()
                        && !mcp_pending
                        && !init_pending
                    {
                        let sender = mcp_sender.clone();
                        mcp_pending = true;
                        message = "Aplicando MCP en segundo plano…".to_owned();
                        thread::spawn(move || {
                            let result = crate::features::mcp_setup::apply_mcp_integration(&plan);
                            let _ = sender.send(result);
                        });
                    }
                }
                KeyCode::Char('b') => {
                    init_preview = None;
                    sync_preview = None;
                    trust_preview = None;
                    mcp_preview = None;
                    match crate::features::project_rollback::plan_project_rollback(&path) {
                        Ok(plan) => {
                            message =
                                "Vista previa de rollback lista; x restaura el backup.".to_owned();
                            rollback_preview = Some(plan);
                        }
                        Err(error) => message = error,
                    }
                }
                KeyCode::Char('x') => {
                    if let Some(plan) = rollback_preview.as_ref() {
                        match crate::features::project_rollback::apply_project_rollback(plan) {
                            Ok((actions, safety_backup)) => {
                                message = actions.join(" · ");
                                if let Some(backup) = safety_backup {
                                    message.push_str(&format!(" · Backup de seguridad: {backup}"));
                                }
                                rollback_preview = None;
                                if let Ok(updated) = inspect_project(&path) {
                                    project = updated;
                                }
                            }
                            Err(error) => message = error,
                        }
                    }
                }
                KeyCode::Char('t' | 'u') => {
                    let desired = key.code == KeyCode::Char('t');
                    match crate::features::project_trust::plan_project_trust(&path, desired) {
                        Ok(plan) => {
                            init_preview = None;
                            sync_preview = None;
                            rollback_preview = None;
                            mcp_preview = None;
                            trust_preview = Some(plan);
                            message =
                                "Vista previa lista; y guarda la decisión explícita.".to_owned();
                        }
                        Err(error) => message = error,
                    }
                }
                KeyCode::Char('c') => {
                    init_preview = None;
                    sync_preview = None;
                    rollback_preview = None;
                    trust_preview = None;
                    match crate::features::mcp_setup::McpClient::parse("all").and_then(|clients| {
                        crate::features::mcp_setup::plan_mcp_integration(&path, &clients, false)
                    }) {
                        Ok(plan) => {
                            message = "Vista previa MCP lista; y aplica tras revisar.".to_owned();
                            mcp_preview = Some(plan);
                        }
                        Err(error) => message = error,
                    }
                }
                KeyCode::Char('s') => {
                    init_preview = None;
                    rollback_preview = None;
                    trust_preview = None;
                    mcp_preview = None;
                    match crate::features::project_sync::plan_projects_sync() {
                        Ok(plan) => {
                            sync_preview = Some(plan);
                            sync_scroll = 0;
                            message = "Vista previa de sincronización lista; y aplica.".to_owned();
                        }
                        Err(error) => message = error,
                    }
                }
                KeyCode::Char('d') => match crate::features::doctor::diagnose_project(&path) {
                    Ok(report) => {
                        doctor_report = Some(report.render());
                        doctor_scroll = 0;
                        message.clear();
                    }
                    Err(error) => {
                        doctor_report = None;
                        message = error;
                    }
                },
                KeyCode::Char('p') => {
                    match crate::features::project_catalog::inspect_registered_projects() {
                        Ok(catalog) => {
                            projects_catalog = Some(catalog);
                            project_scroll = 0;
                            message = "Proyectos registrados · Esc vuelve al proyecto.".to_owned();
                        }
                        Err(error) => message = error,
                    }
                }
                KeyCode::Char('r') => match inspect_project(&path) {
                    Ok(updated) => {
                        project = updated;
                        message = "Estado actualizado.".to_owned();
                    }
                    Err(error) => message = error,
                },
                _ => {}
            }
        }
    })
    .map_err(|error| format!("No se pudo iniciar la interfaz de terminal: {error}"))
}

fn render(frame: &mut Frame<'_>, view: &ViewState<'_>) {
    if let Some(catalog) = view.projects_catalog {
        render_project_catalog(frame, catalog, view.project_scroll);
        return;
    }
    if let Some(plan) = view.sync_preview {
        render_sync_preview(frame, plan, view.sync_pending, view.sync_scroll);
        return;
    }
    if let Some(report) = view.doctor_report {
        let sections =
            Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(frame.area());
        frame.render_widget(
            Paragraph::new(report)
                .scroll((view.doctor_scroll, 0))
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Doctor · solo lectura"),
                ),
            sections[0],
        );
        frame.render_widget(
            Paragraph::new("↑/↓ o j/k · desplazar | Esc · volver | q · salir"),
            sections[1],
        );
        return;
    }
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Length(7),
        Constraint::Min(3),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(frame.area());

    let heading = Paragraph::new("ChxChx · Control plane de proyectos").block(
        Block::default()
            .borders(Borders::ALL)
            .title("Producto Rust"),
    );
    frame.render_widget(heading, sections[0]);
    let rows = [
        format!("Proyecto: {}", view.project.project.name),
        format!("Ruta: {}", view.project.project.root),
    ];
    frame.render_widget(
        Paragraph::new(rows.join("\n"))
            .block(Block::default().borders(Borders::ALL).title("Proyecto")),
        sections[1],
    );
    let config = match (
        view.project.workspace.config_exists,
        view.project.workspace.config_parseable,
        view.project.workspace.config_supported,
    ) {
        (false, _, _) => "sin inicializar",
        (true, Some(false), _) => "TOML mal formado",
        (true, Some(true), Some(false)) => "configuración no compatible",
        (true, Some(true), Some(true)) => "configuración compatible",
        (true, Some(true), None) => "sin validar",
        (true, None, _) => "no se pudo leer",
    };
    let mut status = format!(
        "Git: {}  ·  Configuración: {}  ·  Confianza local: {}",
        mark(view.project.project.git_repository),
        config,
        mark(view.project.workspace.trusted)
    );
    if let Some(error) = &view.project.workspace.config_validation_error {
        status.push_str("  ·  ");
        status.push_str(error);
    }
    frame.render_widget(
        Paragraph::new(status).block(Block::default().borders(Borders::ALL).title("Estado")),
        sections[2],
    );
    let markers = if view.project.project.detected_markers.is_empty() {
        "No se detectaron marcadores conocidos.".to_owned()
    } else {
        view.project.project.detected_markers.join(", ")
    };
    let technology = format!(
        "Marcadores: {markers}\nStacks: {}\nLenguajes: {}\nGestores: {}\nInfraestructura: {}\nPerfil: {}",
        display_list(&view.project.detection.stacks),
        display_list(&view.project.detection.languages),
        display_list(&view.project.detection.package_managers),
        display_list(&view.project.detection.infrastructure),
        view.project.detection.profile,
    );
    frame.render_widget(
        Paragraph::new(technology).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Detección y perfil"),
        ),
        sections[3],
    );
    let mut details = if view.project.detection.profile_rules.is_empty() {
        "El perfil no añade reglas específicas.".to_owned()
    } else {
        view.project.detection.profile_rules.join("\n")
    };
    if let Some(plan) = view.init_preview {
        details.push_str("\n\nVista previa de init:");
        if view.init_pending {
            details.push_str("\nAplicando init en segundo plano…");
        }
        if let Some(reason) = &plan.blocked_reason {
            details.push_str(&format!("\nNo se puede aplicar: {reason}"));
        } else {
            for action in &plan.actions {
                details.push_str(&format!("\n• {action}"));
            }
            for warning in &plan.warnings {
                details.push_str(&format!("\nAviso: {warning}"));
            }
        }
    }
    if let Some(plan) = view.rollback_preview {
        details.push_str("\n\nVista previa de rollback:");
        details.push_str(&format!("\nBackup: {}", plan.backup_path));
        if let Some(reason) = &plan.blocked_reason {
            details.push_str(&format!("\nNo se puede aplicar: {reason}"));
        } else {
            for action in &plan.actions {
                details.push_str(&format!("\n• {action}"));
            }
        }
    }
    if let Some(plan) = view.trust_preview {
        details.push_str("\n\nVista previa de confianza:");
        details.push_str(&format!("\nProyecto: {}", plan.project_root));
        details.push_str(&format!(
            "\nEstado actual: {}",
            if plan.currently_trusted {
                "confiable"
            } else {
                "no confiable"
            }
        ));
        for action in &plan.actions {
            details.push_str(&format!("\n• {action}"));
        }
        details.push_str("\nGuardar trust no ejecuta procesos.");
    }
    if let Some(plan) = view.mcp_preview {
        details.push_str("\n\nVista previa MCP:");
        details.push_str(&format!("\nClientes: {}", plan.client_labels.join(", ")));
        if let Some(reason) = &plan.blocked_reason {
            details.push_str(&format!("\nNo se puede aplicar: {reason}"));
        } else {
            for action in &plan.actions {
                details.push_str(&format!("\n• {action}"));
            }
            for warning in &plan.warnings {
                details.push_str(&format!("\nAviso: {warning}"));
            }
            if view.mcp_pending {
                details.push_str("\nIntegración en curso; no cierres esta vista aún.");
            }
        }
    }
    if !view.message.is_empty() {
        details.push_str(&format!("\n\n{}", view.message));
    }
    frame.render_widget(
        Paragraph::new(details).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Perfil y acciones"),
        ),
        sections[4],
    );
    frame.render_widget(
        Paragraph::new(
            "i init | m compacto | s sync | y aplicar | b rollback | x restaurar | c MCP | d doctor | p proyectos\nt trust | u revocar confianza | r actualizar",
        )
        .block(Block::default().borders(Borders::ALL).title("Migración")),
        sections[5],
    );
    frame.render_widget(Paragraph::new("q · salir"), sections[6]);
}

fn render_project_catalog(
    frame: &mut Frame<'_>,
    catalog: &crate::features::project_catalog::ProjectCatalog,
    scroll: u16,
) {
    let mut lines = vec!["Proyectos registrados".to_owned()];
    if catalog.projects.is_empty() {
        lines.push("No hay proyectos registrados.".to_owned());
    } else {
        lines.extend(catalog.projects.iter().map(|project| {
            let state = match (project.available, project.is_current) {
                (true, true) => "disponible · último",
                (true, false) => "disponible",
                (false, _) => "no encontrado",
            };
            format!(
                "{} · {} · {} · {} · {} · {}",
                project.alias,
                project.name,
                project.profile,
                if project.stacks.is_empty() {
                    "ninguno".to_owned()
                } else {
                    project.stacks.join(", ")
                },
                state,
                project.path.as_deref().unwrap_or("<ruta ausente>")
            )
        }));
    }
    if let Some(current) = &catalog.current {
        lines.push(format!(
            "Último proyecto: {} · {}",
            current.alias, current.path
        ));
    }
    frame.render_widget(
        Paragraph::new(lines.join("\n"))
            .scroll((scroll, 0))
            .wrap(ratatui::widgets::Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Proyectos · ↑/↓ o j/k · Esc vuelve"),
            ),
        frame.area(),
    );
}

fn render_sync_preview(
    frame: &mut Frame<'_>,
    plan: &crate::features::project_sync::ProjectSyncPlan,
    pending: bool,
    scroll: u16,
) {
    let mut lines = vec!["Vista previa: sincronización de proyectos".to_owned()];
    if plan.entries.is_empty() {
        lines.push("No hay proyectos disponibles para sincronizar.".to_owned());
    }
    for entry in &plan.entries {
        lines.push(format!("{} · {}", entry.alias, entry.project_root));
        if let Some(reason) = &entry.blocked_reason {
            lines.push(format!("  Omitido: {reason}"));
        } else if entry.changes.is_empty() {
            lines.push("  Sin cambios.".to_owned());
        } else {
            lines.extend(entry.changes.iter().map(|change| format!("  • {change}")));
        }
    }
    lines.extend(
        plan.warnings
            .iter()
            .map(|warning| format!("Aviso: {warning}")),
    );
    let has_changes = plan
        .entries
        .iter()
        .any(|entry| !entry.changes.is_empty() && entry.blocked_reason.is_none());
    lines.push(if pending {
        "Sincronización en curso en segundo plano…".to_owned()
    } else if has_changes {
        "y aplica · Esc cancela · ↑/↓ o j/k desplaza".to_owned()
    } else {
        "No hay cambios aplicables · Esc cierra · ↑/↓ o j/k desplaza".to_owned()
    });
    frame.render_widget(
        Paragraph::new(lines.join("\n"))
            .scroll((scroll, 0))
            .wrap(ratatui::widgets::Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title("Sync")),
        frame.area(),
    );
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
