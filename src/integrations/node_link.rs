//! A program's side of the local node (ADR 0017, H5): find it, and ask it things with types.
//!
//! Studio uses this to hand its chat turns to the node, to see what the node is running and to
//! manage linked devices. Everything is bounded by a short time limit, so a node that stops
//! answering costs a failed call, never a frozen window. The token is read from the node's own
//! private file and is never printed.

use crate::features::node_state;
use crate::integrations::node_client;
use chxchx_protocol::{
    AgentTurnSpec, Device, DeviceId, Job, JobAnswer, JobEvent, JobId, JobSpec, NodeStatus,
    PairingOffer, ProjectId, Provider, Role,
};
use serde::de::DeserializeOwned;
use std::path::Path;
use std::time::Duration;

/// Time a call to the node may take. On loopback a healthy node answers in milliseconds.
const QUICK: Duration = Duration::from_millis(1_500);
const HEALTH: Duration = Duration::from_millis(500);

/// How to reach a running node.
#[derive(Clone)]
pub struct Endpoint {
    pub port: u16,
    token: String,
}

impl std::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Endpoint {{ port: {}, token: <hidden> }}", self.port)
    }
}

impl Endpoint {
    /// For tests that stand a node of their own up.
    pub fn new(port: u16, token: impl Into<String>) -> Self {
        Self {
            port,
            token: token.into(),
        }
    }
}

/// Tests point the code at a node of their own, without touching the environment (which every
/// test shares). Whoever sets it holds `TEST_NODE_LOCK` for the length of the test.
#[cfg(test)]
pub(crate) static TEST_NODE: std::sync::Mutex<Option<(Endpoint, ProjectId)>> =
    std::sync::Mutex::new(None);
#[cfg(all(test, unix))]
pub(crate) static TEST_NODE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
fn test_node() -> Option<(Endpoint, ProjectId)> {
    TEST_NODE
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clone()
}

/// The node of this user, if one is running and answers: its file says where it is, its process
/// is alive and `health` replies. `None` otherwise (Studio then runs everything itself).
pub fn endpoint() -> Option<Endpoint> {
    #[cfg(test)]
    if let Some((endpoint, _)) = test_node() {
        return Some(endpoint);
    }
    let dir = node_state::dir()?;
    let info = node_state::read_info(&dir)?;
    if !crate::features::workspace_processes::pid_alive(info.pid) {
        return None;
    }
    let token = node_state::read_token(&dir)?;
    let reply =
        node_client::send(info.port, "GET", "/api/v1/health", None, None, &[], HEALTH).ok()?;
    (reply.status == 200).then_some(Endpoint {
        port: info.port,
        token,
    })
}

fn call<T: DeserializeOwned>(
    endpoint: &Endpoint,
    method: &str,
    path: &str,
    body: Option<String>,
    headers: &[(&str, &str)],
) -> Result<T, String> {
    let reply = node_client::send(
        endpoint.port,
        method,
        path,
        Some(&endpoint.token),
        body.as_deref(),
        headers,
        QUICK,
    )?;
    if (200..300).contains(&reply.status) {
        return serde_json::from_str(&reply.body)
            .map_err(|error| format!("Respuesta del nodo ilegible: {error}"));
    }
    // The node explains its refusals as `{code, message}`.
    Err(
        serde_json::from_str::<chxchx_protocol::JobError>(&reply.body)
            .map(|error| error.message)
            .unwrap_or_else(|_| format!("El nodo respondió {}", reply.status)),
    )
}

pub fn status(endpoint: &Endpoint) -> Result<NodeStatus, String> {
    call(endpoint, "GET", "/api/v1/node", None, &[])
}

pub fn list_jobs(endpoint: &Endpoint) -> Result<Vec<Job>, String> {
    call(endpoint, "GET", "/api/v1/jobs", None, &[])
}

pub fn job(endpoint: &Endpoint, id: &JobId) -> Result<Job, String> {
    call(endpoint, "GET", &format!("/api/v1/jobs/{id}"), None, &[])
}

pub fn events(endpoint: &Endpoint, id: &JobId, since: u64) -> Result<Vec<JobEvent>, String> {
    call(
        endpoint,
        "GET",
        &format!("/api/v1/jobs/{id}/events?since={since}"),
        None,
        &[],
    )
}

