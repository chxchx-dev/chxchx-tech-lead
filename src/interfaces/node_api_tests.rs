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
    state_named(name)
}

fn state_named(name: &str) -> AppState {
    let dir = std::env::temp_dir().join(format!("chxchx-node-api-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    AppState::new(
        JobRunner::attended(JobStore::open(&dir.join("jobs")).unwrap(), Done, 1),
        TOKEN.into(),
        PORT,
        1,
        Arc::new(Notify::new()),
        dir,
    )
    .unwrap()
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
    token: Option<String>,
    host: String,
    origin: Option<String>,
    key: Option<&'static str>,
    body: Option<String>,
}

impl Call {
    fn new(method: &'static str, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            token: Some(TOKEN.to_owned()),
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
        call.token = token.map(str::to_owned);
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
    web_page.origin = Some("https://evil.example".to_owned());
    web_page.body = Some(turn().to_string());
    let (status, body) = web_page.send(&state).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "forbidden");
    assert!(state.runner.list().is_empty(), "nothing was created");

    let mut same_origin = Call::new("GET", "/api/v1/jobs");
    same_origin.origin = Some("http://localhost:47999".to_owned());
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
async fn answers_are_checked_before_they_reach_a_job() {
    let state = state("answers");
    let mut create = Call::new("POST", "/api/v1/jobs");
    create.body = Some(turn().to_string());
    let id = create.send(&state).await.1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(state.runner.wait_idle(Duration::from_secs(5)));

    let answer = |body: Value| {
        let mut call = Call::new("POST", format!("/api/v1/jobs/{id}/answers"));
        call.body = Some(body.to_string());
        call
    };
    let (status, error) = answer(json!({"request_id": "r", "behavior": "maybe"}))
        .send(&state)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["code"], "invalid_request");
    let (status, _) = answer(json!({"request_id": "r"})).send(&state).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "neither allow/deny nor answers"
    );
    let (status, error) = answer(json!({"request_id": "r", "behavior": "allow"}))
        .send(&state)
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "the job is over: {error}");

    let mut unknown = Call::new("POST", "/api/v1/jobs/nope/answers");
    unknown.body = Some(json!({"request_id": "r", "behavior": "deny"}).to_string());
    assert_eq!(unknown.send(&state).await.0, StatusCode::NOT_FOUND);

    let log = std::fs::read_to_string(state.dir.join("node.log")).unwrap_or_default();
    assert!(log.contains(&format!("trabajo {id} recibido")), "{log}");
}

#[tokio::test]
async fn a_rejected_token_is_logged_without_the_token() {
    let state = state("log");
    let mut call = Call::new("GET", "/api/v1/jobs");
    call.token = Some("secret-attempt-123".to_owned());
    assert_eq!(call.send(&state).await.0, StatusCode::UNAUTHORIZED);
    let log = std::fs::read_to_string(state.dir.join("node.log")).unwrap();
    assert!(log.contains("token ausente o no válido en /api/v1/jobs"));
    assert!(!log.contains("secret-attempt-123"));
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

/// A WebSocket client just big enough for these tests: handshake, then text frames.
mod ws {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    pub struct Socket(TcpStream);

    /// Opens `path` with `protocols`; the status line of the answer when it is not 101.
    pub fn open(port: u16, path: &str, protocols: &str) -> Result<Socket, String> {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: {protocols}\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
            head.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&head).to_string();
        let status = head.lines().next().unwrap_or_default().to_owned();
        if !status.contains(" 101 ") {
            return Err(status);
        }
        assert!(
            head.to_ascii_lowercase()
                .contains("sec-websocket-protocol: chxchx.v1"),
            "{head}"
        );
        Ok(Socket(stream))
    }

    impl Socket {
        /// How long `text` waits before saying nothing came.
        pub fn wait(&mut self, limit: Duration) {
            self.0.set_read_timeout(Some(limit)).unwrap();
        }

        /// The next text message, or `None` when nothing came in time.
        pub fn text(&mut self) -> Option<String> {
            let mut head = [0u8; 2];
            self.0.read_exact(&mut head).ok()?;
            let mut length = u64::from(head[1] & 0x7f);
            if length == 126 {
                let mut extended = [0u8; 2];
                self.0.read_exact(&mut extended).ok()?;
                length = u64::from(u16::from_be_bytes(extended));
            } else if length == 127 {
                let mut extended = [0u8; 8];
                self.0.read_exact(&mut extended).ok()?;
                length = u64::from_be_bytes(extended);
            }
            let mut payload = vec![0u8; length as usize];
            self.0.read_exact(&mut payload).ok()?;
            (head[0] & 0x0f == 1).then(|| String::from_utf8(payload).unwrap())
        }
    }
}

