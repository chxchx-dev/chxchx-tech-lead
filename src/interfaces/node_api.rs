//! The node's HTTP API v1 (ADR 0017). Every route but `health` needs the local token, and every
//! request must be addressed to loopback: a web page in a browser can send requests to
//! `127.0.0.1`, so a foreign `Host` (DNS rebinding) or `Origin` is refused before anything else.

use crate::features::jobs::JobRunner;
use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chxchx_protocol::{
    ErrorCode, IdempotencyKey, JobAnswer, JobError, JobEvent, JobId, JobSpec, JobState,
    PROTOCOL_VERSION,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// The WebSocket subprotocol of this API. A browser cannot set `Authorization` on a WebSocket, so
/// it offers the token as a second protocol, `bearer.<token>`; the node answers with this one.
pub(crate) const WS_PROTOCOL: &str = "chxchx.v1";

/// Large enough for the biggest prompt a job accepts, and no more.
const MAX_BODY_BYTES: usize = chxchx_protocol::MAX_PROMPT_BYTES + 64 * 1024;

#[derive(Clone)]
pub(crate) struct AppState {
    pub runner: JobRunner,
    pub token: Arc<str>,
    pub port: u16,
    pub started_ms: u64,
    pub stop: Arc<Notify>,
    /// Where `node.log` goes.
    pub dir: PathBuf,
}

impl AppState {
    fn log(&self, line: &str) {
        crate::features::node_state::append_log(&self.dir, line);
    }
}

pub(crate) fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/api/v1/node", get(node))
        .route("/api/v1/node/stop", post(stop))
        .route("/api/v1/jobs", get(list).post(create))
        .route("/api/v1/jobs/{id}", get(show))
        .route("/api/v1/jobs/{id}/events", get(events))
        .route("/api/v1/jobs/{id}/cancel", post(cancel))
        .route("/api/v1/jobs/{id}/answers", post(answer))
        .route("/api/v1/events", get(stream))
        .layer(from_fn_with_state(state.clone(), require_token));
    Router::new()
        .route("/api/v1/health", get(health))
        .merge(protected)
        .layer(from_fn_with_state(state.clone(), loopback_only))
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
            ErrorCode::ProviderUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ProviderFailed => StatusCode::BAD_GATEWAY,
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self.0)).into_response()
    }
}

fn error(code: ErrorCode, message: impl Into<String>) -> ApiError {
    ApiError(JobError::new(code, message))
}

async fn loopback_only(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let allowed = |value: &str, scheme: &str| {
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
        .and_then(|value| value.to_str().ok());
    if !host.is_some_and(|host| allowed(host, "")) {
        state.log(&format!(
            "rechazada: Host {:?} en {}",
            host.unwrap_or("(ninguno)"),
            request.uri().path()
        ));
        return Err(error(
            ErrorCode::Forbidden,
            "El nodo solo atiende peticiones dirigidas a 127.0.0.1",
        ));
    }
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin = origin.to_str().unwrap_or_default();
        if !allowed(origin, "http://") {
            state.log(&format!(
                "rechazada: Origin {origin:?} en {}",
                request.uri().path()
            ));
            return Err(error(
                ErrorCode::Forbidden,
                "Origen no permitido para el nodo",
            ));
        }
    }
    Ok(next.run(request).await)
}

async fn require_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
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
    let given = bearer.or(offered).unwrap_or_default();
    if !same(given.as_bytes(), state.token.as_bytes()) {
        // The path only: never what was sent as a token.
        state.log(&format!(
            "rechazada: token ausente o no válido en {}",
            request.uri().path()
        ));
        return Err(error(
            ErrorCode::Unauthorized,
            "Falta el token del nodo o no es válido",
        ));
    }
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

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "protocol": PROTOCOL_VERSION }))
}

async fn node(State(state): State<AppState>) -> Json<serde_json::Value> {
    let jobs = state.runner.list();
    let count = |wanted: JobState| jobs.iter().filter(|job| job.state == wanted).count();
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": PROTOCOL_VERSION,
        "pid": std::process::id(),
        "started_ms": state.started_ms,
        "jobs": { "queued": count(JobState::Queued), "running": count(JobState::Running) },
    }))
}

async fn stop(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    state.log("parada pedida por la API");
    state.stop.notify_one();
    (StatusCode::ACCEPTED, Json(json!({ "status": "stopping" })))
}

async fn list(State(state): State<AppState>) -> Json<Vec<chxchx_protocol::Job>> {
    Json(state.runner.list())
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<chxchx_protocol::Job>), ApiError> {
    let spec: JobSpec = serde_json::from_slice(&body).map_err(|problem| {
        error(
            ErrorCode::InvalidRequest,
            format!("Petición no válida: {problem}"),
        )
    })?;
    let key = headers
        .get("idempotency-key")
        .map(|value| {
            value
                .to_str()
                .map_err(|_| error(ErrorCode::InvalidRequest, "Idempotency-Key no válida"))
                .and_then(|value| {
                    IdempotencyKey::parse(value)
                        .map_err(|problem| error(ErrorCode::InvalidRequest, problem.to_string()))
                })
        })
        .transpose()?;
    let runner = state.runner.clone();
    // Writing the job is a synced file write: kept off the async workers.
    let job = tokio::task::spawn_blocking(move || runner.submit(spec, key))
        .await
        .map_err(|_| error(ErrorCode::Internal, "Falló el registro del trabajo"))??;
    state.log(&format!("trabajo {} recibido ({:?})", job.id, job.state));
    Ok((StatusCode::ACCEPTED, Json(job)))
}

