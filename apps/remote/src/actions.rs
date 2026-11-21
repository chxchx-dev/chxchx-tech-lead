//! What the person can do, and the reads that keep the screen current. Each one talks to the node
//! and then updates the global signals; they run inside Dioxus tasks.

use crate::model;
use crate::net::{self, HttpError};
use crate::store::{
    self, CONN, Conn, DEVICES, JOBS, JobView, MAX_EVENTS, NODE, PROJECT_DATA, PROJECTS,
    ProjectData, SESSION, Session, toast,
};
use chxchx_protocol::{
    AccessMode, AgentSummary, AgentTurnSpec, ChatSummary, Device, DeviceId, Job, JobAnswer,
    JobEvent, JobEventBody, JobId, JobSpec, NodeStatus, PairRequest, PairResponse, ProjectId,
    ProjectSummary, Provider,
};
use dioxus::prelude::*;

/// A failure, as text for the person. A token the node no longer knows ends this session.
pub fn fail(error: HttpError) -> String {
    if error.is_unauthorized() {
        sign_out();
        return "Este dispositivo ya no está vinculado. Vincúlalo de nuevo.".to_owned();
    }
    error.message
}

/// Links this device with a code from the PC (`chxchx-tech node pair`).
pub async fn pair(code: String, name: String) -> Result<(), String> {
    let request = PairRequest { code, name };
    let linked: PairResponse = net::post("/api/v1/pair", None, &request, None)
        .await
        .map_err(|error| error.message)?;
    let session = Session {
        token: linked.token,
        device: linked.device,
    };
    store::save_session(&session);
    *SESSION.write() = Some(session);
    // The code must not stay in the address bar or the history.
    net::replace_hash("#/");
    crate::engine::session_changed();
    Ok(())
}

pub fn sign_out() {
    store::forget_session();
    *SESSION.write() = None;
    *NODE.write() = None;
    *PROJECTS.write() = None;
    *DEVICES.write() = None;
    JOBS.write().clear();
    PROJECT_DATA.write().clear();
    *CONN.write() = Conn::Connecting;
    crate::engine::session_changed();
}

/// Brings the screen up to date: the node, the jobs and what the running ones said meanwhile.
pub async fn refresh() -> Result<(), HttpError> {
    let Some(token) = store::token() else {
        return Ok(());
    };
    let node: NodeStatus = net::get("/api/v1/node", &token).await?;
    *NODE.write() = Some(node);
    let jobs: Vec<Job> = net::get("/api/v1/jobs", &token).await?;
    let running: Vec<JobId> = jobs
        .iter()
        .filter(|job| !job.state.is_terminal())
        .map(|job| job.id.clone())
        .collect();
    for job in jobs {
        upsert(job);
    }
    for id in running {
        events_of(&id).await?;
    }
    // What the open screen needs that the list does not carry.
    if let crate::route::Route::Job(id) = store::ROUTE.read().clone() {
        events_of(&id).await?;
    }
    *store::CLOCK.write() = net::now_ms();
    Ok(())
}

fn upsert(job: Job) {
    let mut jobs = JOBS.write();
    match jobs.get_mut(&job.id) {
        Some(view) => {
            if job.updated_ms >= view.job.updated_ms {
                view.job = job;
            }
        }
        None => {
            jobs.insert(
                job.id.clone(),
                JobView {
                    job,
                    events: Vec::new(),
                    last_seq: 0,
                },
            );
        }
    }
}

/// Fetches what a job said after the last event heard.
pub async fn events_of(id: &JobId) -> Result<(), HttpError> {
    let Some(token) = store::token() else {
        return Ok(());
    };
    let since = JOBS.read().get(id).map_or(0, |view| view.last_seq);
    let events: Vec<JobEvent> =
        net::get(&format!("/api/v1/jobs/{id}/events?since={since}"), &token).await?;
    for event in events {
        push(event);
    }
    Ok(())
}

/// Adds an event if it is the next one the job was waiting for. Anything else is a gap (fetch it)
/// or a repeat (ignore it).
fn push(event: JobEvent) -> bool {
    let mut jobs = JOBS.write();
    let Some(view) = jobs.get_mut(&event.job) else {
        return false;
    };
    if event.seq != view.last_seq + 1 {
        return false;
    }
    view.last_seq = event.seq;
    if let JobEventBody::State { state } = &event.body {
        view.job.state = *state;
    }
    if view.events.len() < MAX_EVENTS {
        view.events.push(event);
    }
    true
}

