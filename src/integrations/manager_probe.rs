//! Reads what this machine has, for the manager (ADR 0017, H6). Read-only: it only looks for
//! programs and asks the node and the agent service who they are.

use crate::features::manager::Facts;
use crate::integrations::executable_search::find_executable;
use std::path::PathBuf;

fn shown(path: Option<PathBuf>) -> Option<String> {
    path.map(|path| path.display().to_string())
}

pub fn read() -> Facts {
    let (node, node_service_installed) = node();
    Facts {
        claude: shown(find_executable("claude")),
        codex: shown(find_executable("codex")),
        opencode: shown(find_executable("opencode")),
        python: shown(super::agent_adapter_bundle::python().ok()),
        agent_service: shown(super::agent_adapter::locate(None)),
        node,
        node_service_installed,
        tailscale: tailscale(),
        memory_percent: memory(),
        disk_free_bytes: disk_free(),
    }
}

fn memory() -> Option<f64> {
    let system = crate::features::workspace_resources::read_system_resources();
    (system.total_bytes > 0).then(|| system.used_bytes as f64 * 100.0 / system.total_bytes as f64)
}

#[cfg(feature = "node")]
fn node() -> (Option<(u16, String)>, bool) {
    use super::{node_link, user_service};
    let running = node_link::endpoint().and_then(|endpoint| {
        let status = node_link::status(&endpoint).ok()?;
        Some((endpoint.port, status.version))
    });
    let installed = user_service::current_setup(crate::interfaces::node::DEFAULT_PORT)
        .map(|setup| user_service::is_installed(&user_service::plan(&setup)))
        .unwrap_or(false);
    (running, installed)
}

#[cfg(not(feature = "node"))]
fn node() -> (Option<(u16, String)>, bool) {
    (None, false)
}

#[cfg(feature = "node")]
fn tailscale() -> Option<String> {
    shown(super::tailscale::locate())
}

#[cfg(not(feature = "node"))]
fn tailscale() -> Option<String> {
    None
}

/// Free bytes where ChxChx keeps its files, from `df` (Linux and macOS); `None` elsewhere.
fn disk_free() -> Option<u64> {
    let home = crate::features::project_profiles::application_home()?;
    let target = std::iter::successors(Some(home.as_path()), |path| path.parent())
        .find(|path| path.exists())?;
    let output = std::process::Command::new("df")
        .args(["-Pk"])
        .arg(target)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let kib: u64 = text
        .lines()
        .nth(1)?
        .split_whitespace()
        .nth(3)?
        .parse()
        .ok()?;
    Some(kib * 1024)
}
