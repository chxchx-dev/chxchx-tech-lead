use super::api::{AppState, router};
use crate::features::jobs::{Context, Executor, JobRunner, JobStore};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use chxchx_protocol::{ErrorCode, Job, JobError, JobState};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tower::ServiceExt;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const PORT: u16 = 47_999;

struct Done;

impl Executor for Done {
    fn run(&self, _job: &Job, _context: &Context) -> Result<Option<String>, JobError> {
        Ok(Some("hecho".into()))
    }
}

fn state(name: &str) -> AppState {
    let dir = std::env::temp_dir().join(format!("chxchx-node-api-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    AppState {
        runner: JobRunner::new(JobStore::open(&dir).unwrap(), Done, 1),
        token: TOKEN.into(),
        port: PORT,
        started_ms: 1,
        stop: Arc::new(Notify::new()),
    }
}

fn turn() -> Value {
    json!({
        "kind": "agent_turn", "project": "demo", "provider": "claude",
        "prompt": "hola", "access": "read-only"
    })
}

struct Call {
    method: &'static str,
    path: String,
    token: Option<&'static str>,
    host: String,
    origin: Option<&'static str>,
    key: Option<&'static str>,
    body: Option<String>,
}

impl Call {
    fn new(method: &'static str, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            token: Some(TOKEN),
            host: format!("127.0.0.1:{PORT}"),
            origin: None,
            key: None,
            body: None,
        }
    }

    async fn send(self, state: &AppState) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(self.method)
            .uri(&self.path)
            .header(header::HOST, &self.host);
        if let Some(token) = self.token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(origin) = self.origin {
            request = request.header(header::ORIGIN, origin);
        }
        if let Some(key) = self.key {
            request = request.header("idempotency-key", key);
        }
        let body = self.body.map(Body::from).unwrap_or_else(Body::empty);
        let response = router(state.clone())
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
}

#[tokio::test]
async fn health_is_open_and_everything_else_needs_the_token() {
    let state = state("auth");
    let mut call = Call::new("GET", "/api/v1/health");
    call.token = None;
    let (status, body) = call.send(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "status": "ok", "protocol": 1 }));

    for token in [None, Some("wrong"), Some(&TOKEN[..63])] {
        let mut call = Call::new("GET", "/api/v1/jobs");
        call.token = token;
        let (status, body) = call.send(&state).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{token:?}");
        assert_eq!(body["code"], "unauthorized");
    }
    let (status, body) = Call::new("GET", "/api/v1/node").send(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jobs"], json!({ "queued": 0, "running": 0 }));
}

#[tokio::test]
async fn requests_not_addressed_to_loopback_are_refused_even_with_the_token() {
    let state = state("loopback");
    let mut rebinding = Call::new("GET", "/api/v1/health");
    rebinding.host = format!("evil.example:{PORT}");
    assert_eq!(rebinding.send(&state).await.0, StatusCode::FORBIDDEN);

    let mut other_port = Call::new("GET", "/api/v1/jobs");
    other_port.host = "127.0.0.1:1".into();
    assert_eq!(other_port.send(&state).await.0, StatusCode::FORBIDDEN);

    let mut web_page = Call::new("POST", "/api/v1/jobs");
    web_page.origin = Some("https://evil.example");
    web_page.body = Some(turn().to_string());
    let (status, body) = web_page.send(&state).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "forbidden");
    assert!(state.runner.list().is_empty(), "nothing was created");

    let mut same_origin = Call::new("GET", "/api/v1/jobs");
    same_origin.origin = Some("http://localhost:47999");
    same_origin.host = format!("localhost:{PORT}");
    assert_eq!(same_origin.send(&state).await.0, StatusCode::OK);
}

