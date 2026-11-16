use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;

#[path = "cli_catalog.rs"]
mod catalog;
#[path = "cli_connectors.rs"]
mod connectors;
#[path = "cli_launch.rs"]
mod launch;
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

const EXAMPLES: &str = "Con un solo comando:
  chxchx-tech                      Abre ChxChx (Studio con escritorio; si no, el TUI)
  chxchx-tech ~/mi-proyecto        Abre esa carpeta
  chxchx-tech studio .             Studio     chxchx-tech tui .   TUI

Ejemplos:
  chxchx-tech status .             Resume el proyecto sin escribir nada
  chxchx-tech init . --apply       Prepara el proyecto (antes, sin --apply, solo la vista previa)
  chxchx-tech workspace trust .    Decide si los agentes pueden ejecutar procesos aquí
  chxchx-tech skills list .        Skills recomendadas para tu stack
  chxchx-tech doctor .             Qué herramientas faltan
  chxchx-tech tui .                El mismo panel en una terminal interactiva

Atajos: st (status) · dr (doctor) · sk (skills) · ws (workspace) · pr (projects) · t (tui) · gui (studio).
Comandos del agente y del puente (agent, bridge, local-models, run, ci, deploy…) se ejecutan con el CLI de Python que acompaña a la instalación.
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
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resume el proyecto actual sin escribir ni iniciar procesos.
    #[command(visible_alias = "st")]
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
    #[command(visible_alias = "ws")]
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Consulta proyectos registrados en el estado local.
    #[command(visible_alias = "pr")]
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
    /// Conecta tus agentes con apps y datos (Excel, Power BI, bases de datos…) por MCP.
    #[command(visible_alias = "conn", alias = "connector")]
    Connectors {
        #[command(subcommand)]
        command: connectors::ConnectorsCommand,
    },
    /// Gestiona las skills habilitadas del proyecto.
    #[command(visible_alias = "sk", alias = "skill")]
    Skills {
        #[command(subcommand)]
        command: catalog::SkillsCommand,
    },
    /// Consulta y aplica Tech Packs (conjuntos de skills por stack).
    #[command(alias = "pack")]
    Packs {
        #[command(subcommand)]
        command: catalog::PacksCommand,
    },
    /// Instala o retira Mods (flujos de trabajo para agentes) en el proyecto.
    #[command(alias = "mod")]
    Mods {
        #[command(subcommand)]
        command: catalog::ModsCommand,
    },
    /// Diagnostica herramientas y configuración del proyecto sin ejecutar comandos.
    #[command(visible_alias = "dr")]
    Doctor {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Abre la interfaz de terminal del producto.
    #[cfg(feature = "tui")]
    #[command(visible_alias = "t")]
    Tui {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Abre el Studio del producto (`studio .`; `studio open .` sigue valiendo).
    #[cfg(feature = "studio")]
    #[command(visible_alias = "gui")]
    Studio {
        /// Carpeta del proyecto; por defecto la actual.
        #[arg(num_args = 0..=2)]
        paths: Vec<PathBuf>,
    },
    /// Muestra la versión común del producto.
    Version,
    /// Lo demás lo resuelve el CLI heredado (agent, bridge, local-models…) o abre una carpeta.
    #[command(external_subcommand)]
    External(Vec<OsString>),
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

/// The program: `chxchx-tech` and its short name `chxchx` both start here.
pub fn main() {
    restore_default_sigpipe();
    if let Err(error) = run() {
        report_error(&error);
        std::process::exit(1);
    }
}

/// Rust ignores SIGPIPE, so `chxchx-tech skills list | head` would panic on the closed pipe.
/// A command-line tool should simply stop, like every other one.
#[cfg(unix)]
fn restore_default_sigpipe() {
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    // SAFETY: installing the default disposition for SIGPIPE at startup, before any thread exists.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// Opens ChxChx in the best interface for where it runs, on `path`.
#[cfg_attr(not(any(feature = "studio", feature = "tui")), allow(unused_variables))]
fn open_default(path: PathBuf) -> Result<(), String> {
    match launch::choice_here() {
        #[cfg(feature = "studio")]
        launch::Choice::Studio => {
            println!("Abriendo ChxChx Studio en {}…", path.display());
            launch::spawn_studio(&path)
        }
        #[cfg(feature = "tui")]
        launch::Choice::Tui => crate::interfaces::tui::run(path),
        _ => {
            use clap::CommandFactory;
            Arguments::command()
                .print_help()
                .map_err(|error| error.to_string())
        }
    }
}

pub fn run() -> Result<(), String> {
    let args = Arguments::parse();
    let ui = Ui::stdout();
    let Some(command) = args.command else {
        return open_default(PathBuf::from("."));
    };
    match command {
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
        Command::Connectors { command } => connectors::run(&ui, command),
        Command::Skills { command } => catalog::skills(&ui, command),
        Command::Packs { command } => catalog::packs(&ui, command),
        Command::Mods { command } => catalog::mods(&ui, command),
        Command::Doctor { path } => view::doctor(&ui, &path),
        #[cfg(feature = "tui")]
        Command::Tui { path } => crate::interfaces::tui::run(path),
        #[cfg(feature = "studio")]
        Command::Studio { paths } => {
            let mut paths = paths.into_iter().peekable();
            // `studio open <path>` was the old spelling.
            if paths.peek().is_some_and(|first| first == "open") {
                paths.next();
            }
            crate::interfaces::studio::run(paths.next().unwrap_or_else(|| PathBuf::from(".")))
        }
        Command::Version => {
            view::version(&ui);
            Ok(())
        }
        Command::External(arguments) => {
            if let Some(folder) = launch::folder_argument(&arguments) {
                return open_default(folder);
            }
            match launch::delegate(&arguments)? {
                0 => Ok(()),
                code => std::process::exit(code),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(words: &[&str]) -> Arguments {
        Arguments::try_parse_from(std::iter::once("chxchx-tech").chain(words.iter().copied()))
            .expect("the words are a valid command line")
    }

    #[test]
    fn short_names_reach_the_same_commands() {
        assert!(matches!(
            parse(&["st", "."]).command,
            Some(Command::Status { .. })
        ));
        assert!(matches!(
            parse(&["dr"]).command,
            Some(Command::Doctor { .. })
        ));
        assert!(matches!(
            parse(&["sk", "list"]).command,
            Some(Command::Skills { .. })
        ));
        assert!(matches!(
            parse(&["skill", "list"]).command,
            Some(Command::Skills { .. })
        ));
        assert!(matches!(
            parse(&["pack", "list"]).command,
            Some(Command::Packs { .. })
        ));
        assert!(matches!(
            parse(&["mod", "list"]).command,
            Some(Command::Mods { .. })
        ));
        assert!(matches!(
            parse(&["ws", "trust"]).command,
            Some(Command::Workspace { .. })
        ));
        assert!(matches!(
            parse(&["pr", "list"]).command,
            Some(Command::Projects { .. })
        ));
    }

    #[test]
    fn nothing_at_all_means_open_everything() {
        assert!(parse(&[]).command.is_none());
    }

    #[cfg(feature = "studio")]
    #[test]
    fn studio_takes_a_folder_directly_and_still_understands_open() {
        for words in [
            &["studio"][..],
            &["studio", "."],
            &["gui", "."],
            &["studio", "open", "."],
        ] {
            assert!(
                matches!(parse(words).command, Some(Command::Studio { .. })),
                "{words:?}"
            );
        }
    }

    #[test]
    fn what_the_legacy_cli_owns_is_passed_through_untouched() {
        match parse(&["agent", "chat", "claude", "--live", "-x"]).command {
            Some(Command::External(args)) => {
                assert_eq!(args.len(), 5);
                assert_eq!(args[0], "agent");
                assert_eq!(args[4], "-x", "flags travel with it");
            }
            other => panic!("expected a pass-through, got {other:?}"),
        }
    }
}