#[test]
fn events_stream_live_and_a_reconnection_only_gets_what_it_missed() {
    let dir = std::env::temp_dir().join(format!("chxchx-node-ws-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runner = JobRunner::attended(JobStore::open(&dir.join("jobs")).unwrap(), Done, 1);
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
    while request(port, "GET", "/api/v1/health", None, None).is_err() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }

    let bearer = format!("chxchx.v1, bearer.{TOKEN}");
    assert!(
        ws::open(port, "/api/v1/events", "chxchx.v1, bearer.wrong")
            .err()
            .unwrap()
            .contains("401"),
        "the token travels as a subprotocol and is checked"
    );
    let mut all = ws::open(port, "/api/v1/events", &bearer).unwrap();
    let created = request(
        port,
        "POST",
        "/api/v1/jobs",
        Some(TOKEN),
        Some(&turn().to_string()),
    )
    .unwrap();
    let id = serde_json::from_str::<Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut live = Vec::new();
    while live.len() < 3 {
        let event: Value = serde_json::from_str(&all.text().expect("live event")).unwrap();
        live.push(event);
    }
    let states: Vec<&str> = live.iter().map(|e| e["state"].as_str().unwrap()).collect();
    assert_eq!(states, ["queued", "running", "succeeded"]);

    let mut again = ws::open(port, &format!("/api/v1/events?job={id}&since=2"), &bearer).unwrap();
    let missed: Value = serde_json::from_str(&again.text().unwrap()).unwrap();
    assert_eq!(
        (missed["seq"].as_u64(), missed["state"].as_str()),
        (Some(3), Some("succeeded"))
    );
    again.wait(Duration::from_millis(300));
    assert!(again.text().is_none(), "nothing it had already seen");

    assert!(
        ws::open(port, "/api/v1/events?job=missing", &bearer)
            .err()
            .unwrap()
            .contains("404")
    );
    request(port, "POST", "/api/v1/node/stop", Some(TOKEN), None).unwrap();
    runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(10), server).await })
        .expect("the node stops with sockets still open")
        .unwrap()
        .unwrap();
}

// ---- H3: linked devices, roles, the Tailscale name and limits ------------------------------

impl Call {
    fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_owned());
        self
    }

    fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body.to_string());
        self
    }
}

/// Links a device the way a person would: a code on the PC, typed on the new device.
async fn link(state: &AppState, role: &str) -> (String, String) {
    let (status, offer) = Call::new("POST", "/api/v1/devices/pairing")
        .with_body(json!({ "role": role }))
        .send(state)
        .await;
    assert_eq!(status, StatusCode::OK, "{offer}");
    let mut pair = Call::new("POST", "/api/v1/pair").with_body(json!({
        "code": offer["code"], "name": "Pixel de Ana"
    }));
    pair.token = None; // the new device has no token yet: the code is what proves it
    let (status, linked) = pair.send(state).await;
    assert_eq!(status, StatusCode::OK, "{linked}");
    (
        linked["device"]["id"].as_str().unwrap().to_owned(),
        linked["token"].as_str().unwrap().to_owned(),
    )
}