#[tokio::test]
async fn a_job_is_created_once_per_key_and_can_be_followed() {
    let state = state("jobs");
    let mut create = Call::new("POST", "/api/v1/jobs");
    create.body = Some(turn().to_string());
    create.key = Some("k-1");
    let (status, first) = create.send(&state).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let id = first["id"].as_str().unwrap().to_owned();

    let mut again = Call::new("POST", "/api/v1/jobs");
    again.body = Some(turn().to_string());
    again.key = Some("k-1");
    assert_eq!(again.send(&state).await.1["id"], id.as_str());

    assert!(state.runner.wait_idle(Duration::from_secs(5)));
    let (status, job) = Call::new("GET", format!("/api/v1/jobs/{id}"))
        .send(&state)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(job["state"], "succeeded");
    assert_eq!(job["summary"], "hecho");

    let (_, events) = Call::new("GET", format!("/api/v1/jobs/{id}/events?since=1"))
        .send(&state)
        .await;
    let seqs: Vec<u64> = events
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["seq"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, [2, 3], "only what came after `since`");
    let (_, list) = Call::new("GET", "/api/v1/jobs").send(&state).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn bad_requests_get_a_typed_error_and_change_nothing() {
    let state = state("bad");
    let cases = [
        ("not json".to_owned(), "invalid_request"),
        (
            json!({"kind": "shell", "command": "rm -rf /"}).to_string(),
            "invalid_request",
        ),
        (
            json!({
                "kind": "agent_turn", "project": "../etc", "provider": "claude",
                "prompt": "hola", "access": "read-only"
            })
            .to_string(),
            "invalid_request",
        ),
        (
            json!({
                "kind": "agent_turn", "project": "demo", "provider": "claude",
                "prompt": "  ", "access": "read-only"
            })
            .to_string(),
            "invalid_request",
        ),
    ];
    for (body, code) in cases {
        let mut call = Call::new("POST", "/api/v1/jobs");
        call.body = Some(body.clone());
        let (status, error) = call.send(&state).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(error["code"], code);
    }
    let mut bad_key = Call::new("POST", "/api/v1/jobs");
    bad_key.body = Some(turn().to_string());
    bad_key.key = Some("has spaces");
    assert_eq!(bad_key.send(&state).await.0, StatusCode::BAD_REQUEST);
    assert!(state.runner.list().is_empty());

    let (status, _) = Call::new("GET", "/api/v1/jobs/..%2Fetc").send(&state).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, error) = Call::new("POST", "/api/v1/jobs/missing/cancel")
        .send(&state)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["code"], "not_found");
}

#[tokio::test]
async fn a_huge_body_is_refused_before_it_is_read() {
    let state = state("huge");
    let mut call = Call::new("POST", "/api/v1/jobs");
    call.body = Some("x".repeat(2 * 1024 * 1024));
    assert_eq!(call.send(&state).await.0, StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn a_real_node_answers_on_loopback_and_stops_when_asked() {
    let dir = std::env::temp_dir().join(format!("chxchx-node-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runner = JobRunner::new(JobStore::open(&dir.join("jobs")).unwrap(), Done, 1);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let serve_dir = dir.clone();
    let server = runtime.spawn(async move {
        super::serve(listener, runner, TOKEN.into(), &serve_dir, false).await
    });

    use crate::integrations::node_client::request;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let health = loop {
        match request(port, "GET", "/api/v1/health", None, None) {
            Ok(response) => break response,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) => panic!("{error}"),
        }
    };
    assert_eq!(health.status, 200);
    let info = crate::features::node_state::read_info(&dir).expect("node.json is written");
    assert_eq!(info.port, port);

    let created = request(
        port,
        "POST",
        "/api/v1/jobs",
        Some(TOKEN),
        Some(&turn().to_string()),
    )
    .unwrap();
    assert_eq!(created.status, 202, "{}", created.body);
    let job: Job = serde_json::from_str(&created.body).unwrap();
    assert_eq!(job.state, JobState::Queued);

    let refused = request(port, "GET", "/api/v1/jobs", Some("wrong"), None).unwrap();
    assert_eq!(refused.status, 401);
    let error: JobError = serde_json::from_str(&refused.body).unwrap();
    assert_eq!(error.code, ErrorCode::Unauthorized);

    let stopping = request(port, "POST", "/api/v1/node/stop", Some(TOKEN), None).unwrap();
    assert_eq!(stopping.status, 202);
    runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(10), server).await })
        .expect("the node stops when asked")
        .unwrap()
        .unwrap();
}
