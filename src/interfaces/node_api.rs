//! The node's HTTP API v1 (ADR 0017): state, who is calling, and the guards every request passes.
//!
//! Three guards, outermost first:
//! 1. **Limits:** a body cap and a ceiling on requests in flight.
//! 2. **Where it was addressed:** `Host` must be `127.0.0.1`/`localhost` on the node's port, or the
//!    Tailscale name the person enabled (`remote.json`). A web page can send requests to
//!    `127.0.0.1`, so any other `Host` (DNS rebinding) or `Origin` is refused before anything else.
//! 3. **Who is calling:** every route but `health` and `pair` needs a token. The local token is
//!    the person at this PC (admin) and only works when the request is addressed to loopback; a
//!    device token carries the role it was paired with.

use super::{api_devices as devices, api_jobs as jobs};
use crate::features::node_devices::Devices;
use crate::features::node_state::{self, RemoteConfig};
use crate::features::{jobs::JobRunner, node_audit};
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chxchx_protocol::{AccessMode, Device, ErrorCode, JobError, PROTOCOL_VERSION, Role};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::{Notify, Semaphore};

/// The WebSocket subprotocol of this API. A browser cannot set `Authorization` on a WebSocket, so
/// it offers the token as a second protocol, `bearer.<token>`; the node answers with this one.
pub(crate) const WS_PROTOCOL: &str = "chxchx.v1";

/// Large enough for the biggest prompt a job accepts, and no more. Every other route takes 8 KiB.
const MAX_JOB_BODY_BYTES: usize = chxchx_protocol::MAX_PROMPT_BYTES + 64 * 1024;
const MAX_BODY_BYTES: usize = 8 * 1024;
const MAX_IN_FLIGHT: usize = 128;
/// Event streams open at once.
const MAX_SOCKETS: usize = 16;

#[derive(Clone)]
pub(crate) struct AppState {
    pub runner: JobRunner,
    pub token: Arc<str>,
    pub port: u16,
    pub started_ms: u64,
    pub stop: Arc<Notify>,
    /// Where `node.log`, `audit.jsonl`, `devices.json` and `remote.json` live.
    pub dir: PathBuf,
    pub devices: Arc<Mutex<Devices>>,
    pub in_flight: Arc<Semaphore>,
    pub sockets: Arc<Semaphore>,
}

impl AppState {
    pub fn new(
        runner: JobRunner,
        token: Arc<str>,
        port: u16,
        started_ms: u64,
        stop: Arc<Notify>,
        dir: PathBuf,
    ) -> Result<Self, String> {
        Ok(Self {
            runner,
            token,
            port,
            started_ms,
            stop,
            devices: Arc::new(Mutex::new(Devices::open(&dir)?)),
            dir,
            in_flight: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
            sockets: Arc::new(Semaphore::new(MAX_SOCKETS)),
        })
    }

    pub fn log(&self, line: &str) {
        node_state::append_log(&self.dir, line);
    }

    pub fn audit(&self, who: &Principal, action: &str, detail: Value) {
        node_audit::record(&self.dir, &who.actor(), action, detail);
    }

    pub fn devices(&self) -> MutexGuard<'_, Devices> {
        self.devices
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

/// Who is calling.
#[derive(Clone, Debug)]
pub(crate) enum Principal {
    /// The person at this PC, with the local token.
    Local,
    Device(Device),
}

impl Principal {
    pub fn role(&self) -> Role {
        match self {
            Principal::Local => Role::Admin,
            Principal::Device(device) => device.role,
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Principal::Local)
    }

    /// What the audit log calls this caller.
    pub fn actor(&self) -> String {
        match self {
            Principal::Local => "local".to_owned(),
            Principal::Device(device) => device.id.to_string(),
        }
    }

    /// The highest access level this caller may give an agent (ADR 0014, ADR 0017).
    pub fn access_cap(&self) -> AccessMode {
        match self {
            Principal::Local => AccessMode::FullAccess,
            Principal::Device(_) => AccessMode::REMOTE_DEFAULT_CAP,
        }
    }

    /// Whether the agent may reach the network. A device below admin cannot enable it.
    pub fn may_use_network(&self) -> bool {
        self.role() == Role::Admin
    }
}

/// How the request got here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Via {
    /// Addressed to `127.0.0.1` or `localhost`.
    Loopback,
    /// Addressed to the Tailscale name, through `tailscale serve`.
    Remote,
}

pub(crate) fn require(who: &Principal, needed: Role) -> Result<(), ApiError> {
    if who.role() >= needed {
        Ok(())
    } else {
        Err(error(
            ErrorCode::Forbidden,
            format!(
                "Este dispositivo es «{}» y esto pide «{}»",
                who.role().as_str(),
                needed.as_str()
            ),
        ))
    }
}

pub(crate) fn require_local(who: &Principal) -> Result<(), ApiError> {
    if who.is_local() {
        Ok(())
    } else {
        Err(error(
            ErrorCode::Forbidden,
            "Esto solo se puede hacer desde la PC del nodo",
        ))
    }
}

