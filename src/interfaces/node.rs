//! `chxchx-tech node`: the local node (ADR 0017, H2). It owns this user's jobs and serves the API
//! on `127.0.0.1` only. Stopping it (Ctrl+C or `node stop`) cancels what is still running and
//! waits a moment for it, so nothing is left half done without a record.

#[path = "node_api.rs"]
mod api;
#[path = "node_api_devices.rs"]
mod api_devices;
#[path = "node_api_jobs.rs"]
mod api_jobs;
#[path = "node_api_projects.rs"]
mod api_projects;
#[cfg(test)]
#[path = "node_api_tests.rs"]
mod api_tests;
#[cfg(test)]
#[path = "node_link_tests.rs"]
mod link_tests;
#[path = "node_web.rs"]
mod web;

use crate::features::jobs::{self, JobRunner, JobStore, ProductExecutor};
use crate::features::node_state::{self, NodeInfo};
use chxchx_protocol::{AccessMode, PROTOCOL_VERSION};
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Notify;

pub const DEFAULT_PORT: u16 = 47_821;
pub fn run(port: u16) -> Result<(), String> {
    let dir = node_state::dir().ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    node_state::create_private_dir(&dir)?;
    let jobs_dir = jobs::default_dir().ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    let store = JobStore::open(&jobs_dir).map_err(|error| {
        format!("{error}. Si ya hay un nodo en marcha: `chxchx-tech node status`")
    })?;
    for warning in &store.warnings {
        eprintln!("Aviso: {warning}");
    }
    let token = node_state::ensure_token(&dir, || {
        let mut bytes = [0u8; 32];
        api::fill_random(&mut bytes);
        bytes
    })?;
    // The token holder can read this user's files: the same person as at the keyboard.
    let executor = ProductExecutor {
        access_cap: AccessMode::FullAccess,
    };
    // The node's clients can answer what an agent asks, so its jobs wait for them.
    let runner = JobRunner::attended(
        store,
        executor,
        crate::features::manager::parallel_jobs_here(),
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("No se pudo iniciar el nodo: {error}"))?;
    let result = runtime.block_on(async {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|error| format!("No se pudo escuchar en 127.0.0.1:{port}: {error}"))?;
        serve(listener, runner.clone(), token.into(), &dir, true).await
    });
    drain(&runner);
    node_state::append_log(&dir, "nodo detenido");
    node_state::remove_info(&dir);
    result
}

/// Serves until Ctrl+C (when `signals`) or `POST /api/v1/node/stop`.
pub(crate) async fn serve(
    listener: TcpListener,
    runner: JobRunner,
    token: Arc<str>,
    dir: &Path,
    signals: bool,
) -> Result<(), String> {
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let started_ms = jobs::store::now_ms();
    node_state::write_info(
        dir,
        &NodeInfo {
            pid: std::process::id(),
            port,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol: PROTOCOL_VERSION,
            started_ms,
        },
    )?;
    println!(
        "Nodo en http://127.0.0.1:{port} (solo este equipo). Ctrl+C o `chxchx-tech node stop` para pararlo."
    );
    let stop = Arc::new(Notify::new());
    node_state::append_log(dir, &format!("nodo iniciado en 127.0.0.1:{port}"));
    let state = api::AppState::new(
        runner,
        token,
        port,
        started_ms,
        stop.clone(),
        dir.to_owned(),
    )?;
    let shutdown = async move {
        if signals {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                () = stop.notified() => {}
            }
        } else {
            stop.notified().await;
        }
    };
    axum::serve(listener, api::router(state))
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|error| format!("El nodo se detuvo por un error: {error}"))
}

/// Cancels whatever is still queued or running and gives it a few seconds to stop.
fn drain(runner: &JobRunner) {
    for job in runner.list() {
        if !job.state.is_terminal() {
            let _ = runner.cancel(&job.id);
        }
    }
    if !runner.wait_idle(Duration::from_secs(10)) {
        eprintln!("Aviso: algún trabajo no terminó a tiempo; quedará como interrumpido.");
    }
}
