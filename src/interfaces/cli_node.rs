//! `chxchx-tech node start|status|stop`.
use crate::features::node_state;
use crate::integrations::node_client;
use clap::Subcommand;
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
}

pub(super) fn run(command: NodeCommand) -> Result<(), String> {
    match command {
        NodeCommand::Start { port } => crate::interfaces::node::run(port),
        NodeCommand::Status { json } => status(json),
        NodeCommand::Stop => stop(),
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