#[tokio::test]
async fn a_linked_device_can_only_do_what_its_role_allows() {
    let state = state("roles");
    let (_, reader) = link(&state, "reader").await;
    let (operator_id, operator) = link(&state, "operator").await;
    let (_, admin) = link(&state, "admin").await;
    assert_eq!(reader.len(), 64);

    let code = |call: Call| async { call.send(&state).await.0 };
    assert_eq!(
        code(Call::new("GET", "/api/v1/jobs").with_token(&reader)).await,
        StatusCode::OK
    );
    let (_, me) = Call::new("GET", "/api/v1/node")
        .with_token(&reader)
        .send(&state)
        .await;
    assert_eq!(me["you"], json!({ "role": "reader", "local": false }));
    assert!(
        me.get("pid").is_none(),
        "process details are for the person at the PC"
    );

    let job = |token: &str| {
        Call::new("POST", "/api/v1/jobs")
            .with_token(token)
            .with_body(turn())
    };
    assert_eq!(
        code(job(&reader)).await,
        StatusCode::FORBIDDEN,
        "a reader only looks"
    );
    assert_eq!(code(job(&operator)).await, StatusCode::ACCEPTED);
    let workspace = json!({ "kind": "workspace_start", "project": "demo" });
    let start = Call::new("POST", "/api/v1/jobs")
        .with_token(&operator)
        .with_body(workspace);
    assert_eq!(
        code(start).await,
        StatusCode::FORBIDDEN,
        "changing the machine needs admin"
    );

    for path in ["/api/v1/devices", "/api/v1/audit"] {
        assert_eq!(
            code(Call::new("GET", path).with_token(&operator)).await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            code(Call::new("GET", path).with_token(&admin)).await,
            StatusCode::OK
        );
    }
    let pairing = Call::new("POST", "/api/v1/devices/pairing").with_token(&admin);
    assert_eq!(
        code(pairing).await,
        StatusCode::FORBIDDEN,
        "new devices are linked on the PC"
    );
    let stop = Call::new("POST", "/api/v1/node/stop").with_token(&admin);
    assert_eq!(
        code(stop).await,
        StatusCode::FORBIDDEN,
        "so is stopping the node"
    );

    let (_, list) = Call::new("GET", "/api/v1/devices")
        .with_token(&admin)
        .send(&state)
        .await;
    assert_eq!(list.as_array().unwrap().len(), 3);
    assert!(
        !list.to_string().contains(&operator),
        "a token is never listed"
    );

    let revoke = Call::new("DELETE", format!("/api/v1/devices/{operator_id}")).with_token(&admin);
    assert_eq!(code(revoke).await, StatusCode::OK);
    let after = Call::new("GET", "/api/v1/jobs").with_token(&operator);
    assert_eq!(
        code(after).await,
        StatusCode::UNAUTHORIZED,
        "revoked at once"
    );
}

#[tokio::test]
async fn a_pairing_code_works_once_and_guessing_it_is_stopped() {
    let state = state("pairing");
    let (status, offer) = Call::new("POST", "/api/v1/devices/pairing")
        .send(&state)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(offer["role"], "operator", "operator unless said otherwise");
    let pair = |code: Value| {
        let mut call =
            Call::new("POST", "/api/v1/pair").with_body(json!({ "code": code, "name": "X" }));
        call.token = None;
        call
    };
    assert_eq!(
        pair(offer["code"].clone()).send(&state).await.0,
        StatusCode::OK
    );
    assert_eq!(
        pair(offer["code"].clone()).send(&state).await.0,
        StatusCode::UNAUTHORIZED,
        "used"
    );

    // A clean node, so the count of wrong tries starts at zero.
    let state = state_named("pairing-guess");
    let (_, fresh) = Call::new("POST", "/api/v1/devices/pairing")
        .send(&state)
        .await;
    for _ in 0..5 {
        let (status, _) = pair(json!("AAAAA-AAAAA")).send(&state).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, body) = pair(fresh["code"].clone()).send(&state).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["code"], "too_many_requests");
}

