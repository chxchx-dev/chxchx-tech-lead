//! `chxchx-tech node start|status|stop`.
#[path = "cli_node_qr.rs"]
mod qr;
#[path = "cli_node_remote.rs"]
mod remote;

use crate::features::{node_audit, node_state};
use crate::integrations::node_client;
use chxchx_protocol::{Device, PairingOffer, Role};
use clap::{Subcommand, ValueEnum};
use std::time::{Duration, Instant};

#[derive(Debug, Subcommand)]
pub(super) enum NodeCommand {
    /// Inicia el nodo en este equipo (solo 127.0.0.1) y se queda en primer plano.
    ///
    /// El nodo administra los trabajos: mientras corre, `jobs run` dice que lo usa otro proceso.
    Start {
        /// Puerto local; 0 elige uno libre.
        #[arg(long, default_value_t = crate::interfaces::node::DEFAULT_PORT)]
        port: u16,
    },
    /// Dice si el nodo está en marcha, en qué puerto y qué trabajos tiene.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Pide al nodo que se detenga; cancela lo que esté en curso.
    Stop,
    /// Crea un código de un solo uso (5 min) para vincular un celular u otro equipo.
    ///
    /// El dispositivo lo escribe y recibe su propio token, que se muestra una sola vez.
    Pair {
        /// Qué podrá hacer: reader mira; operator además lanza trabajos y aprueba; admin
        /// además gestiona dispositivos y lee la auditoría.
        #[arg(long, value_enum, default_value = "operator")]
        role: RoleArg,
    },
    /// Lista los dispositivos vinculados.
    Devices {
        #[arg(long)]
        json: bool,
    },
    /// Desvincula un dispositivo: su token deja de valer al instante.
    Revoke { id: String },
    /// Acceso desde tus otros dispositivos por Tailscale (vista previa y `--apply`).
    Remote {
        #[command(subcommand)]
        command: remote::RemoteCommand,
    },
    /// Muestra quién hizo qué (no hace falta que el nodo esté en marcha).
    Audit {
        #[arg(long, default_value_t = 30)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum RoleArg {
    Reader,
    Operator,
    Admin,
}

pub(super) fn run(command: NodeCommand) -> Result<(), String> {
    match command {
        NodeCommand::Start { port } => crate::interfaces::node::run(port),
        NodeCommand::Status { json } => status(json),
        NodeCommand::Stop => stop(),
        NodeCommand::Pair { role } => pair(match role {
            RoleArg::Reader => Role::Reader,
            RoleArg::Operator => Role::Operator,
            RoleArg::Admin => Role::Admin,
        }),
        NodeCommand::Devices { json } => devices(json),
        NodeCommand::Revoke { id } => revoke(&id),
        NodeCommand::Audit { limit, json } => audit(limit, json),
        NodeCommand::Remote { command } => remote::run(command),
    }
}

/// Asks the running node something with the local token and returns the answer's body.
fn ask(method: &str, path: &str, body: Option<&str>) -> Result<String, String> {
    let (_, info, token) = running()?;
    let response = node_client::request(info.port, method, path, Some(&token), body)?;
    if (200..300).contains(&response.status) {
        return Ok(response.body);
    }
    let message = serde_json::from_str::<chxchx_protocol::JobError>(&response.body)
        .map(|error| error.message)
        .unwrap_or(response.body);
    Err(message)
}

fn pair(role: Role) -> Result<(), String> {
    let body = serde_json::json!({ "role": role }).to_string();
    let offer: PairingOffer =
        serde_json::from_str(&ask("POST", "/api/v1/devices/pairing", Some(&body))?)
            .map_err(|error| error.to_string())?;
    let minutes = offer
        .expires_ms
        .saturating_sub(crate::features::jobs::store::now_ms())
        / 60_000;
    println!("Código de vinculación: {}", offer.code);
    println!(
        "Rol: {} · vale un solo uso y caduca en {minutes} min.",
        offer.role.as_str()
    );
    let remote = node_state::dir().and_then(|dir| node_state::read_remote(&dir));
    match remote {
        Some(remote) => {
            // The code goes after `#`: it never reaches a server, a proxy or a log.
            let address = format!("{}#code={}", remote.url(), offer.code);
            println!(
                "En el dispositivo, con Tailscale activo, abre {} e introduce el código,",
                remote.url()
            );
            println!("o escanea esto con la cámara:");
            if std::io::IsTerminal::is_terminal(&std::io::stdout())
                && let Some(code) = qr::render(&address)
            {
                println!("{code}");
            }
            println!("{address}");
        }
        None => println!(
            "El acceso desde fuera aún no está activado: `chxchx-tech node remote enable`."
        ),
    }
    Ok(())
}

fn devices(json: bool) -> Result<(), String> {
    let body = ask("GET", "/api/v1/devices", None)?;
    if json {
        println!("{body}");
        return Ok(());
    }
    let list: Vec<Device> = serde_json::from_str(&body).map_err(|error| error.to_string())?;
    if list.is_empty() {
        println!("No hay dispositivos vinculados. `chxchx-tech node pair` crea un código.");
    }
    for device in list {
        println!(
            "{}  {:<8}  {}  (vinculado {})",
            device.id,
            device.role.as_str(),
            device.name,
            when(device.created_ms)
        );
    }
    Ok(())
}

fn revoke(id: &str) -> Result<(), String> {
    let id = chxchx_protocol::DeviceId::parse(id).map_err(|error| error.to_string())?;
    let device: Device =
        serde_json::from_str(&ask("DELETE", &format!("/api/v1/devices/{id}"), None)?)
            .map_err(|error| error.to_string())?;
    println!(
        "Desvinculado «{}» ({}). Su token ya no vale.",
        device.name, device.id
    );
    Ok(())
}

fn audit(limit: usize, json: bool) -> Result<(), String> {
    let dir = node_state::dir().ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    let entries = node_audit::read(&dir, limit.clamp(1, 500));
    if json {
        return serde_json::to_string_pretty(&entries)
            .map(|text| println!("{text}"))
            .map_err(|error| error.to_string());
    }
    if entries.is_empty() {
        println!("La auditoría está vacía.");
    }
    for entry in entries {
        let detail = if entry.detail.as_object().is_some_and(|map| map.is_empty()) {
            String::new()
        } else {
            format!("  {}", entry.detail)
        };
        println!(
            "{}  {:<22}  {}{detail}",
            when(entry.at_ms),
            entry.action,
            entry.actor
        );
    }
    Ok(())
}

fn when(ms: u64) -> String {
    match time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000) {
        Ok(t) => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            t.year(),
            u8::from(t.month()),
            t.day(),
            t.hour(),
            t.minute(),
            t.second()
        ),
        Err(_) => ms.to_string(),
    }
}

