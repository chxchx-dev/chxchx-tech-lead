//! `chxchx-tech connection`: how this installation reaches the world, chosen by the person: the
//! node's port, the HTTPS port of Tailscale, where updates come from (and the public key they must
//! be signed with) and where each tool lives. Every value has the product's own as the starting
//! point; `reset` goes back to them and the replaced file stays as `.bak`.
use crate::features::connection::{self, Connection};
use crate::integrations::executable_search;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(super) enum ConnectionCommand {
    /// Muestra cada valor, si es el de fábrica o el tuyo, y dónde se encuentra cada herramienta.
    Show {
        #[arg(long)]
        json: bool,
    },
    /// Cambia lo que quieras; lo que no pongas se queda. `default` vuelve al valor de fábrica.
    Set {
        /// Puerto del nodo en este equipo (1024-65535).
        #[arg(long)]
        node_port: Option<String>,
        /// Puerto HTTPS de Tailscale que usa `node remote enable` si no le dices otro.
        #[arg(long)]
        https_port: Option<String>,
        /// Dueño (usuario u organización de GitHub) de las versiones.
        #[arg(long)]
        update_owner: Option<String>,
        /// Repositorio de GitHub de las versiones.
        #[arg(long)]
        update_repository: Option<String>,
        /// Clave PÚBLICA de firma de las versiones (64 dígitos hexadecimales). Solo si distribuyes
        /// tu propia versión: con ella se aceptan actualizaciones firmadas por ti, no por el autor.
        #[arg(long)]
        update_key: Option<String>,
        /// `nombre=ruta` de una herramienta (claude, codex, opencode, python, tailscale); se puede repetir.
        /// `nombre=default` la quita.
        #[arg(long = "tool", value_name = "NOMBRE=RUTA")]
        tools: Vec<String>,
        /// Solo muestra qué quedaría, sin guardar.
        #[arg(long)]
        dry_run: bool,
    },
    /// Vuelve a los valores de fábrica (el archivo anterior queda como `.bak`).
    Reset,
    /// La carpeta donde ChxChx guarda sus datos: sin opciones la muestra; con `--to` enseña qué
    /// copiaría (y con `--apply` lo hace); con `--back` vuelve a la original.
    ///
    /// Es una copia, no un movimiento: la carpeta de ahora no se toca y queda como respaldo.
    /// Hay que detener el nodo antes y reiniciar Studio y el nodo después.
    DataFolder {
        /// La carpeta nueva (vacía o que no exista), con ruta completa.
        #[arg(long, conflicts_with = "back")]
        to: Option<String>,
        /// Hace la copia y cambia a la carpeta nueva; sin esto solo se muestra.
        #[arg(long, requires = "to")]
        apply: bool,
        /// Vuelve a la carpeta original (lo guardado en la otra no se copia de vuelta).
        #[arg(long)]
        back: bool,
    },
}

pub(super) fn run(command: ConnectionCommand) -> Result<(), String> {
    match command {
        ConnectionCommand::Show { json } => show(json),
        ConnectionCommand::Set {
            node_port,
            https_port,
            update_owner,
            update_repository,
            update_key,
            tools,
            dry_run,
        } => {
            let mut mine = connection::load();
            if let Some(value) = &node_port {
                mine.node_port = number(value, "--node-port")?;
            }
            if let Some(value) = &https_port {
                mine.https_port = number(value, "--https-port")?;
            }
            if let Some(value) = update_owner {
                mine.update_owner = text(value);
            }
            if let Some(value) = update_repository {
                mine.update_repository = text(value);
            }
            if let Some(value) = update_key {
                mine.update_key = text(value);
            }
            for entry in tools {
                let (name, path) = entry.split_once('=').ok_or(
                    "--tool se escribe NOMBRE=RUTA (por ejemplo codex=/opt/codex/bin/codex)",
                )?;
                if path.eq_ignore_ascii_case("default") {
                    mine.tools.remove(name.trim());
                } else {
                    mine.tools
                        .insert(name.trim().to_owned(), path.trim().to_owned());
                }
            }
            connection::check(&mine)?;
            describe(&mine);
            if dry_run {
                println!("\nVista previa: no se guardó nada (quita --dry-run para aplicarlo).");
                return Ok(());
            }
            connection::save(&mine)?;
            println!(
                "\nListo. El puerto del nodo y el servicio se aplican la próxima vez que se inicien."
            );
            Ok(())
        }
        ConnectionCommand::DataFolder { to, apply, back } => data_folder(to, apply, back),
        ConnectionCommand::Reset => {
            connection::reset()?;
            println!("Valores de fábrica restaurados.");
            Ok(())
        }
    }
}

