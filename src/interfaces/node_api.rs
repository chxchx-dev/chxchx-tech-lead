//! The node's HTTP API v1 (ADR 0017). Every route but `health` needs the local token, and every
//! request must be addressed to loopback: a web page in a browser can send requests to
//! `127.0.0.1`, so a foreign `Host` (DNS rebinding) or `Origin` is refused before anything else.

use crate::features::jobs::JobRunner;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chxchx_protocol::{
    ErrorCode, IdempotencyKey, JobError, JobId, JobSpec, JobState, PROTOCOL_VERSION,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Notify;

/// Large enough for the biggest prompt a job accepts, and no more.
const MAX_BODY_BYTES: usize = chxchx_protocol::MAX_PROMPT_BYTES + 64 * 1024;

#[derive(Clone)]
pub(crate) struct AppState {
    pub runner: JobRunner,
    pub token: Arc<str>,
    pub port: u16,
    pub started_ms: u64,
    pub stop: Arc<Notify>,
}

pub(crate) fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/api/v1/node", get(node))
        .route("/api/v1/node/stop", post(stop))
        .route("/api/v1/jobs", get(list).post(create))
        .route("/api/v1/jobs/{id}", get(show))
        .route("/api/v1/jobs/{id}/events", get(events))
        .route("/api/v1/jobs/{id}/cancel", post(cancel))
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
        return Err(error(
            ErrorCode::Forbidden,
            "El nodo solo atiende peticiones dirigidas a 127.0.0.1",
        ));
    }
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin = origin.to_str().unwrap_or_default();
        if !allowed(origin, "http://") {
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
    let given = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !same(given.as_bytes(), state.token.as_bytes()) {
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
    Ok(Json(job))
}