/// The running node's port and token, or why there is none.
fn running() -> Result<(std::path::PathBuf, node_state::NodeInfo, String), String> {
    let dir = node_state::dir().ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    let info = node_state::read_info(&dir).ok_or("El nodo no está en marcha.")?;
    let token = node_state::read_token(&dir).ok_or("Falta el token del nodo (node/token).")?;
    Ok((dir, info, token))
}

fn status(json: bool) -> Result<(), String> {
    let (_, info, token) = running()?;
    let response = node_client::request(info.port, "GET", "/api/v1/node", Some(&token), None)
        .map_err(|error| {
            format!("{error}. Si el nodo se cerró de golpe, `chxchx-tech node start` lo vuelve a levantar.")
        })?;
    if response.status != 200 {
        return Err(format!(
            "El nodo respondió {}: {}",
            response.status, response.body
        ));
    }
    if json {
        println!("{}", response.body);
        return Ok(());
    }
    let node: serde_json::Value =
        serde_json::from_str(&response.body).map_err(|error| error.to_string())?;
    println!(
        "Nodo en marcha en http://127.0.0.1:{} · versión {} · PID {}",
        info.port,
        node["version"].as_str().unwrap_or("?"),
        node["pid"]
    );
    println!(
        "Trabajos: {} en curso, {} en cola",
        node["jobs"]["running"], node["jobs"]["queued"]
    );
    Ok(())
}

fn stop() -> Result<(), String> {
    let (dir, info, token) = running()?;
    let response =
        node_client::request(info.port, "POST", "/api/v1/node/stop", Some(&token), None)?;
    if response.status != 202 {
        return Err(format!(
            "El nodo respondió {}: {}",
            response.status, response.body
        ));
    }
    // It cancels running jobs and waits for them before it is gone.
    let deadline = Instant::now() + Duration::from_secs(20);
    while node_state::read_info(&dir).is_some() {
        if Instant::now() > deadline {
            return Err("El nodo recibió la orden pero aún no se ha detenido.".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("Nodo detenido.");
    Ok(())
}
