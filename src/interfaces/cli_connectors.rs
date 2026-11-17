//! `connectors`: give an agent hands in Excel, Power BI, a database… by adding an MCP server to the
//! project. Previews by default (with the exact files if asked); `--apply` writes.
use super::style::{Tone, Ui, pad};
use crate::features::mcp_connectors::connectors;
use crate::features::mcp_connectors_plan::{
    ConnectorsChange, ConnectorsPlan, Standing, apply_connectors, connector_states,
    default_clients, plan_connectors,
};
use crate::features::mcp_setup::McpClient;
use clap::Subcommand;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub(super) enum ConnectorsCommand {
    /// Lista los conectores (Excel, Power BI, bases de datos…) y dónde están configurados.
    List {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Añade conectores del catálogo al proyecto para los agentes elegidos.
    Add {
        #[arg(required = true)]
        names: Vec<String>,
        /// all, claude, codex u opencode; por defecto, los agentes instalados o ya configurados.
        #[arg(long)]
        client: Option<String>,
        /// Valor de un parámetro del conector, como `db_path=./datos.db`.
        #[arg(long = "param", value_name = "NOMBRE=VALOR")]
        params: Vec<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Muestra también el contenido exacto de cada archivo.
        #[arg(long)]
        show: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Quita conectores que ChxChx añadió (nunca los que configuraste distinto a mano).
    Remove {
        #[arg(required = true)]
        names: Vec<String>,
        #[arg(long)]
        client: Option<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        show: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Añade un servidor MCP tuyo que no está en el catálogo.
    Custom {
        name: String,
        /// Programa que inicia el servidor (`npx`, `uvx`, una ruta…).
        #[arg(long)]
        command: String,
        /// Argumento del programa (repite la opción por cada uno).
        #[arg(long = "arg", allow_hyphen_values = true)]
        args: Vec<String>,
        /// Variable de entorno que el servidor lee; su valor viene de tu entorno y no se guarda.
        #[arg(long = "env")]
        env: Vec<String>,
        #[arg(long)]
        client: Option<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        show: bool,
        #[arg(long)]
        apply: bool,
    },
}

fn pick(path: &Path, client: &Option<String>) -> Result<Vec<McpClient>, String> {
    match client {
        Some(value) => McpClient::parse(value),
        None => {
            let root = crate::features::project_overview::inspect_project(path)
                .map(|o| PathBuf::from(o.project.root))
                .unwrap_or_else(|_| path.to_path_buf());
            Ok(default_clients(&root))
        }
    }
}

fn values(params: &[String]) -> Result<BTreeMap<String, String>, String> {
    params
        .iter()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.trim().to_owned(), v.to_owned()))
                .ok_or_else(|| format!("`{p}` no tiene la forma NOMBRE=VALOR."))
        })
        .collect()
}

pub(super) fn run(ui: &Ui, command: ConnectorsCommand) -> Result<(), String> {
    match command {
        ConnectorsCommand::List { path } => list(ui, &path),
        ConnectorsCommand::Add {
            names,
            client,
            params,
            path,
            show,
            apply,
        } => {
            let change = ConnectorsChange::Add {
                names,
                values: values(&params)?,
            };
            let clients = pick(&path, &client)?;
            change_and_show(
                ui,
                &path,
                &change,
                &clients,
                "Añadir conectores",
                show,
                apply,
            )
        }
        ConnectorsCommand::Remove {
            names,
            client,
            path,
            show,
            apply,
        } => {
            let clients = pick(&path, &client)?;
            change_and_show(
                ui,
                &path,
                &ConnectorsChange::Remove { names },
                &clients,
                "Quitar conectores",
                show,
                apply,
            )
        }
        ConnectorsCommand::Custom {
            name,
            command,
            args,
            env,
            client,
            path,
            show,
            apply,
        } => {
            let words = std::iter::once(command).chain(args).collect();
            let change = ConnectorsChange::Custom {
                name,
                command: words,
                env,
            };
            let clients = pick(&path, &client)?;
            change_and_show(
                ui,
                &path,
                &change,
                &clients,
                "Añadir un servidor MCP propio",
                show,
                apply,
            )
        }
    }
}