pub(crate) fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/api/v1/node", get(jobs::node))
        .route("/api/v1/node/stop", post(jobs::stop))
        .route("/api/v1/projects", get(devices::projects))
        .route(
            "/api/v1/jobs",
            get(jobs::list)
                .post(jobs::create)
                .layer(DefaultBodyLimit::max(MAX_JOB_BODY_BYTES)),
        )
        .route("/api/v1/jobs/{id}", get(jobs::show))
        .route("/api/v1/jobs/{id}/events", get(jobs::events))
        .route("/api/v1/jobs/{id}/cancel", post(jobs::cancel))
        .route("/api/v1/jobs/{id}/answers", post(jobs::answer))
        .route("/api/v1/events", get(jobs::stream))
        .route("/api/v1/devices", get(devices::list))
        .route("/api/v1/devices/pairing", post(devices::offer))
        .route("/api/v1/devices/{id}", delete(devices::revoke))
        .route("/api/v1/audit", get(devices::audit))
        .layer(from_fn_with_state(state.clone(), require_token));
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/pair", post(devices::pair))
        .merge(protected)
        .layer(from_fn_with_state(state.clone(), host_guard))
        .layer(from_fn_with_state(state.clone(), limit_in_flight))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

pub(crate) struct ApiError(JobError);

impl From<JobError> for ApiError {
    fn from(error: JobError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code {
            ErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden | ErrorCode::UntrustedProject => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict | ErrorCode::Interrupted => StatusCode::CONFLICT,
            ErrorCode::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::ProviderUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ProviderFailed => StatusCode::BAD_GATEWAY,
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self.0)).into_response()
    }
}

pub(crate) fn error(code: ErrorCode, message: impl Into<String>) -> ApiError {
    ApiError(JobError::new(code, message))
}

/// Fills `buffer` with random bytes from the operating system.
pub(crate) fn fill_random(buffer: &mut [u8]) {
    getrandom::fill(buffer).expect("the operating system provides random bytes");
}

async fn limit_in_flight(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let Ok(_permit) = state.in_flight.clone().try_acquire_owned() else {
        return Err(error(
            ErrorCode::TooManyRequests,
            "El nodo está atendiendo demasiadas peticiones a la vez",
        ));
    };
    Ok(next.run(request).await)
}

async fn host_guard(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let loopback = |value: &str, scheme: &str| {
        [
            format!("{scheme}127.0.0.1:{}", state.port),
            format!("{scheme}localhost:{}", state.port),
        ]
        .iter()
        .any(|expected| expected == value)
    };
    let headers = request.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_ascii_lowercase);
    let origin = headers
        .get(header::ORIGIN)
        .map(|value| value.to_str().unwrap_or_default().to_ascii_lowercase());
    let path = request.uri().path().to_owned();

    let (via, allowed_origin) = match host.as_deref() {
        Some(host) if loopback(host, "") => (Via::Loopback, None),
        // Read only for a request that is not addressed to loopback: the common path stays free.
        Some(host) => match node_state::read_remote(&state.dir) {
            Some(remote) if remote.authority() == host => (Via::Remote, Some(remote)),
            _ => return Err(refused_host(&state, Some(host.to_owned()), &path)),
        },
        None => return Err(refused_host(&state, None, &path)),
    };
    if let Some(origin) = origin {
        let fine = match &allowed_origin {
            Some(remote) => origin == remote.origin(),
            None => loopback(&origin, "http://"),
        };
        if !fine {
            state.log(&format!("rechazada: Origin {origin:?} en {path}"));
            return Err(error(
                ErrorCode::Forbidden,
                "Origen no permitido para el nodo",
            ));
        }
    }
    request.extensions_mut().insert(via);
    Ok(next.run(request).await)
}

fn refused_host(state: &AppState, host: Option<String>, path: &str) -> ApiError {
    state.log(&format!(
        "rechazada: Host {:?} en {path}",
        host.as_deref().unwrap_or("(ninguno)")
    ));
    error(
        ErrorCode::Forbidden,
        "El nodo solo atiende peticiones dirigidas a este equipo o a su nombre de Tailscale",
    )
}

async fn require_token(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let via = request
        .extensions()
        .get::<Via>()
        .copied()
        .unwrap_or(Via::Loopback);
    let headers = request.headers();
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let offered = headers
        .get(header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .split(',')
                .find_map(|protocol| protocol.trim().strip_prefix("bearer."))
        });
    let given = bearer.or(offered).unwrap_or_default().to_owned();
    let path = request.uri().path().to_owned();

    let principal = if !given.is_empty() && same(given.as_bytes(), state.token.as_bytes()) {
        if via == Via::Remote {
            state.log(&format!(
                "rechazada: token local usado desde fuera en {path}"
            ));
            return Err(error(
                ErrorCode::Forbidden,
                "El token local solo vale en el propio equipo: vincula este dispositivo",
            ));
        }
        Principal::Local
    } else {
        let device = if given.is_empty() {
            None
        } else {
            state
                .devices()
                .authenticate(&given, crate::features::jobs::store::now_ms())
        };
        match device {
            Some(device) => Principal::Device(device),
            None => {
                // The path only: never what was sent as a token.
                state.log(&format!("rechazada: token ausente o no válido en {path}"));
                return Err(error(
                    ErrorCode::Unauthorized,
                    "Falta el token del nodo o no es válido",
                ));
            }
        }
    };
    request.extensions_mut().insert(principal);
    Ok(next.run(request).await)
}

/// Compares without stopping at the first difference, so timing does not reveal the token.
fn same(given: &[u8], expected: &[u8]) -> bool {
    given.len() == expected.len()
        && given
            .iter()
            .zip(expected)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "protocol": PROTOCOL_VERSION }))
}

/// What `remote.json` says, for handlers that report it.
pub(crate) fn remote(state: &AppState) -> Option<RemoteConfig> {
    node_state::read_remote(&state.dir)
}
