//! Node status, jobs and the event stream.

use super::api::{
    ApiError, AppState, Principal, WS_PROTOCOL, error, remote, require, require_local,
};
use crate::features::jobs::JobRunner;
use axum::Extension;
use axum::Json;
use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use chxchx_protocol::{
    DeviceId, ErrorCode, IdempotencyKey, JobAnswer, JobEvent, JobId, JobSpec, JobState,
    PROTOCOL_VERSION, Role,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::OwnedSemaphorePermit;

pub(super) async fn node(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
) -> Json<Value> {
    let jobs = state.runner.list();
    let count = |wanted: JobState| jobs.iter().filter(|job| job.state == wanted).count();
    let mut body = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": PROTOCOL_VERSION,
        "started_ms": state.started_ms,
        "jobs": { "queued": count(JobState::Queued), "running": count(JobState::Running) },
        "you": { "role": who.role().as_str(), "local": who.is_local() },
    });
    if who.is_local() {
        // Process details are for the person at this PC.
        body["pid"] = json!(std::process::id());
        body["remote"] = json!(remote(&state).map(|remote| remote.url()));
    }
    Json(body)
}

pub(super) async fn stop(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    require_local(&who)?;
    state.log("parada pedida por la API");
    state.audit(&who, "node_stop", json!({}));
    state.stop.notify_one();
    Ok((StatusCode::ACCEPTED, Json(json!({ "status": "stopping" }))))
}

pub(super) async fn list(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
) -> Result<Json<Vec<chxchx_protocol::Job>>, ApiError> {
    require(&who, Role::Reader)?;
    Ok(Json(state.runner.list()))
}

pub(super) async fn create(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<chxchx_protocol::Job>), ApiError> {
    let mut spec: JobSpec = serde_json::from_slice(&body).map_err(|problem| {
        error(
            ErrorCode::InvalidRequest,
            format!("Petición no válida: {problem}"),
        )
    })?;
    // A key belongs to whoever sent it: two devices using the same text are not the same request,
    // and one can never be handed the other's job (or its answer) by repeating a key.
    let key = headers
        .get("idempotency-key")
        .map(|value| {
            let text = value
                .to_str()
                .map_err(|_| error(ErrorCode::InvalidRequest, "Idempotency-Key no válida"))?;
            IdempotencyKey::parse(text)
                .and_then(|_| IdempotencyKey::parse(format!("{}:{text}", who.actor())))
                .map_err(|_| {
                    error(
                        ErrorCode::InvalidRequest,
                        "Idempotency-Key no válida o demasiado larga",
                    )
                })
        })
        .transpose()?;

    // What the caller may ask for. The job is stored as it will run, so what is listed, retried
    // with the same key and audited is the effective request, not the one that was refused.
    let mut detail = json!({});
    match &mut spec {
        JobSpec::AgentTurn(turn) => {
            require(&who, Role::Operator)?;
            let (asked_access, asked_network) = (turn.access, turn.network);
            turn.access = turn.access.capped(who.access_cap());
            turn.network = turn.network && who.may_use_network();
            detail = json!({
                "project": turn.project,
                "provider": turn.provider,
                "access": turn.access,
                "network": turn.network,
                "capped": asked_access != turn.access || asked_network != turn.network,
                "asked_access": asked_access,
                "asked_network": asked_network,
            });
        }
        // Workspace and local-model jobs change the machine: admin only.
        _ => require(&who, Role::Admin)?,
    }
    let runner = state.runner.clone();
    // Writing the job is a synced file write: kept off the async workers.
    let (job, created) = tokio::task::spawn_blocking(move || runner.submit_tracked(spec, key))
        .await
        .map_err(|_| error(ErrorCode::Internal, "Falló el registro del trabajo"))??;
    if created {
        detail["job"] = json!(job.id);
        state.audit(&who, "job_created", detail);
        state.log(&format!("trabajo {} recibido de {}", job.id, who.actor()));
    } else {
        // A retry is not a new job: the audit log says what happened, once.
        state.log(&format!(
            "trabajo {}: reintento de {} (ya existía)",
            job.id,
            who.actor()
        ));
    }
    Ok((StatusCode::ACCEPTED, Json(job)))
}

