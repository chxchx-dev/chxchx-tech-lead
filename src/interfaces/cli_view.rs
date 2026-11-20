//! What each command prints. Plain output is the text scripts have always read; on a terminal the
//! same facts get color, glyphs and small layouts (see `cli_style`).
use super::style::{Tone, Ui, pad};
use crate::features::project_overview::{ProjectOverview, inspect_project};
use std::path::Path;

pub(super) fn version(ui: &Ui) {
    if !ui.is_rich() {
        println!("ChxChx Tech {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    println!();
    for row in ui.banner() {
        println!("{row}");
    }
    println!(
        "\n  {}  {}",
        ui.strong(Tone::Cyan, &format!("Tech {}", env!("CARGO_PKG_VERSION"))),
        ui.dim("control plane local para proyectos con agentes de IA")
    );
    println!();
}

pub(super) fn doctor(ui: &Ui, path: &Path) -> Result<(), String> {
    let report = crate::features::doctor::diagnose_project(path)?;
    if !ui.is_rich() {
        println!("{}", report.render());
        return Ok(());
    }
    let count = |status: &str| {
        report
            .findings
            .iter()
            .filter(|f| f.status == status)
            .count()
    };
    println!(
        "{}",
        ui.heading(
            "",
            &format!("Doctor · {}", report.project_name),
            &report.project_root
        )
    );
    println!(
        "  {} {} {}   {}",
        ui.badge(Tone::Green, &format!("{} bien", count("OK"))),
        ui.badge(Tone::Yellow, &format!("{} faltan", count("FALTA"))),
        ui.badge(Tone::Red, &format!("{} con error", count("ERROR"))),
        ui.dim(&format!("perfil {}", report.profile))
    );
    let status = |s: &str| match s {
        "OK" | "ok" => ui.strong(Tone::Green, "✓"),
        "FALTA" | "AVISO" => ui.strong(Tone::Yellow, "▲"),
        "ERROR" => ui.strong(Tone::Red, "✗"),
        _ => ui.dim("·"),
    };
    for area in ["HERRAMIENTAS", "ARCHIVOS DEL PROYECTO", "WORKSPACE"] {
        println!("\n  {}", ui.strong(Tone::Cyan, area));
        let rows: Vec<_> = report.findings.iter().filter(|f| f.area == area).collect();
        let width = rows
            .iter()
            .map(|f| f.subject.chars().count())
            .max()
            .unwrap_or(0);
        for finding in rows {
            println!(
                "    {} {}  {}",
                status(finding.status),
                pad(&finding.subject, width),
                ui.dim(&finding.detail)
            );
        }
    }
    println!(
        "\n  {}",
        ui.strong(Tone::Cyan, "INTEGRACIONES MCP (solo lectura)")
    );
    if report.mcp.is_empty() {
        println!(
            "    {}",
            ui.dim("Todavía no hay servidores MCP configurados.")
        );
    }
    for item in &report.mcp {
        println!(
            "    {} {} / {}  {}",
            status(item.status),
            item.client,
            item.server,
            ui.dim(&format!("{} · {}", item.scope, item.detail))
        );
    }
    println!(
        "{}",
        ui.hint("Diagnóstico estático: no ejecuta agentes, servidores ni comandos configurados.")
    );
    Ok(())
}

pub(super) fn projects_list(ui: &Ui) -> Result<(), String> {
    let catalog = crate::features::project_catalog::inspect_registered_projects()?;
    if catalog.projects.is_empty() {
        println!("No hay proyectos registrados.");
        return Ok(());
    }
    let stacks = |p: &crate::features::project_catalog::RegisteredProject| {
        if p.stacks.is_empty() {
            "ninguno".to_owned()
        } else {
            p.stacks.join(", ")
        }
    };
    let state =
        |p: &crate::features::project_catalog::RegisteredProject| match (p.available, p.is_current)
        {
            (true, true) => "disponible · último",
            (true, false) => "disponible",
            (false, _) => "no encontrado",
        };
    if !ui.is_rich() {
        println!("Alias | Nombre | Perfil | Stacks | Estado | Ruta");
        for p in &catalog.projects {
            println!(
                "{} | {} | {} | {} | {} | {}",
                p.alias,
                p.name,
                p.profile,
                stacks(p),
                state(p),
                p.path.as_deref().unwrap_or("<ruta ausente>")
            );
        }
        return Ok(());
    }
    let width = |f: &dyn Fn(&crate::features::project_catalog::RegisteredProject) -> String,
                 head: &str| {
        catalog
            .projects
            .iter()
            .map(|p| f(p).chars().count())
            .chain([head.chars().count()])
            .max()
            .unwrap_or(0)
    };
    let (wa, wn, wp, ws) = (
        width(&|p| p.alias.clone(), "Alias"),
        width(&|p| p.name.clone(), "Nombre"),
        width(&|p| p.profile.clone(), "Perfil"),
        width(&|p| stacks(p), "Stacks"),
    );
    println!("{}", ui.heading("", "Proyectos registrados", ""));
    println!(
        "    {}",
        ui.strong(
            Tone::Cyan,
            &format!(
                "  {}  {}  {}  {}  Ruta",
                pad("Alias", wa),
                pad("Nombre", wn),
                pad("Perfil", wp),
                pad("Stacks", ws)
            )
        )
    );
    for p in &catalog.projects {
        let dot = if p.available {
            ui.strong(Tone::Green, "●")
        } else {
            ui.strong(Tone::Red, "○")
        };
        println!(
            "    {dot} {}  {}  {}  {}  {}",
            ui.bold(&pad(&p.alias, wa)),
            pad(&p.name, wn),
            ui.paint(Tone::Purple, &pad(&p.profile, wp)),
            ui.paint(Tone::Cyan, &pad(&stacks(p), ws)),
            ui.dim(p.path.as_deref().unwrap_or("<ruta ausente>"))
        );
        if !p.available || p.is_current {
            println!(
                "      {}",
                ui.paint(if p.available { Tone::Cyan } else { Tone::Red }, state(p))
            );
        }
    }
    println!();
    Ok(())
}

pub(super) fn projects_current(ui: &Ui) -> Result<(), String> {
    let catalog = crate::features::project_catalog::inspect_registered_projects()?;
    if let Some(project) = catalog.current {
        println!(
            "{}",
            ui.done(&format!("{}: {}", project.alias, project.path))
                .trim_start()
        );
    } else {
        println!("No hay un último proyecto disponible.");
    }
    Ok(())
}

/// Keeps a failure where `workspace errors` can show it. The cache is a convenience: when it
/// cannot be written the original error is still the one the person sees.
pub(super) fn remember_failure(path: &Path, operation: &str, error: &str) {
    let Some(home) = crate::features::project_profiles::application_home() else {
        return;
    };
    let root = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let project = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let _ = crate::features::error_cache::record(&home, &project, &root, operation, error);
}

pub(super) fn errors(ui: &Ui, path: Option<&Path>, limit: usize) -> Result<(), String> {
    let home = crate::features::project_profiles::application_home()
        .ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    let found = crate::features::error_cache::list(&home, path, limit);
    if found.is_empty() {
        println!("No hay errores guardados.");
        return Ok(());
    }
    for error in found {
        println!(
            "{} · {} · {}",
            ui.dim(&error.occurred_at),
            ui.bold(&error.project),
            error.operation
        );
        println!("  {}", error.message.replace('\n', "\n  "));
    }
    Ok(())
}

pub(super) fn report_error(
    ui: &Ui,
    path: &Path,
    operation: &str,
    message: &str,
) -> Result<(), String> {
    let home = crate::features::project_profiles::application_home()
        .ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    let root =
        std::fs::canonicalize(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let project = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    crate::features::error_cache::record(&home, &project, &root, operation, message)?;
    println!("{}", ui.done("Error guardado").trim_start());
    Ok(())
}

pub(super) fn tools_install(ui: &Ui, names: &[String], dry_run: bool) -> Result<(), String> {
    use crate::features::tools_install::{Outcome, TOOLS, install};
    let wanted: Vec<&str> = if names.is_empty() {
        TOOLS.iter().map(|tool| tool.name).collect()
    } else {
        names.iter().map(String::as_str).collect()
    };
    for name in wanted {
        match install(name, dry_run)? {
            Outcome::AlreadyInstalled => println!(
                "{}",
                ui.done(&format!("{name}: ya está instalado")).trim_start()
            ),
            Outcome::Installed => {
                println!("{}", ui.done(&format!("{name}: instalado")).trim_start())
            }
            Outcome::Preview(command) => println!("DRY RUN: {}", command.join(" ")),
        }
    }
    Ok(())
}

pub(super) fn projects_switch(ui: &Ui, alias: &str, dry_run: bool) -> Result<(), String> {
    let switched = crate::features::project_switch::switch(alias, dry_run)?;
    let prefix = if dry_run { "DRY RUN: " } else { "" };
    for (path, action) in &switched.suspended {
        println!(
            "{}",
            ui.done(&format!(
                "{prefix}workspace suspendido: {} · {}",
                path.display(),
                action.status
            ))
            .trim_start()
        );
    }
    println!("{}", ui.bullet(&switched.started.message).trim_start());
    for process in &switched.started.processes {
        println!("{}", ui.bullet(&process.message).trim_start());
    }
    println!(
        "{}",
        ui.done(&format!(
            "{prefix}proyecto activo: {} ({})",
            switched.name, switched.alias
        ))
        .trim_start()
    );
    Ok(())
}

pub(super) fn projects_sync(ui: &Ui, apply: bool) -> Result<(), String> {
    let plan = crate::features::project_sync::plan_projects_sync()?;
    println!(
        "{}",
        ui.heading(
            "Sincronización de proyectos registrados:",
            "Sincronización de proyectos registrados",
            ""
        )
    );
    for entry in &plan.entries {
        println!(
            "\n{} · {}",
            ui.bold(&entry.alias),
            ui.dim(&entry.project_root)
        );
        if let Some(reason) = &entry.blocked_reason {
            println!("  Omitido: {reason}");
        } else if entry.changes.is_empty() {
            println!("  Sin cambios.");
        } else {
            for action in &entry.changes {
                println!("  {}", ui.bullet(action).trim_start());
            }
        }
    }
    if !apply {
        for warning in &plan.warnings {
            println!("{}", ui.warning(warning));
        }
        if plan.entries.iter().any(|entry| !entry.changes.is_empty()) {
            println!(
                "{}",
                ui.hint("Vista previa: vuelve a ejecutar con `projects sync --apply`.")
            );
        } else {
            println!("\nNo hay cambios aplicables.");
        }
        return Ok(());
    }
    let result = crate::features::project_sync::apply_projects_sync(&plan)?;
    if result.outcomes.is_empty() && result.warnings.is_empty() {
        println!("No hay cambios aplicables.");
    }
    for outcome in result.outcomes {
        if outcome.changed_files.is_empty() {
            println!(
                "{}",
                ui.done(&format!("{} · sin cambios", outcome.project_root))
            );
        } else {
            println!(
                "{}",
                ui.done(&format!("{} · sincronizado", outcome.project_root))
            );
            for file in outcome.changed_files {
                println!("  {}", ui.bullet(&file).trim_start());
            }
            if let Some(backup) = outcome.backup {
                println!("  {}", ui.field("Backup", &backup).trim_start());
            }
        }
    }
    for warning in &result.warnings {
        println!("{}", ui.warning(warning));
    }
    if !result.warnings.is_empty() {
        return Err("La sincronización terminó parcialmente; revisa los avisos".to_owned());
    }
    Ok(())
}

pub(super) fn integrate(
    ui: &Ui,
    path: &Path,
    client: &str,
    refresh: bool,
    apply: bool,
) -> Result<(), String> {
    let clients = crate::features::mcp_setup::McpClient::parse(client)?;
    let plan = crate::features::mcp_setup::plan_mcp_integration(path, &clients, refresh)?;
    println!(
        "{}",
        ui.heading(
            &format!("Integración MCP: {}", plan.project_root),
            "Integración MCP",
            &plan.project_root
        )
    );
    println!("{}", ui.field("Clientes", &plan.client_labels.join(", ")));
    for action in &plan.actions {
        println!("{}", ui.bullet(action));
    }
    if !apply {
        for warning in &plan.warnings {
            println!("{}", ui.warning(warning));
        }
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("{}", ui.blocked(reason));
        if apply {
            return Err("Integración MCP bloqueada; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if !apply {
        println!(
            "{}",
            ui.hint(&format!(
                "Vista previa: usa `integrate {} --client {client}{} --apply` para configurar.",
                path.display(),
                if refresh { " --refresh" } else { "" }
            ))
        );
        return Ok(());
    }
    let result = crate::features::mcp_setup::apply_mcp_integration(&plan)?;
    for action in result.completed {
        println!("{}", ui.done(&action));
    }
    for warning in &result.warnings {
        println!("{}", ui.warning(warning));
    }
    if !result.warnings.is_empty() {
        return Err("La integración MCP se aplicó parcialmente; revisa los avisos".to_owned());
    }
    Ok(())
}

pub(super) fn trust(ui: &Ui, path: &Path, desired: bool, apply: bool) -> Result<(), String> {
    let plan = crate::features::project_trust::plan_project_trust(path, desired)?;
    let operation = if desired { "Trust" } else { "Revocar trust" };
    println!(
        "{}",
        ui.heading(
            &format!("{operation}: {}", plan.project_root),
            operation,
            &plan.project_root
        )
    );
    let current = if plan.currently_trusted {
        "confiable"
    } else {
        "no confiable"
    };
    println!(
        "{}",
        ui.field(
            "Estado actual",
            &ui.paint(
                if plan.currently_trusted {
                    Tone::Green
                } else {
                    Tone::Yellow
                },
                current
            )
        )
    );
    for action in &plan.actions {
        println!("{}", ui.bullet(action));
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("{}", ui.blocked(reason));
        if apply {
            return Err("Cambio de confianza bloqueado; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if apply {
        let changed = plan.changes_state;
        let backup = crate::features::project_trust::apply_project_trust(&plan)?;
        if changed {
            println!(
                "\n{}",
                ui.paint(
                    Tone::Green,
                    "Decisión de confianza guardada explícitamente."
                )
            );
        } else {
            println!("\nSin cambios; la confianza ya estaba en ese estado.");
        }
        if let Some(backup) = backup {
            println!("{}", ui.field("Backup", &backup));
        }
    } else {
        let action = if desired { "trust" } else { "untrust" };
        println!(
            "{}",
            ui.hint(&format!(
                "Vista previa: usa `workspace {action} {} --apply` para guardar.",
                path.display()
            ))
        );
    }
    Ok(())
}

pub(super) fn rollback(ui: &Ui, path: &Path, apply: bool) -> Result<(), String> {
    let plan = crate::features::project_rollback::plan_project_rollback(path)?;
    println!(
        "{}",
        ui.heading(
            &format!("Rollback del proyecto: {}", plan.project_root),
            "Rollback del proyecto",
            &plan.project_root
        )
    );
    println!("{}", ui.field("Backup seleccionado", &plan.backup_path));
    for action in &plan.actions {
        println!("{}", ui.bullet(action));
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("{}", ui.blocked(reason));
        if apply {
            return Err("Rollback bloqueado; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if apply {
        let (actions, safety_backup) =
            crate::features::project_rollback::apply_project_rollback(&plan)?;
        println!(
            "\n{}",
            ui.section("Cambios aplicados:").trim_start_matches('\n')
        );
        for action in actions {
            println!("{}", ui.done(&action));
        }
        if let Some(backup) = safety_backup {
            println!("{}", ui.field("Backup de seguridad", &backup));
        }
    } else {
        println!(
            "{}",
            ui.hint(&format!(
                "Vista previa: usa `rollback {} --apply` para restaurar.",
                path.display()
            ))
        );
    }
    Ok(())
}

pub(super) fn init(ui: &Ui, path: &Path, apply: bool, minimal: bool) -> Result<(), String> {
    let plan = crate::features::project_setup::plan_project_init(path, minimal)?;
    let mode = if minimal { "compacta" } else { "completa" };
    println!(
        "{}",
        ui.heading(
            &format!("Inicialización {mode}: {}", plan.project_root),
            &format!("Inicialización {mode}"),
            &plan.project_root
        )
    );
    for action in &plan.actions {
        println!("{}", ui.bullet(action));
    }
    if !apply {
        for warning in &plan.warnings {
            println!("{}", ui.warning(warning));
        }
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("{}", ui.blocked(reason));
        if apply {
            return Err("Init bloqueado; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if apply {
        let application = crate::features::project_setup::apply_project_init(&plan)?;
        println!(
            "\n{}",
            ui.section("Cambios aplicados:").trim_start_matches('\n')
        );
        for action in application.completed_actions {
            println!("{}", ui.done(&action));
        }
        for backup in application.backups {
            println!("{}", ui.field("Backup", &backup));
        }
        for warning in application.warnings {
            println!("{}", ui.warning(&warning));
        }
    } else {
        let minimal_flag = if minimal { " --minimal" } else { "" };
        println!(
            "{}",
            ui.hint(&format!(
                "Vista previa: usa `init {}{minimal_flag} --apply` para aplicar el plan.",
                path.display()
            ))
        );
    }
    Ok(())
}

pub(super) fn status(ui: &Ui, path: &Path, json: bool) -> Result<(), String> {
    let overview = inspect_project(path)?;
    if json {
        let payload = serde_json::to_string_pretty(&overview)
            .map_err(|error| format!("No se pudo generar el resumen JSON: {error}"))?;
        println!("{payload}");
    } else if ui.is_rich() {
        status_rich(ui, &overview);
    } else {
        status_plain(&overview);
    }
    Ok(())
}

fn status_rich(ui: &Ui, o: &ProjectOverview) {
    println!("{}", ui.heading("", &o.project.name, &o.project.root));
    let mark = |state: Option<bool>| match state {
        Some(true) => ui.strong(Tone::Green, "✓"),
        Some(false) => ui.strong(Tone::Red, "✗"),
        None => ui.dim("·"),
    };
    let row = |state: Option<bool>, label: &str, value: String| {
        println!("  {} {}  {value}", mark(state), pad(label, 15));
    };
    row(
        Some(o.project.git_repository),
        "Git",
        yes_no(o.project.git_repository).to_owned(),
    );
    let ok = matches!(
        (
            o.workspace.config_exists,
            o.workspace.config_parseable,
            o.workspace.config_supported
        ),
        (true, Some(true), Some(true))
    );
    row(Some(ok), "Configuración", config_summary(o));
    row(
        Some(o.workspace.trusted),
        "Confianza local",
        if o.workspace.trusted {
            "sí, puede ejecutar agentes".into()
        } else {
            "no, ningún agente ejecuta nada".into()
        },
    );
    println!(
        "  {} {}  AGENTS.md {}   CLAUDE.md {}",
        mark(Some(
            o.project.has_agents_guide && o.project.has_claude_guide
        )),
        pad("Guías", 15),
        mark(Some(o.project.has_agents_guide)),
        mark(Some(o.project.has_claude_guide)),
    );
    println!("\n  {}", ui.strong(Tone::Cyan, "Tecnología"));
    let tags = |items: &[String], tone: Tone| {
        if items.is_empty() {
            ui.dim("ninguno detectado")
        } else {
            items
                .iter()
                .map(|i| ui.badge(tone, i))
                .collect::<Vec<_>>()
                .join(" ")
        }
    };
    let d = &o.detection;
    for (label, value) in [
        (
            "Marcadores",
            if o.project.detected_markers.is_empty() {
                ui.dim("ninguno detectado")
            } else {
                o.project.detected_markers.join(", ")
            },
        ),
        ("Stacks", tags(&d.stacks, Tone::Cyan)),
        ("Lenguajes", tags(&d.languages, Tone::Purple)),
        ("Gestores", tags(&d.package_managers, Tone::Orange)),
        ("Infraestructura", tags(&d.infrastructure, Tone::Green)),
        ("Perfil", ui.badge(Tone::Yellow, &d.profile)),
    ] {
        println!("    {}  {value}", ui.dim(&pad(label, 15)));
    }
    for rule in &d.profile_rules {
        println!("    {} {}", ui.dim("regla"), rule);
    }
    if let Some(error) = &o.workspace.config_validation_error {
        println!(
            "\n{}",
            ui.warning(&format!("Validación de configuración: {error}"))
        );
    }
    if !o.workspace.config_exists {
        println!("{}", ui.hint("Siguiente paso: `chxchx-tech init . --apply` prepara el proyecto (antes muestra la vista previa)."));
    } else if !o.workspace.trusted {
        println!("{}", ui.hint("Siguiente paso: `chxchx-tech workspace trust .` para permitir agentes y terminales aquí."));
    }
    println!();
}

fn status_plain(overview: &ProjectOverview) {
    println!("Proyecto: {}", overview.project.name);
    println!("Ruta: {}", overview.project.root);
    println!("Git: {}", yes_no(overview.project.git_repository));
    println!("Configuración: {}", config_summary(overview));
    println!("Confianza local: {}", yes_no(overview.workspace.trusted));
    println!(
        "Guías: AGENTS.md {}; CLAUDE.md {}",
        yes_no(overview.project.has_agents_guide),
        yes_no(overview.project.has_claude_guide)
    );
    println!(
        "Marcadores: {}",
        if overview.project.detected_markers.is_empty() {
            "ninguno detectado".to_owned()
        } else {
            overview.project.detected_markers.join(", ")
        }
    );
    println!("Stacks: {}", list_or_none(&overview.detection.stacks));
    println!("Lenguajes: {}", list_or_none(&overview.detection.languages));
    println!(
        "Gestores de paquetes: {}",
        list_or_none(&overview.detection.package_managers)
    );
    println!(
        "Infraestructura: {}",
        list_or_none(&overview.detection.infrastructure)
    );
    println!("Perfil: {}", overview.detection.profile);
    for rule in &overview.detection.profile_rules {
        println!("Regla del perfil: {rule}");
    }
    if let Some(error) = &overview.workspace.config_validation_error {
        println!("Validación de configuración: {error}");
    }
}

fn config_summary(overview: &ProjectOverview) -> String {
    match (
        overview.workspace.config_exists,
        overview.workspace.config_parseable,
        overview.workspace.config_supported,
    ) {
        (false, _, _) => "sin inicializar".to_owned(),
        (true, Some(false), _) => "TOML mal formado".to_owned(),
        (true, Some(true), Some(false)) => "TOML legible, configuración no compatible".to_owned(),
        (true, Some(true), Some(true)) => format!(
            "compatible (versión {})",
            overview
                .workspace
                .config_version
                .map_or_else(|| "desconocida".to_owned(), |version| version.to_string())
        ),
        (true, Some(true), None) => "TOML legible, no se pudo validar".to_owned(),
        (true, None, _) => "no se pudo leer".to_owned(),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "sí" } else { "no" }
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "ninguno detectado".to_owned()
    } else {
        values.join(", ")
    }
}