fn data_folder(to: Option<String>, apply: bool, back: bool) -> Result<(), String> {
    use crate::features::data_home as data;
    if back {
        for line in data::back()? {
            println!("{line}");
        }
        return Ok(());
    }
    let Some(to) = to else {
        for line in data::describe() {
            println!("{line}");
        }
        return Ok(());
    };
    let plan = data::preview(&to)?;
    println!(
        "Copiaría {} archivos ({}) de {} a {}.",
        plan.files,
        data::human(plan.bytes),
        plan.from.display(),
        plan.to.display()
    );
    if !apply {
        println!("Vista previa: no se cambió nada. Añade --apply para hacerlo.");
        return Ok(());
    }
    for line in data::apply(&plan)? {
        println!("{line}");
    }
    Ok(())
}

/// A number, or `default` to go back to the product's.
fn number(value: &str, flag: &str) -> Result<Option<u16>, String> {
    if value.eq_ignore_ascii_case("default") {
        return Ok(None);
    }
    value
        .trim()
        .parse::<u16>()
        .map(Some)
        .map_err(|_| format!("{flag} debe ser un número de puerto o `default`"))
}

fn text(value: String) -> Option<String> {
    let value = value.trim().to_owned();
    (!value.is_empty() && !value.eq_ignore_ascii_case("default")).then_some(value)
}

fn mark(chosen: bool) -> &'static str {
    if chosen { "tuyo" } else { "de fábrica" }
}

fn describe(mine: &Connection) {
    println!(
        "Puerto del nodo:    {} ({})",
        mine.node_port(),
        mark(mine.node_port.is_some())
    );
    println!(
        "Puerto HTTPS de Tailscale: {} ({})",
        mine.https_port(),
        mark(mine.https_port.is_some())
    );
    println!(
        "Versiones de:       github.com/{}/{} ({})",
        mine.owner(),
        mine.repository(),
        mark(mine.update_owner.is_some() || mine.update_repository.is_some())
    );
    println!(
        "Clave de firma:     {} ({})",
        mine.update_key.as_deref().map_or_else(
            || "la incluida en esta versión".to_owned(),
            |key| format!("{}…{}", &key[..8], &key[key.len() - 8..])
        ),
        mark(mine.update_key.is_some())
    );
    for name in connection::TOOLS {
        let chosen = mine.tools.get(*name);
        let found = chosen
            .map(std::path::PathBuf::from)
            .or_else(|| executable_search::find_executable(name));
        println!(
            "{:<19} {} ({})",
            format!("{name}:"),
            found.map_or_else(
                || "no encontrado".to_owned(),
                |path| path.display().to_string()
            ),
            if chosen.is_some() { "tuyo" } else { "del PATH" }
        );
    }
}

fn show(json: bool) -> Result<(), String> {
    let mine = connection::load();
    if json {
        let value = serde_json::json!({
            "schema": "chxchx.connection",
            "node_port": mine.node_port(),
            "https_port": mine.https_port(),
            "update_owner": mine.owner(),
            "update_repository": mine.repository(),
            "update_key": mine.update_key,
            "tools": mine.tools,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    describe(&mine);
    for line in crate::features::data_home::describe() {
        println!("{line}");
    }
    Ok(())
}