fn job_id(raw: &str) -> Result<JobId, ApiError> {
    JobId::parse(raw).map_err(|problem| error(ErrorCode::InvalidRequest, problem.to_string()))
}

fn not_found(id: &JobId) -> ApiError {
    error(ErrorCode::NotFound, format!("No existe el trabajo {id}"))
}

async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<chxchx_protocol::Job>, ApiError> {
    let id = job_id(&id)?;
    state
        .runner
        .get(&id)
        .map(Json)
        .ok_or_else(|| not_found(&id))
}

#[derive(Deserialize)]
struct Since {
    #[serde(default)]
    since: u64,
}

async fn events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<Since>,
) -> Result<Json<Vec<chxchx_protocol::JobEvent>>, ApiError> {
    let id = job_id(&id)?;
    if state.runner.get(&id).is_none() {
        return Err(not_found(&id));
    }
    Ok(Json(state.runner.events_since(&id, query.since)))
}

async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<chxchx_protocol::Job>, ApiError> {
    let id = job_id(&id)?;
    let runner = state.runner.clone();
    let job = tokio::task::spawn_blocking(move || runner.cancel(&id))
        .await
        .map_err(|_| error(ErrorCode::Internal, "Falló la cancelación"))??;
    state.log(&format!("trabajo {} cancelado ({:?})", job.id, job.state));
    Ok(Json(job))
}

async fn answer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let id = job_id(&id)?;
    let answer: JobAnswer = serde_json::from_slice(&body).map_err(|problem| {
        error(
            ErrorCode::InvalidRequest,
            format!("Respuesta no válida: {problem}"),
        )
    })?;
    let request_id = answer.request_id.clone();
    let behavior = answer.behavior;
    state.runner.answer(&id, answer)?;
    // What was decided, never the text of an answer to a question.
    state.log(&format!(
        "trabajo {id}: respuesta a {request_id} ({})",
        behavior.map_or("pregunta".to_owned(), |b| format!("{b:?}").to_lowercase())
    ));
    Ok((StatusCode::ACCEPTED, Json(json!({ "status": "delivered" }))))
}

#[derive(Deserialize)]
struct Stream {
    job: Option<String>,
    #[serde(default)]
    since: u64,
}

/// Events as they happen, one JSON `JobEvent` per text message. With `job`, it first sends that
/// job's events after `since`, so a client that reconnects only gets what it missed.
async fn stream(
    State(state): State<AppState>,
    Query(query): Query<Stream>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let job = query.job.as_deref().map(job_id).transpose()?;
    if let Some(id) = &job
        && state.runner.get(id).is_none()
    {
        return Err(not_found(id));
    }
    let runner = state.runner.clone();
    Ok(upgrade
        .protocols([WS_PROTOCOL])
        .on_upgrade(move |socket| forward(socket, runner, job, query.since)))
}

async fn forward(mut socket: WebSocket, runner: JobRunner, job: Option<JobId>, since: u64) {
    // Subscribed before reading the backlog: nothing can fall between the two.
    let live = runner.subscribe();
    let (sender, mut events) = tokio::sync::mpsc::channel::<JobEvent>(256);
    std::thread::spawn(move || {
        loop {
            match live.recv_timeout(Duration::from_millis(250)) {
                Ok(event) => {
                    if sender.blocking_send(event).is_err() {
                        return;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) if !sender.is_closed() => {}
                Err(_) => return,
            }
        }
    });
    let mut sent: BTreeMap<JobId, u64> = BTreeMap::new();
    if let Some(id) = &job {
        for event in runner.events_since(id, since) {
            sent.insert(event.job.clone(), event.seq);
            if send(&mut socket, &event).await.is_err() {
                return;
            }
        }
    }
    loop {
        tokio::select! {
            event = events.recv() => {
                let Some(event) = event else { return };
                if job.as_ref().is_some_and(|id| *id != event.job)
                    || sent.get(&event.job).is_some_and(|last| event.seq <= *last)
                {
                    continue;
                }
                sent.insert(event.job.clone(), event.seq);
                if send(&mut socket, &event).await.is_err() {
                    return;
                }
            }
            incoming = socket.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => {} // pings are answered by the library; nothing else is expected
            }
        }
    }
}

async fn send(socket: &mut WebSocket, event: &JobEvent) -> Result<(), ()> {
    let text = serde_json::to_string(event).map_err(|_| ())?;
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ())
}