fn list(ui: &Ui, path: &Path) -> Result<(), String> {
    let all = McpClient::parse("all")?;
    let states = connector_states(path, &all)?;
    println!(
        "{}",
        ui.heading(
            "Conectores MCP",
            "Conectores MCP",
            &path.display().to_string()
        )
    );
    let width = connectors()
        .iter()
        .map(|c| c.name.chars().count())
        .max()
        .unwrap_or(0);
    let mut category = "";
    for c in connectors() {
        if c.category != category {
            category = &c.category;
            println!("{}", ui.section(&format!("{category}:")));
        }
        let state = states.iter().find(|s| s.name == c.name);
        let marks: Vec<String> = state
            .map(|s| {
                s.standing
                    .iter()
                    .map(|(client, standing)| {
                        let label = client
                            .label()
                            .split(' ')
                            .next()
                            .unwrap_or("")
                            .to_lowercase();
                        match standing {
                            Standing::Configured => ui.paint(Tone::Green, &format!("{label} ✓")),
                            Standing::Different => ui.paint(Tone::Yellow, &format!("{label} ≠")),
                            Standing::Absent => ui.dim(&format!("{label} ·")),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        if ui.is_rich() {
            println!(
                "  {}  {}  {}",
                ui.bold(&pad(&c.name, width)),
                marks.join("  "),
                ui.dim(&c.title)
            );
            println!("  {}  {}", pad("", width), ui.dim(&c.description));
        } else {
            println!(
                "{} — {} [{}] {} — {}",
                c.name,
                c.title,
                c.origin,
                marks.join(" "),
                c.description
            );
        }
        if let Some(s) = state {
            for program in &s.missing_requirements {
                println!(
                    "  {}  {}",
                    pad("", width),
                    ui.warning(&format!("falta `{program}` en el PATH"))
                        .trim_start()
                );
            }
        }
    }
    println!("{}", ui.hint("✓ configurado · ≠ existe con otra configuración · · sin configurar. Añade uno: `connectors add excel --apply`."));
    Ok(())
}

fn show_plan(ui: &Ui, title: &str, plan: &ConnectorsPlan, show: bool) {
    println!(
        "{}",
        ui.heading(
            &format!("{title}: {}", plan.project_root),
            title,
            &plan.project_root
        )
    );
    println!("{}", ui.field("Agentes", &plan.clients.join(", ")));
    for action in &plan.actions {
        println!("{}", ui.bullet(action));
    }
    for warning in &plan.warnings {
        println!("{}", ui.warning(warning));
    }
    if let Some(reason) = &plan.blocked_reason {
        println!("{}", ui.blocked(reason));
    }
    if show {
        for (file, text) in plan.previews() {
            println!("{}", ui.section(&format!("{file}:")));
            println!("{text}");
        }
    }
}

fn change_and_show(
    ui: &Ui,
    path: &Path,
    change: &ConnectorsChange,
    clients: &[McpClient],
    title: &str,
    show: bool,
    apply: bool,
) -> Result<(), String> {
    let plan = plan_connectors(path, change, clients)?;
    show_plan(ui, title, &plan, show);
    if plan.blocked_reason.is_some() {
        return if apply {
            Err("Bloqueado; no se aplicaron cambios".to_owned())
        } else {
            Ok(())
        };
    }
    if !apply || !plan.changes_state() {
        if plan.changes_state() {
            println!("{}", ui.hint("Vista previa: añade --apply para escribirlo (con respaldo de lo que ya existía)."));
        }
        return Ok(());
    }
    let done = apply_connectors(&plan)?;
    println!(
        "\n{}",
        ui.section("Cambios aplicados:").trim_start_matches('\n')
    );
    for action in &done.completed {
        println!("{}", ui.done(action));
    }
    if let Some(backup) = &done.backup {
        println!("{}", ui.field("Respaldo", backup));
    }
    println!(
        "{}",
        ui.dim("Reinicia el agente (o abre una sesión nueva) para que cargue los servidores.")
    );
    Ok(())
}