fn job_id(raw: &str) -> Result<JobId, ApiError> {
    JobId::parse(raw).map_err(|problem| error(ErrorCode::InvalidRequest, problem.to_string()))
}

fn not_found(id: &JobId) -> ApiError {
    error(ErrorCode::NotFound, format!("No existe el trabajo {id}"))
}

pub(super) async fn show(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<chxchx_protocol::Job>, ApiError> {
    require(&who, Role::Reader)?;
    let id = job_id(&id)?;
    state
        .runner
        .get(&id)
        .map(Json)
        .ok_or_else(|| not_found(&id))
}

#[derive(Deserialize)]
pub(super) struct Since {
    #[serde(default)]
    since: u64,
}

pub(super) async fn events(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
    Query(query): Query<Since>,
) -> Result<Json<Vec<JobEvent>>, ApiError> {
    require(&who, Role::Reader)?;
    let id = job_id(&id)?;
    if state.runner.get(&id).is_none() {
        return Err(not_found(&id));
    }
    Ok(Json(state.runner.events_since(&id, query.since)))
}

pub(super) async fn cancel(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<chxchx_protocol::Job>, ApiError> {
    require(&who, Role::Operator)?;
    let id = job_id(&id)?;
    let runner = state.runner.clone();
    let job = tokio::task::spawn_blocking(move || runner.cancel(&id))
        .await
        .map_err(|_| error(ErrorCode::Internal, "Falló la cancelación"))??;
    state.audit(&who, "job_cancelled", json!({ "job": job.id }));
    state.log(&format!("trabajo {} cancelado ({:?})", job.id, job.state));
    Ok(Json(job))
}

pub(super) async fn answer(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    require(&who, Role::Operator)?;
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
    let decision = behavior.map_or("pregunta".to_owned(), |b| format!("{b:?}").to_lowercase());
    state.audit(
        &who,
        "job_answered",
        json!({ "job": id, "request": request_id, "decision": decision }),
    );
    state.log(&format!(
        "trabajo {id}: respuesta a {request_id} ({decision})"
    ));
    Ok((StatusCode::ACCEPTED, Json(json!({ "status": "delivered" }))))
}

#[derive(Deserialize)]
pub(super) struct Stream {
    job: Option<String>,
    #[serde(default)]
    since: u64,
}

/// Events as they happen, one JSON `JobEvent` per text message. With `job`, it first sends that
/// job's events after `since`, so a client that reconnects only gets what it missed.
pub(super) async fn stream(
    State(state): State<AppState>,
    Extension(who): Extension<Principal>,
    Query(query): Query<Stream>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    require(&who, Role::Reader)?;
    let job = query.job.as_deref().map(job_id).transpose()?;
    if let Some(id) = &job
        && state.runner.get(id).is_none()
    {
        return Err(not_found(id));
    }
    let Ok(permit) = state.sockets.clone().try_acquire_owned() else {
        return Err(error(
            ErrorCode::TooManyRequests,
            "Hay demasiadas conexiones de eventos abiertas",
        ));
    };
    let runner = state.runner.clone();
    let watch = match &who {
        Principal::Device(device) => Some((device.id.clone(), state.devices.clone())),
        Principal::Local => None,
    };
    Ok(upgrade
        .protocols([WS_PROTOCOL])
        .on_upgrade(move |socket| forward(socket, runner, job, query.since, watch, permit)))
}

type Watch = Option<(DeviceId, Arc<Mutex<crate::features::node_devices::Devices>>)>;

async fn forward(
    mut socket: WebSocket,
    runner: JobRunner,
    job: Option<JobId>,
    since: u64,
    watch: Watch,
    _permit: OwnedSemaphorePermit,
) {
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
    // A revoked device must not keep listening on a connection it opened before.
    let mut recheck = tokio::time::interval(Duration::from_secs(1));
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
            _ = recheck.tick() => {
                if let Some((id, devices)) = &watch
                    && !devices
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner())
                        .is_linked(id)
                {
                    let _ = socket.send(Message::Close(None)).await;
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
