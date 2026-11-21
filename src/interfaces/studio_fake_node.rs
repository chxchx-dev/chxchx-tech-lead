//! A node in miniature for Studio's tests: a real socket on loopback that speaks just enough of
//! the API (jobs and their events) and plays a script. The real node is tested in
//! `node_link_tests`; this is for what Studio does with one.
use crate::integrations::node_link::Endpoint;
use chxchx_protocol::{
    AccessMode, AgentTurnSpec, ErrorCode, Job, JobError, JobEvent, JobEventBody, JobId, JobSpec,
    JobState, ProjectId, Provider,
};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
pub(super) struct Script {
    /// The jobs the node has, with what each said.
    pub jobs: Vec<(Job, Vec<JobEvent>)>,
    /// What a job that Studio creates says, between it starting and ending well.
    pub on_create: Vec<Value>,
    /// Answers `POST /jobs` with an error.
    pub refuse_create: bool,
    /// The bodies of the jobs Studio asked for.
    pub created: Vec<Value>,
}

pub(super) struct FakeNode {
    pub port: u16,
    pub script: Arc<Mutex<Script>>,
    stop: Arc<AtomicBool>,
}

/// A message from the agent, as the node reports it.
pub(super) fn agent(event: Value) -> Value {
    let mut event = event;
    event["schema"] = json!("chxchx.agent-event");
    event["schema_version"] = json!(1);
    event
}

pub(super) fn event(id: &JobId, seq: u64, body: JobEventBody) -> JobEvent {
    JobEvent {
        job: id.clone(),
        seq,
        at_ms: seq,
        body,
    }
}

/// A turn of an agent in a project, as the node lists it.
pub(super) fn turn_job(
    id: &str,
    project: &str,
    provider: Provider,
    prompt: &str,
    state: JobState,
) -> Job {
    Job {
        id: JobId::parse(id).unwrap(),
        spec: JobSpec::AgentTurn(AgentTurnSpec {
            project: ProjectId::parse(project).unwrap(),
            provider,
            prompt: prompt.into(),
            session: None,
            model: None,
            effort: None,
            access: AccessMode::WorkspaceWrite,
            network: false,
        }),
        state,
        created_ms: 1,
        updated_ms: 1,
        idempotency_key: None,
        summary: None,
        error: None,
    }
}

impl FakeNode {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let script = Arc::new(Mutex::new(Script::default()));
        let stop = Arc::new(AtomicBool::new(false));
        {
            let (script, stop) = (script.clone(), stop.clone());
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let script = script.clone();
                            std::thread::spawn(move || serve(stream, &script));
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(3)),
                    }
                }
            });
        }
        Self { port, script, stop }
    }

    pub fn endpoint(&self) -> Endpoint {
        Endpoint::new(self.port, "fake-token")
    }
}

impl Drop for FakeNode {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn serve(mut stream: TcpStream, script: &Mutex<Script>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    let (head_end, length) = loop {
        let Ok(read) = stream.read(&mut chunk) else {
            return;
        };
        if read == 0 {
            return;
        }
        raw.extend_from_slice(&chunk[..read]);
        if let Some(at) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..at]).to_ascii_lowercase();
            let length = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            break (at + 4, length);
        }
    };
    while raw.len() < head_end + length {
        let Ok(read) = stream.read(&mut chunk) else {
            return;
        };
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..read]);
    }
    let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
    let body = String::from_utf8_lossy(&raw[head_end..]).into_owned();
    let mut first = head.lines().next().unwrap_or_default().split_whitespace();
    let (method, target) = (
        first.next().unwrap_or_default(),
        first.next().unwrap_or_default(),
    );
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let (code, payload) = route(method, path, query, &body, script);
    let text = payload.to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    );
}

fn route(
    method: &str,
    path: &str,
    query: &str,
    body: &str,
    script: &Mutex<Script>,
) -> (u16, Value) {
    let mut script = script.lock().unwrap_or_else(|poison| poison.into_inner());
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match (method, parts.as_slice()) {
        ("GET", ["api", "v1", "health"]) => (200, json!({"status": "ok"})),
        ("GET", ["api", "v1", "jobs"]) => {
            let jobs: Vec<&Job> = script.jobs.iter().map(|(job, _)| job).collect();
            (200, serde_json::to_value(jobs).unwrap())
        }
        ("POST", ["api", "v1", "jobs"]) => {
            if script.refuse_create {
                return (
                    503,
                    serde_json::to_value(JobError::new(ErrorCode::Internal, "no puedo ahora"))
                        .unwrap(),
                );
            }
            let asked: Value = serde_json::from_str(body).unwrap_or(Value::Null);
            script.created.push(asked.clone());
            let id = JobId::parse(format!("fake-{}", script.jobs.len() + 1)).unwrap();
            let spec: JobSpec = serde_json::from_value(asked).unwrap();
            let mut events = vec![event(
                &id,
                1,
                JobEventBody::State {
                    state: JobState::Running,
                },
            )];
            for said in script.on_create.clone() {
                let seq = events.len() as u64 + 1;
                events.push(event(&id, seq, JobEventBody::Agent { event: said }));
            }
            let seq = events.len() as u64 + 1;
            events.push(event(
                &id,
                seq,
                JobEventBody::State {
                    state: JobState::Succeeded,
                },
            ));
            let mut job = turn_job(
                id.as_str(),
                "demo",
                Provider::Codex,
                "",
                JobState::Succeeded,
            );
            job.spec = spec;
            script.jobs.push((job.clone(), events));
            job.state = JobState::Running;
            (202, serde_json::to_value(job).unwrap())
        }
        ("GET", ["api", "v1", "jobs", id]) => {
            match script.jobs.iter().find(|(job, _)| job.id.as_str() == *id) {
                Some((job, _)) => (200, serde_json::to_value(job).unwrap()),
                None => (404, json!({"code": "not_found", "message": "no existe"})),
            }
        }
        ("GET", ["api", "v1", "jobs", id, "events"]) => {
            let since = query
                .split('&')
                .find_map(|pair| pair.strip_prefix("since="))
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);
            match script.jobs.iter().find(|(job, _)| job.id.as_str() == *id) {
                Some((_, events)) => {
                    let news: Vec<&JobEvent> = events.iter().filter(|e| e.seq > since).collect();
                    (200, serde_json::to_value(news).unwrap())
                }
                None => (404, json!({"code": "not_found", "message": "no existe"})),
            }
        }
        _ => (
            404,
            json!({"code": "not_found", "message": "ruta desconocida"}),
        ),
    }
}