#[tokio::test]
async fn a_device_cannot_raise_its_own_access_level() {
    let state = state("cap");
    let (_, operator) = link(&state, "operator").await;
    let (_, admin) = link(&state, "admin").await;
    let greedy = json!({
        "kind": "agent_turn", "project": "demo", "provider": "claude",
        "prompt": "hola", "access": "full-access", "network": true
    });
    let post = |token: Option<&str>| {
        let mut call = Call::new("POST", "/api/v1/jobs").with_body(greedy.clone());
        call.key = Some("cap-1");
        if let Some(token) = token {
            call = call.with_token(token);
        }
        call
    };

    let (_, job) = post(Some(&operator)).send(&state).await;
    assert_eq!(
        job["spec"]["access"], "workspace-write",
        "lowered, not refused: {job}"
    );
    assert_eq!(job["spec"]["network"], false);
    let (_, again) = post(Some(&operator)).send(&state).await;
    assert_eq!(again["id"], job["id"], "a retry is still the same job");

    let mut admin_post = post(Some(&admin));
    admin_post.key = Some("cap-2");
    let (_, job) = admin_post.send(&state).await;
    assert_eq!(
        job["spec"]["access"], "workspace-write",
        "admin devices are capped too"
    );
    assert_eq!(
        job["spec"]["network"], true,
        "but an admin may allow the network"
    );

    let mut local = post(None);
    local.key = Some("cap-3");
    let (_, job) = local.send(&state).await;
    assert_eq!(
        job["spec"]["access"], "full-access",
        "the person at the PC is not capped"
    );

    let (_, audit) = Call::new("GET", "/api/v1/audit").send(&state).await;
    let capped: Vec<&Value> = audit
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["action"] == "job_created" && entry["detail"]["capped"] == true)
        .collect();
    assert_eq!(
        capped.len(),
        2,
        "both lowered requests were recorded: {audit}"
    );
    assert_eq!(capped[0]["detail"]["asked_access"], "full-access");
}

#[tokio::test]
async fn the_tailscale_name_is_accepted_only_after_it_is_enabled_and_never_for_the_local_token() {
    use crate::features::node_state::{RemoteConfig, write_remote};
    let state = state("remote");
    let (_, device) = link(&state, "operator").await;
    let name = "fedora.tail33e808.ts.net";
    let via = |host: &str, origin: Option<&str>, token: &str| {
        let mut call = Call::new("GET", "/api/v1/jobs").with_token(token);
        call.host = host.to_owned();
        call.origin = origin.map(str::to_owned);
        call
    };

    assert_eq!(
        via(name, None, &device).send(&state).await.0,
        StatusCode::FORBIDDEN,
        "not enabled yet: only loopback"
    );
    write_remote(&state.dir, &RemoteConfig::new(name, 443).unwrap()).unwrap();
    let origin = format!("https://{name}");
    assert_eq!(
        via(name, Some(&origin), &device).send(&state).await.0,
        StatusCode::OK
    );
    assert_eq!(
        via(&name.to_uppercase(), None, &device)
            .send(&state)
            .await
            .0,
        StatusCode::OK
    );

    for (host, origin) in [
        ("evil.example", None),
        (&format!("{name}:8443"), None),
        (name, Some("https://evil.example")),
        (name, Some("http://127.0.0.1:47999")),
    ] {
        let (status, _) = via(host, origin, &device).send(&state).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{host} {origin:?}");
    }

    let (status, body) = via(name, None, TOKEN).send(&state).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "the local token stays on this PC"
    );
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("solo vale en el propio equipo")
    );
    let local = Call::new("GET", "/api/v1/jobs");
    assert_eq!(
        local.send(&state).await.0,
        StatusCode::OK,
        "and still works here"
    );
}

#[tokio::test]
async fn a_busy_node_refuses_new_requests_instead_of_piling_them_up() {
    let mut state = state("busy");
    state.in_flight = Arc::new(tokio::sync::Semaphore::new(0));
    let mut call = Call::new("GET", "/api/v1/health");
    call.token = None;
    let (status, body) = call.send(&state).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], "too_many_requests");
}

