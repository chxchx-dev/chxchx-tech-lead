use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[path = "cli_catalog.rs"]
mod catalog;
#[path = "cli_style.rs"]
mod style;
#[path = "cli_view.rs"]
mod view;

use style::Ui;

/// Colors of `--help` (clap only paints them when the output is a terminal that wants color).
const HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Blue.on_default())
    .valid(AnsiColor::Green.on_default())
    .invalid(AnsiColor::Red.on_default().effects(Effects::BOLD))
    .error(AnsiColor::Red.on_default().effects(Effects::BOLD));

const EXAMPLES: &str = "Ejemplos:
  chxchx-tech status .             Resume el proyecto sin escribir nada
  chxchx-tech init . --apply       Prepara el proyecto (antes, sin --apply, solo la vista previa)
  chxchx-tech workspace trust .    Decide si los agentes pueden ejecutar procesos aquí
  chxchx-tech skills list .        Skills recomendadas para tu stack
  chxchx-tech doctor .             Qué herramientas faltan
  chxchx-tech tui .                El mismo panel en una terminal interactiva

Todo cambio muestra una vista previa; `--apply` la confirma. `status --json` imprime el contrato versionado.";

#[derive(Debug, Parser)]
#[command(
    name = "chxchx-tech",
    version,
    about = "Control plane local para proyectos de desarrollo asistido",
    styles = HELP_STYLES,
    after_help = EXAMPLES
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
    /// Gestiona las skills habilitadas del proyecto.
    Skills {
        #[command(subcommand)]
        command: catalog::SkillsCommand,
    },
    /// Consulta y aplica Tech Packs (conjuntos de skills por stack).
    Packs {
        #[command(subcommand)]
        command: catalog::PacksCommand,
    },
    /// Instala o retira Mods (flujos de trabajo para agentes) en el proyecto.
    Mods {
        #[command(subcommand)]
        command: catalog::ModsCommand,
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

/// Prints a failure the way the terminal can show it: `Error: …`, in red when it can.
pub fn report_error(error: &str) {
    let ui = Ui::stderr();
    eprintln!("{} {error}", ui.strong(style::Tone::Red, "Error:"));
}

pub fn run() -> Result<(), String> {
    let args = Arguments::parse();
    let ui = Ui::stdout();
    match args.command {
        Command::Status { path, json } => view::status(&ui, &path, json),
        Command::Init {
            path,
            dry_run,
            apply,
            minimal,
        } => view::init(&ui, &path, apply && !dry_run, minimal),
        Command::Rollback { path, apply } => view::rollback(&ui, &path, apply),
        Command::Workspace { command } => match command {
            WorkspaceCommand::Trust { path, apply } => view::trust(&ui, &path, true, apply),
            WorkspaceCommand::Untrust { path, apply } => view::trust(&ui, &path, false, apply),
        },
        Command::Projects { command } => match command {
            ProjectsCommand::List => view::projects_list(&ui),
            ProjectsCommand::Current => view::projects_current(&ui),
            ProjectsCommand::Sync { apply } => view::projects_sync(&ui, apply),
        },
        Command::Integrate {
            path,
            client,
            refresh,
            apply,
        } => view::integrate(&ui, &path, &client, refresh, apply),
        Command::Skills { command } => catalog::skills(&ui, command),
        Command::Packs { command } => catalog::packs(&ui, command),
        Command::Mods { command } => catalog::mods(&ui, command),
        Command::Doctor { path } => view::doctor(&ui, &path),
        #[cfg(feature = "tui")]
        Command::Tui { path } => crate::interfaces::tui::run(path),
        #[cfg(feature = "studio")]
        Command::Studio { command } => match command {
            StudioCommand::Open { path } => crate::interfaces::studio::run(path),
        },
        Command::Version => {
            view::version(&ui);
            Ok(())
        }
    }
}