/// Creates a job. A repeat of the same `key` is the same job.
pub fn create_job(endpoint: &Endpoint, spec: &JobSpec, key: &str) -> Result<Job, String> {
    let body = serde_json::to_string(spec).map_err(|error| error.to_string())?;
    call(
        endpoint,
        "POST",
        "/api/v1/jobs",
        Some(body),
        &[("Idempotency-Key", key)],
    )
}

pub fn answer(endpoint: &Endpoint, id: &JobId, answer: &JobAnswer) -> Result<(), String> {
    let body = serde_json::to_string(answer).map_err(|error| error.to_string())?;
    call::<serde_json::Value>(
        endpoint,
        "POST",
        &format!("/api/v1/jobs/{id}/answers"),
        Some(body),
        &[],
    )
    .map(|_| ())
}

pub fn cancel(endpoint: &Endpoint, id: &JobId) -> Result<Job, String> {
    call(
        endpoint,
        "POST",
        &format!("/api/v1/jobs/{id}/cancel"),
        Some("{}".to_owned()),
        &[],
    )
}

pub fn devices(endpoint: &Endpoint) -> Result<Vec<Device>, String> {
    call(endpoint, "GET", "/api/v1/devices", None, &[])
}

pub fn revoke(endpoint: &Endpoint, id: &DeviceId) -> Result<Device, String> {
    call(
        endpoint,
        "DELETE",
        &format!("/api/v1/devices/{id}"),
        None,
        &[],
    )
}

/// Asks the node to stop. It cancels what is running first, so this is for the person to decide.
pub fn stop(endpoint: &Endpoint) -> Result<(), String> {
    call::<serde_json::Value>(endpoint, "POST", "/api/v1/node/stop", None, &[]).map(|_| ())
}

/// A one-use code for linking a new device.
pub fn pairing(endpoint: &Endpoint, role: Role) -> Result<PairingOffer, String> {
    let body = serde_json::json!({ "role": role }).to_string();
    call(endpoint, "POST", "/api/v1/devices/pairing", Some(body), &[])
}

/// The alias the project registry gives the project at `root`, which is how the node names it.
/// `None` when the project is not registered: the node does not know it.
pub fn alias_for(root: &Path) -> Option<ProjectId> {
    #[cfg(test)]
    if let Some((_, project)) = test_node() {
        return Some(project);
    }
    let wanted = std::fs::canonicalize(root).ok()?;
    crate::features::project_catalog::inspect_registered_projects()
        .ok()?
        .projects
        .into_iter()
        .filter(|project| project.available)
        .find(|project| {
            project
                .path
                .as_deref()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .is_some_and(|path| path == wanted)
        })
        .and_then(|project| ProjectId::parse(project.alias).ok())
}

/// The agent turns of `alias` with `provider` that the node is still running, newest last.
pub fn running_turns(
    endpoint: &Endpoint,
    alias: &ProjectId,
    provider: Provider,
) -> Result<Vec<Job>, String> {
    let mut jobs: Vec<Job> = list_jobs(endpoint)?
        .into_iter()
        .filter(|job| !job.state.is_terminal())
        .filter(|job| {
            matches!(&job.spec, JobSpec::AgentTurn(AgentTurnSpec { project, provider: p, .. })
                if project == alias && *p == provider)
        })
        .collect();
    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(jobs)
}

/// A key that is different for every request this process makes.
pub fn fresh_key() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "studio-{}-{}-{}",
        std::process::id(),
        crate::features::jobs::store::now_ms(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_never_shows_in_debug_output() {
        let endpoint = Endpoint::new(47821, "secret-token-value");
        let shown = format!("{endpoint:?}");
        assert!(
            shown.contains("47821") && !shown.contains("secret"),
            "{shown}"
        );
    }

    #[test]
    fn keys_are_different_for_every_request_and_valid_for_the_node() {
        let (a, b) = (fresh_key(), fresh_key());
        assert_ne!(a, b);
        assert!(chxchx_protocol::IdempotencyKey::parse(a).is_ok());
    }

    #[test]
    fn without_a_node_file_there_is_no_endpoint() {
        // The test home has no node: Studio must go on by itself, not fail.
        let dir = std::env::temp_dir().join(format!("chxchx-link-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(node_state::read_info(&dir), None);
    }
}