/// One event from the live stream.
pub async fn ingest(event: JobEvent) {
    let Some(token) = store::token() else { return };
    let id = event.job.clone();
    if !JOBS.read().contains_key(&id) {
        match net::get::<Job>(&format!("/api/v1/jobs/{id}"), &token).await {
            Ok(job) => upsert(job),
            Err(error) => {
                fail(error);
                return;
            }
        }
    }
    let (last, was_terminal) = JOBS
        .read()
        .get(&id)
        .map(|view| (view.last_seq, view.terminal()))
        .unwrap_or_default();
    if event.seq > last + 1 {
        let _ = events_of(&id).await;
        return;
    }
    let state = match &event.body {
        JobEventBody::State { state } => Some(*state),
        _ => None,
    };
    let asks = matches!(&event.body, JobEventBody::Agent { event } if
        matches!(event["type"].as_str(), Some("permission" | "question"))
            && event["request_id"].as_str().is_some_and(|id| !id.is_empty()));
    let summary = match &event.body {
        JobEventBody::Agent { event } => event["summary"].as_str().unwrap_or_default().to_owned(),
        _ => String::new(),
    };
    if !push(event) {
        return;
    }
    if asks {
        net::notify("ChxChx necesita tu respuesta", &model::clip(&summary, 120));
    }
    if let Some(state) = state
        && state.is_terminal()
        && !was_terminal
    {
        // The result and the error are on the job, not in its events.
        if let Ok(job) = net::get::<Job>(&format!("/api/v1/jobs/{id}"), &token).await {
            net::notify(
                "ChxChx",
                &format!(
                    "{}: {}",
                    model::describe(&job.spec),
                    model::state_label(job.state)
                ),
            );
            upsert(job);
        }
    }
}

pub async fn load_projects() {
    let Some(token) = store::token() else { return };
    match net::get::<Vec<ProjectSummary>>("/api/v1/projects", &token).await {
        Ok(list) => *PROJECTS.write() = Some(list),
        Err(error) => toast(fail(error), true),
    }
}

/// Opens a project's chat: its agents and the conversations saved so far.
pub async fn open_chat(project: &ProjectId) {
    let Some(token) = store::token() else { return };
    let key = project.to_string();
    PROJECT_DATA.write().insert(
        key.clone(),
        ProjectData {
            opened_ms: net::now_ms(),
            ..Default::default()
        },
    );
    let agents_path = format!("/api/v1/projects/{project}/agents");
    let chats_path = format!("/api/v1/projects/{project}/chats");
    let agents = net::get::<Vec<AgentSummary>>(&agents_path, &token);
    let chats = net::get::<Vec<ChatSummary>>(&chats_path, &token);
    let (agents, chats) = futures_util::future::join(agents, chats).await;
    let mut error = None;
    let agents = agents.map_err(|e| error = Some(fail(e))).ok();
    let chats = chats.map_err(|e| error = Some(fail(e))).ok();
    if let Some(data) = PROJECT_DATA.write().get_mut(&key) {
        data.agents = agents;
        data.chats = chats;
        data.error = error;
    }
}

pub struct Draft {
    pub project: ProjectId,
    pub provider: Provider,
    pub prompt: String,
    pub access: AccessMode,
    pub network: bool,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub session: Option<String>,
}

/// Sends a message to an agent: a job. A retry of the same request is the same job.
pub async fn send_turn(draft: Draft) -> Result<JobId, String> {
    let token = store::token().ok_or("No hay sesión")?;
    let spec = JobSpec::AgentTurn(AgentTurnSpec {
        project: draft.project,
        provider: draft.provider,
        prompt: draft.prompt,
        session: draft.session,
        model: draft.model,
        effort: draft.effort,
        access: draft.access,
        network: draft.network,
    });
    let key = net::random_key();
    let job: Job = net::post("/api/v1/jobs", Some(&token), &spec, Some(&key))
        .await
        .map_err(fail)?;
    let id = job.id.clone();
    upsert(job);
    let _ = events_of(&id).await;
    Ok(id)
}

pub async fn answer(job: JobId, answer: JobAnswer) -> Result<(), String> {
    let token = store::token().ok_or("No hay sesión")?;
    net::post::<serde_json::Value>(
        &format!("/api/v1/jobs/{job}/answers"),
        Some(&token),
        &answer,
        None,
    )
    .await
    .map(|_| ())
    .map_err(fail)
}

pub async fn cancel(job: JobId) -> Result<(), String> {
    let token = store::token().ok_or("No hay sesión")?;
    let cancelled: Job = net::post(
        &format!("/api/v1/jobs/{job}/cancel"),
        Some(&token),
        &serde_json::json!({}),
        None,
    )
    .await
    .map_err(fail)?;
    upsert(cancelled);
    Ok(())
}

pub async fn load_devices() {
    let Some(token) = store::token() else { return };
    match net::get::<Vec<Device>>("/api/v1/devices", &token).await {
        Ok(list) => *DEVICES.write() = Some(list),
        Err(error) => toast(fail(error), true),
    }
}

pub async fn revoke(id: DeviceId) -> Result<(), String> {
    let token = store::token().ok_or("No hay sesión")?;
    net::delete::<Device>(&format!("/api/v1/devices/{id}"), &token)
        .await
        .map_err(fail)?;
    load_devices().await;
    Ok(())
}
