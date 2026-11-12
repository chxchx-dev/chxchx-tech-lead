use crate::features::project_overview::inspect_project;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "chxchx-tech",
    version,
    about = "Control plane local para proyectos de desarrollo asistido"
)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resume el proyecto actual sin escribir ni iniciar procesos.
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Escribe el contrato JSON compartido por las interfaces.
        #[arg(long)]
        json: bool,
    },
    /// Previsualiza o inicializa archivos del proyecto.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Muestra el plan sin escribir (también es el comportamiento predeterminado).
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        /// Aplica el plan; solo crea archivos/directorios que faltan.
        #[arg(long)]
        apply: bool,
        /// Inicializa solo configuración, memoria y registro local.
        #[arg(long)]
        minimal: bool,
    },
    /// Previsualiza o restaura los archivos administrados desde el último backup.
    Rollback {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Restaura el último backup después de mostrar el plan.
        #[arg(long)]
        apply: bool,
    },
    /// Gestiona la confianza local del proyecto.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Consulta proyectos registrados en el estado local.
    Projects {
        #[command(subcommand)]
        command: ProjectsCommand,
    },
    /// Previsualiza o configura servidores MCP para clientes instalados.
    Integrate {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// all, claude, codex u opencode.
        #[arg(long, default_value = "all")]
        client: String,
        /// Sustituye las definiciones basic-memory/serena ya existentes.
        #[arg(long)]
        refresh: bool,
        /// Aplica la vista previa y permite invocar los CLIs oficiales.
        #[arg(long)]
        apply: bool,
    },
    /// Diagnostica herramientas y configuración del proyecto sin ejecutar comandos.
    Doctor {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Abre la interfaz de terminal del producto.
    #[cfg(feature = "tui")]
    Tui {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Abre el Studio del producto.
    #[cfg(feature = "studio")]
    Studio {
        #[command(subcommand)]
        command: StudioCommand,
    },
    /// Muestra la versión común del producto.
    Version,
}

#[cfg(feature = "studio")]
#[derive(Debug, Subcommand)]
enum StudioCommand {
    Open {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    /// Previsualiza o registra confianza para la configuración actual.
    Trust {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Guarda la decisión explícita después de mostrar el plan.
        #[arg(long)]
        apply: bool,
    },
    /// Previsualiza o revoca la confianza local del proyecto.
    Untrust {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Guarda la decisión explícita después de mostrar el plan.
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectsCommand {
    /// Lista los proyectos registrados y su disponibilidad.
    List,
    /// Muestra el último proyecto activado, si sigue disponible.
    Current,
    /// Previsualiza la sincronización de bloques administrados.
    Sync {
        /// Aplica los cambios después de mostrar el plan.
        #[arg(long)]
        apply: bool,
    },
}

pub fn run() -> Result<(), String> {
    let args = Arguments::parse();
    match args.command {
        Command::Status { path, json } => show_status(&path, json),
        Command::Init {
            path,
            dry_run,
            apply,
            minimal,
        } => show_init(&path, apply && !dry_run, minimal),
        Command::Rollback { path, apply } => show_rollback(&path, apply),
        Command::Workspace { command } => match command {
            WorkspaceCommand::Trust { path, apply } => show_trust(&path, true, apply),
            WorkspaceCommand::Untrust { path, apply } => show_trust(&path, false, apply),
        },
        Command::Projects { command } => match command {
            ProjectsCommand::List => show_projects_list(),
            ProjectsCommand::Current => show_current_project(),
            ProjectsCommand::Sync { apply } => show_projects_sync(apply),
        },
        Command::Integrate {
            path,
            client,
            refresh,
            apply,
        } => show_integrate(&path, &client, refresh, apply),
        Command::Doctor { path } => show_doctor(&path),
        #[cfg(feature = "tui")]
        Command::Tui { path } => crate::interfaces::tui::run(path),
        #[cfg(feature = "studio")]
        Command::Studio { command } => match command {
            StudioCommand::Open { path } => crate::interfaces::studio::run(path),
        },
        Command::Version => {
            println!("ChxChx Tech {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn show_doctor(path: &std::path::Path) -> Result<(), String> {
    let report = crate::features::doctor::diagnose_project(path)?;
    println!("{}", report.render());
    Ok(())
}

fn show_projects_list() -> Result<(), String> {
    let catalog = crate::features::project_catalog::inspect_registered_projects()?;
    if catalog.projects.is_empty() {
        println!("No hay proyectos registrados.");
        return Ok(());
    }
    println!("Alias | Nombre | Perfil | Stacks | Estado | Ruta");
    for project in catalog.projects {
        let state = match (project.available, project.is_current) {
            (true, true) => "disponible · último",
            (true, false) => "disponible",
            (false, _) => "no encontrado",
        };
        println!(
            "{} | {} | {} | {} | {} | {}",
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
        );
    }
    Ok(())
}

fn show_current_project() -> Result<(), String> {
    let catalog = crate::features::project_catalog::inspect_registered_projects()?;
    if let Some(project) = catalog.current {
        println!("✓ {}: {}", project.alias, project.path);
    } else {
        println!("No hay un último proyecto disponible.");
    }
    Ok(())
}

fn show_projects_sync(apply: bool) -> Result<(), String> {
    let plan = crate::features::project_sync::plan_projects_sync()?;
    println!("Sincronización de proyectos registrados:");
    for entry in &plan.entries {
        println!("\n{} · {}", entry.alias, entry.project_root);
        if let Some(reason) = &entry.blocked_reason {
            println!("  Omitido: {reason}");
        } else if entry.changes.is_empty() {
            println!("  Sin cambios.");
        } else {
            for action in &entry.changes {
                println!("  • {action}");
            }
        }
    }
    if !apply {
        for warning in &plan.warnings {
            println!("Aviso: {warning}");
        }
    }
    if !apply {
        if plan.entries.iter().any(|entry| !entry.changes.is_empty()) {
            println!("\nVista previa: vuelve a ejecutar con `projects sync --apply`.");
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
            println!("✓ {} · sin cambios", outcome.project_root);
        } else {
            println!("✓ {} · sincronizado", outcome.project_root);
            for file in outcome.changed_files {
                println!("  • {file}");
            }
            if let Some(backup) = outcome.backup {
                println!("  Backup: {backup}");
            }
        }
    }
    for warning in &result.warnings {
        println!("Aviso: {warning}");
    }
    if !result.warnings.is_empty() {
        return Err("La sincronización terminó parcialmente; revisa los avisos".to_owned());
    }
    Ok(())
}

fn show_integrate(
    path: &std::path::Path,
    client: &str,
    refresh: bool,
    apply: bool,
) -> Result<(), String> {
    let clients = crate::features::mcp_setup::McpClient::parse(client)?;
    let plan = crate::features::mcp_setup::plan_mcp_integration(path, &clients, refresh)?;
    println!("Integración MCP: {}", plan.project_root);
    println!("Clientes: {}", plan.client_labels.join(", "));
    for action in &plan.actions {
        println!("• {action}");
    }
    if !apply {
        for warning in &plan.warnings {
            println!("Aviso: {warning}");
        }
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("No se puede aplicar: {reason}");
        if apply {
            return Err("Integración MCP bloqueada; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if !apply {
        println!(
            "\nVista previa: usa `integrate {} --client {client}{} --apply` para configurar.",
            path.display(),
            if refresh { " --refresh" } else { "" }
        );
        return Ok(());
    }
    let result = crate::features::mcp_setup::apply_mcp_integration(&plan)?;
    for action in result.completed {
        println!("✓ {action}");
    }
    for warning in &result.warnings {
        println!("Aviso: {warning}");
    }
    if !result.warnings.is_empty() {
        return Err("La integración MCP se aplicó parcialmente; revisa los avisos".to_owned());
    }
    Ok(())
}

fn show_trust(path: &std::path::Path, desired: bool, apply: bool) -> Result<(), String> {
    let plan = crate::features::project_trust::plan_project_trust(path, desired)?;
    let operation = if desired { "Trust" } else { "Revocar trust" };
    println!("{operation}: {}", plan.project_root);
    println!(
        "Estado actual: {}",
        if plan.currently_trusted {
            "confiable"
        } else {
            "no confiable"
        }
    );
    for action in &plan.actions {
        println!("• {action}");
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("No se puede aplicar: {reason}");
        if apply {
            return Err("Cambio de confianza bloqueado; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if apply {
        let changed = plan.changes_state;
        let backup = crate::features::project_trust::apply_project_trust(&plan)?;
        if changed {
            println!("\nDecisión de confianza guardada explícitamente.");
        } else {
            println!("\nSin cambios; la confianza ya estaba en ese estado.");
        }
        if let Some(backup) = backup {
            println!("Backup: {backup}");
        }
    } else {
        let action = if desired { "trust" } else { "untrust" };
        println!(
            "\nVista previa: usa `workspace {action} {} --apply` para guardar.",
            path.display()
        );
    }
    Ok(())
}

fn show_rollback(path: &std::path::Path, apply: bool) -> Result<(), String> {
    let plan = crate::features::project_rollback::plan_project_rollback(path)?;
    println!("Rollback del proyecto: {}", plan.project_root);
    println!("Backup seleccionado: {}", plan.backup_path);
    for action in &plan.actions {
        println!("• {action}");
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("No se puede aplicar: {reason}");
        if apply {
            return Err("Rollback bloqueado; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if apply {
        let (actions, safety_backup) =
            crate::features::project_rollback::apply_project_rollback(&plan)?;
        println!("\nCambios aplicados:");
        for action in actions {
            println!("✓ {action}");
        }
        if let Some(backup) = safety_backup {
            println!("Backup de seguridad: {backup}");
        }
    } else {
        println!(
            "\nVista previa: usa `rollback {} --apply` para restaurar.",
            path.display()
        );
    }
    Ok(())
}

fn show_init(path: &std::path::Path, apply: bool, minimal: bool) -> Result<(), String> {
    let plan = crate::features::project_setup::plan_project_init(path, minimal)?;
    let mode = if minimal { "compacta" } else { "completa" };
    println!("Inicialización {mode}: {}", plan.project_root);
    for action in &plan.actions {
        println!("• {action}");
    }
    if !apply {
        for warning in &plan.warnings {
            println!("Aviso: {warning}");
        }
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("No se puede aplicar: {reason}");
        if apply {
            return Err("Init bloqueado; no se aplicaron cambios".to_owned());
        }
        return Ok(());
    }
    if apply {
        let application = crate::features::project_setup::apply_project_init(&plan)?;
        println!("\nCambios aplicados:");
        for action in application.completed_actions {
            println!("✓ {action}");
        }
        for backup in application.backups {
            println!("Backup: {backup}");
        }
        for warning in application.warnings {
            println!("Aviso: {warning}");
        }
    } else {
        let minimal_flag = if minimal { " --minimal" } else { "" };
        println!(
            "\nVista previa: usa `init {}{minimal_flag} --apply` para aplicar el plan.",
            path.display(),
        );
    }
    Ok(())
}

fn show_status(path: &std::path::Path, json: bool) -> Result<(), String> {
    let overview = inspect_project(path)?;
    if json {
        let payload = serde_json::to_string_pretty(&overview)
            .map_err(|error| format!("No se pudo generar el resumen JSON: {error}"))?;
        println!("{payload}");
    } else {
        println!("Proyecto: {}", overview.project.name);
        println!("Ruta: {}", overview.project.root);
        println!("Git: {}", yes_no(overview.project.git_repository));
        println!("Configuración: {}", config_summary(&overview));
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
    Ok(())
}

fn config_summary(overview: &crate::ProjectOverview) -> String {
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