#[tokio::test]
async fn the_audit_log_keeps_who_did_what_but_no_secrets() {
    let state = state("audit");
    let (_, offer) = Call::new("POST", "/api/v1/devices/pairing")
        .send(&state)
        .await;
    let code = offer["code"].as_str().unwrap().to_owned();
    let mut pair =
        Call::new("POST", "/api/v1/pair").with_body(json!({ "code": code, "name": "Mi tablet" }));
    pair.token = None;
    let (_, linked) = pair.send(&state).await;
    let (id, token) = (
        linked["device"]["id"].as_str().unwrap().to_owned(),
        linked["token"].as_str().unwrap().to_owned(),
    );
    let mut bad = Call::new("POST", "/api/v1/pair")
        .with_body(json!({ "code": "ZZZZZ-ZZZZZ", "name": "Intruso" }));
    bad.token = None;
    bad.send(&state).await;
    Call::new("DELETE", format!("/api/v1/devices/{id}"))
        .send(&state)
        .await;

    let text = std::fs::read_to_string(state.dir.join("audit.jsonl")).unwrap();
    let actions: Vec<String> = text
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).unwrap()["action"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        actions,
        [
            "pairing_offered",
            "device_paired",
            "pairing_failed",
            "device_revoked"
        ]
    );
    for secret in [token.as_str(), code.as_str(), TOKEN, "ZZZZZ-ZZZZZ"] {
        assert!(!text.contains(secret), "{secret} leaked into the audit log");
    }
    let log = std::fs::read_to_string(state.dir.join("node.log")).unwrap();
    for secret in [token.as_str(), code.as_str(), TOKEN, "ZZZZZ-ZZZZZ"] {
        assert!(!log.contains(secret), "{secret} leaked into node.log");
    }
}

/// Starts a real node on a free port; the closure runs with its port and a way to stop it.
fn with_real_node(name: &str, test: impl FnOnce(u16)) {
    let dir = std::env::temp_dir().join(format!("chxchx-node-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runner = JobRunner::attended(JobStore::open(&dir.join("jobs")).unwrap(), Done, 1);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
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
    while request(port, "GET", "/api/v1/health", None, None).is_err() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    test(port);
    request(port, "POST", "/api/v1/node/stop", Some(TOKEN), None).unwrap();
    runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(10), server).await })
        .expect("the node stops")
        .unwrap()
        .unwrap();
}

#[test]
fn an_open_event_stream_is_closed_when_its_device_is_revoked() {
    use crate::integrations::node_client::request;
    with_real_node("revoke-ws", |port| {
        let offer: Value = serde_json::from_str(
            &request(port, "POST", "/api/v1/devices/pairing", Some(TOKEN), None)
                .unwrap()
                .body,
        )
        .unwrap();
        let linked: Value = serde_json::from_str(
            &request(
                port,
                "POST",
                "/api/v1/pair",
                None,
                Some(&json!({ "code": offer["code"], "name": "Pixel" }).to_string()),
            )
            .unwrap()
            .body,
        )
        .unwrap();
        let (id, token) = (
            linked["device"]["id"].as_str().unwrap().to_owned(),
            linked["token"].as_str().unwrap().to_owned(),
        );
        let mut socket = ws::open(
            port,
            "/api/v1/events",
            &format!("chxchx.v1, bearer.{token}"),
        )
        .unwrap();
        let revoked = request(
            port,
            "DELETE",
            &format!("/api/v1/devices/{id}"),
            Some(TOKEN),
            None,
        )
        .unwrap();
        assert_eq!(revoked.status, 200);

        socket.wait(Duration::from_secs(8));
        let started = std::time::Instant::now();
        assert!(socket.text().is_none());
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the server closed the stream instead of the read timing out"
        );
        let again = ws::open(
            port,
            "/api/v1/events",
            &format!("chxchx.v1, bearer.{token}"),
        );
        assert!(
            again.err().unwrap().contains("401"),
            "and it cannot reconnect"
        );
    });
}

#[test]
fn only_so_many_event_streams_can_be_open_at_once() {
    with_real_node("sockets", |port| {
        let bearer = format!("chxchx.v1, bearer.{TOKEN}");
        let open: Vec<_> = (0..16)
            .map(|_| ws::open(port, "/api/v1/events", &bearer).unwrap())
            .collect();
        let refused = ws::open(port, "/api/v1/events", &bearer);
        assert!(refused.err().unwrap().contains("429"));
        drop(open);
    });
}
