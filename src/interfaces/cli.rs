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
#[path = "cli_local_models.rs"]
mod local_models;
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
  chxchx-tech workspace memory-prune . --dry-run  Previsualiza capturas automáticas inválidas
  chxchx-tech skills list .        Skills recomendadas para tu stack
  chxchx-tech doctor .             Qué herramientas faltan
  chxchx-tech resources .          RAM, swap y procesos de proyectos registrados
  chxchx-tech tui .                El mismo panel en una terminal interactiva

Atajos: st (status) · dr (doctor) · sk (skills) · ws (workspace) · pr (projects) · t (tui) · gui (studio).
Modelos locales: `local-models status|setup|remove`. Los demás comandos pendientes de portar se mantienen disponibles durante la migración.
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
    /// Diagnostica y configura modelos locales de Ollama para OpenCode.
    #[command(name = "local-models")]
    LocalModels {
        #[command(subcommand)]
        command: local_models::LocalModelsCommand,
    },
    /// Lee o reinicia el historial persistido de un chat visual.
    #[command(subcommand_required = false)]
    Agent {
        #[command(subcommand)]
        command: Option<AgentCommand>,
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
    /// Administra procesos configurados en workspace.processes.
    Process {
        #[command(subcommand)]
        command: ProcessCommand,
    },
    /// Consulta RAM, swap y recursos de procesos administrados.
    Resources {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Imprime el contrato JSON versionado `chxchx.resources-overview`.
        #[arg(long)]
        json: bool,
    },
    /// Instala las herramientas que usan las integraciones de memoria y código (Basic Memory, Serena).
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },
    /// Una carpeta se abre; cualquier otra palabra es un comando del CLI anterior ya retirado.
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    /// Inicia el workspace y sus procesos configurados para auto_start.
    Start {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Detiene los procesos administrados y marca el workspace como detenido.
    Stop {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Detiene el workspace y guarda el estado SUSPENDED.
    Suspend {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Reanuda el workspace y sus procesos auto_start.
    Resume {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
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
    /// Subcomandos del CLI anterior (attach, terminal, open…): se retiraron.
    #[command(external_subcommand)]
    External(Vec<OsString>),
    /// Muestra los errores recientes guardados en esta máquina (sin secretos).
    Errors {
        /// Solo los de este proyecto; sin ruta, los de todos.
        path: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Guarda un error en el registro local; el mensaje se recorta y se le quitan los secretos.
    ReportError {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Qué se estaba haciendo (por ejemplo `start`).
        #[arg(long)]
        operation: String,
        #[arg(long)]
        message: String,
    },
    /// Previsualiza o limpia capturas automáticas inválidas de memoria explícita.
    MemoryPrune {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Muestra las entradas que se retirarían sin modificar el archivo.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ToolsCommand {
    /// Instala con `uv` lo que falte; lo ya instalado no se toca.
    Install {
        /// Solo estas (`basic-memory`, `serena`); sin nombres, las dos.
        names: Vec<String>,
        /// Muestra los comandos sin ejecutarlos.
        #[arg(long)]
        dry_run: bool,
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
    /// Subcomandos del CLI anterior (prepare): se retiraron.
    #[command(external_subcommand)]
    External(Vec<OsString>),
    /// Suspende los workspaces activos y activa otro proyecto registrado.
    Switch {
        /// Alias del proyecto (véase `projects list`).
        alias: String,
        /// Muestra la transición sin ejecutarla.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ProcessCommand {
    /// Lista procesos configurados y su estado.
    List {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Inicia un proceso después de comprobar la confianza del proyecto.
    Start {
        id: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Detiene un proceso administrado.
    Stop {
        id: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Muestra las últimas líneas del log de un proceso configurado.
    Log {
        id: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value_t = 300)]
        lines: usize,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// Conversación guardada del chat visual (`chxchx.chat-state` v1).
    ChatState {
        agent_id: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Olvida el historial guardado.
        #[arg(long)]
        reset: bool,
        /// Marca la sesión para comprobarla al continuar el chat.
        #[arg(long)]
        restore: bool,
    },
    /// Los subcomandos del CLI anterior: se retiraron y se explica dónde está cada función.
    #[command(external_subcommand)]
    External(Vec<OsString>),
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
            WorkspaceCommand::Start { path, dry_run } => {
                let action = crate::features::workspace_lifecycle::start(&path, dry_run)
                    .inspect_err(|error| view::remember_failure(&path, "workspace start", error))?;
                print_workspace_action(action);
                Ok(())
            }
            WorkspaceCommand::Stop { path, dry_run } => {
                let action = crate::features::workspace_lifecycle::stop(&path, dry_run)?;
                print_workspace_action(action);
                Ok(())
            }
            WorkspaceCommand::Suspend { path, dry_run } => {
                let action = crate::features::workspace_lifecycle::suspend(&path, dry_run)?;
                print_workspace_action(action);
                Ok(())
            }
            WorkspaceCommand::Resume { path, dry_run } => {
                let action = crate::features::workspace_lifecycle::resume(&path, dry_run)
                    .inspect_err(|error| {
                        view::remember_failure(&path, "workspace resume", error)
                    })?;
                print_workspace_action(action);
                Ok(())
            }
            WorkspaceCommand::External(arguments) => Err(launch::retired("workspace", &arguments)),
            WorkspaceCommand::Errors { path, limit } => view::errors(&ui, path.as_deref(), limit),
            WorkspaceCommand::ReportError {
                path,
                operation,
                message,
            } => view::report_error(&ui, &path, &operation, &message),
            WorkspaceCommand::Trust { path, apply } => view::trust(&ui, &path, true, apply),
            WorkspaceCommand::Untrust { path, apply } => view::trust(&ui, &path, false, apply),
            WorkspaceCommand::MemoryPrune { path, dry_run } => {
                let result = crate::features::explicit_memory_prune::prune(&path, dry_run)?;
                if result.removed.is_empty() {
                    println!(
                        "✓ La memoria explícita ya está limpia: {}",
                        result.path.display()
                    );
                    return Ok(());
                }
                let prefix = if dry_run {
                    "DRY RUN: se retirarían"
                } else {
                    "Retiradas"
                };
                println!(
                    "✓ {prefix} {} entrada(s) de {}",
                    result.removed.len(),
                    result.path.display()
                );
                for message in &result.removed {
                    let preview = message.chars().take(160).collect::<String>();
                    println!(
                        "  - {}",
                        if preview.is_empty() {
                            "(entrada vacía)"
                        } else {
                            &preview
                        }
                    );
                }
                if let Some(backup) = result.backup {
                    println!("Backup: {}", backup.display());
                }
                if dry_run {
                    println!("No se realizaron cambios (--dry-run).");
                }
                Ok(())
            }
        },
        Command::Projects { command } => match command {
            ProjectsCommand::List => view::projects_list(&ui),
            ProjectsCommand::Current => view::projects_current(&ui),
            ProjectsCommand::Sync { apply } => view::projects_sync(&ui, apply),
            ProjectsCommand::External(arguments) => Err(launch::retired("projects", &arguments)),
            ProjectsCommand::Switch { alias, dry_run } => {
                view::projects_switch(&ui, &alias, dry_run).inspect_err(|error| {
                    view::remember_failure(std::path::Path::new("."), "projects switch", error)
                })
            }
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
        Command::LocalModels { command } => local_models::run(command),
        Command::Agent { command } => match command {
            Some(AgentCommand::ChatState {
                agent_id,
                path,
                reset,
                restore,
            }) => {
                if reset {
                    crate::features::chat_state::reset(&path, &agent_id);
                }
                let payload =
                    crate::features::chat_state::contract(&path, &agent_id, restore && !reset);
                println!(
                    "{}",
                    serde_json::to_string(&payload).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some(AgentCommand::External(arguments)) => Err(launch::retired("agent", &arguments)),
            None => Err(launch::retired("agent", &[])),
        },
        Command::Tools { command } => match command {
            ToolsCommand::Install { names, dry_run } => view::tools_install(&ui, &names, dry_run),
        },
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
        Command::Process { command } => match command {
            ProcessCommand::List { path } => {
                let records = crate::features::workspace_processes::list(&path)?;
                if records.is_empty() {
                    println!("No hay procesos con estado guardado.");
                }
                for record in records {
                    println!(
                        "{}\t{:?}\tPID {}\t{}",
                        record.id,
                        record.status,
                        record
                            .pid
                            .map(|p| p.to_string())
                            .unwrap_or_else(|| "-".into()),
                        record.label
                    );
                }
                Ok(())
            }
            ProcessCommand::Start { id, path, dry_run } => {
                let action = crate::features::workspace_processes::start(&path, &id, dry_run)?;
                println!("{}", action.message);
                Ok(())
            }
            ProcessCommand::Stop { id, path, dry_run } => {
                let action = crate::features::workspace_processes::stop(&path, &id, dry_run)?;
                println!("{}", action.message);
                Ok(())
            }
            ProcessCommand::Log { id, path, lines } => {
                let log =
                    crate::features::workspace_processes::read_log(&path, &id, 16_000, lines)?;
                println!("{log}");
                Ok(())
            }
        },
        Command::Resources { path, json } => {
            let payload = crate::features::workspace_resources::resources_overview_payload(&path)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?
                );
            } else {
                let system = &payload["system"];
                println!(
                    "RAM: {:.1}% · {} / {} bytes",
                    system["memory_percent"].as_f64().unwrap_or_default(),
                    system["memory_used_bytes"].as_u64().unwrap_or_default(),
                    system["memory_total_bytes"].as_u64().unwrap_or_default()
                );
                println!(
                    "Swap: {:.1}% · {} / {} bytes",
                    system["swap_percent"].as_f64().unwrap_or_default(),
                    system["swap_used_bytes"].as_u64().unwrap_or_default(),
                    system["swap_total_bytes"].as_u64().unwrap_or_default()
                );
                for project in payload["projects"].as_array().into_iter().flatten() {
                    println!(
                        "{}\t{}\t{} proceso(s) activo(s)\t{} bytes RSS",
                        project["alias"].as_str().unwrap_or(""),
                        project["status"].as_str().unwrap_or("STOPPED"),
                        project["running_count"].as_u64().unwrap_or_default(),
                        project["rss_bytes"].as_u64().unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
        Command::External(arguments) => {
            if let Some(folder) = launch::folder_argument(&arguments) {
                return open_default(folder);
            }
            Err(launch::retired("", &arguments))
        }
    }
}

fn print_workspace_action(action: crate::features::workspace_lifecycle::WorkspaceAction) {
    println!("{} · {}", action.message, action.status);
    for process in action.processes {
        println!("{}", process.message);
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

    #[test]
    fn local_models_commands_are_handled_by_rust() {
        assert!(matches!(
            parse(&["local-models", "setup", "qwen2.5-coder:7b", "--dry-run"]).command,
            Some(Command::LocalModels {
                command: local_models::LocalModelsCommand::Setup { dry_run: true, .. }
            })
        ));
        assert!(matches!(
            parse(&["local-models", "remove", "qwen2.5-coder:7b", "--yes"]).command,
            Some(Command::LocalModels {
                command: local_models::LocalModelsCommand::Remove { yes: true, .. }
            })
        ));
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
    fn the_commands_of_the_earlier_cli_are_recognised_to_explain_where_they_went() {
        match parse(&["agent", "chat", "claude", "--live", "-x"]).command {
            Some(Command::Agent {
                command: Some(AgentCommand::External(args)),
            }) => assert_eq!(
                args.len(),
                4,
                "its flags are accepted, not rejected by the parser"
            ),
            other => panic!("expected a retired command, got {other:?}"),
        }
        assert!(matches!(
            parse(&["workspace", "attach"]).command,
            Some(Command::Workspace {
                command: WorkspaceCommand::External(_)
            })
        ));
        assert!(matches!(
            parse(&["projects", "prepare"]).command,
            Some(Command::Projects {
                command: ProjectsCommand::External(_)
            })
        ));
        assert!(matches!(
            parse(&["ci", "run"]).command,
            Some(Command::External(_))
        ));
    }

    #[test]
    fn each_retired_command_says_where_its_function_lives_now() {
        let say = |scope: &str, words: &[&str]| {
            launch::retired(scope, &words.iter().map(OsString::from).collect::<Vec<_>>())
        };
        assert!(say("", &["ci", "run"]).contains("CI/CD"));
        assert!(say("", &["sync"]).contains("projects sync"));
        assert!(say("", &["skill", "list"]).contains("`skills`"));
        assert!(say("workspace", &["terminal"]).contains("Studio"));
        assert!(say("projects", &["prepare"]).contains("init"));
        assert!(say("agent", &["list"]).contains("chat-state"));
        assert!(say("agent", &[]).contains("tui"));
        assert!(say("", &["bridge", "status"]).contains("se retiró"));
        let unknown = say("", &["quizas"]);
        assert!(
            unknown.contains("no es un comando") && !unknown.contains("se retiró"),
            "{unknown}"
        );
    }

    #[test]
    fn chat_state_is_a_rust_command_and_the_other_agent_commands_are_retired() {
        assert!(matches!(
            parse(&["agent", "chat-state", "codex", "--restore"]).command,
            Some(Command::Agent {
                command: Some(AgentCommand::ChatState { restore: true, .. })
            })
        ));
        assert!(matches!(
            parse(&["agent", "preflight", "codex"]).command,
            Some(Command::Agent {
                command: Some(AgentCommand::External(_))
            })
        ));
    }
}
