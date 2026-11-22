//! The visual chat's connection to the agents.
//!
//! Claude Code turns (and its hidden handoff summary) run directly from Rust: see
//! `claude_runner`. Codex and OpenCode still go through `chxchx-tech agent chat` of the Python
//! CLI, which this module finds, probes and drives. Either way Rust owns the visible transcript
//! and Claude's live control responses.
pub use crate::integrations::agent_catalog::{AgentOptions, ModelOption};
mod claude_runner;
mod node_runner;
mod service_runner;
use crate::integrations::executable_search::agent_path;
pub use claude_runner::{missing_runner_hint, prepare_native, runs_natively};
pub use node_runner::delegable;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What a finished process printed last. The process can be gone before the thread that reads
/// its output has passed the final lines on, so this waits for that thread to end (its sender
/// drops at end of output), for a second at most in case something else keeps the pipe open.
pub(super) fn drain_after_exit(events: &Receiver<Value>) -> Vec<Value> {
    let mut found = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match events.recv_timeout(Duration::from_millis(20)) {
            Ok(event) => found.push(event),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if Instant::now() >= deadline {
                    break;
                }
            }
        }
    }
    found
}

/// Runs a short command and returns its stdout, giving up after `limit`.
pub(super) fn run_short(mut process: Command, limit: Duration) -> Result<(i32, String), String> {
    let mut child = process
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut stdout = child.stdout.take().ok_or("sin salida")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let deadline = Instant::now() + limit;
    let code = loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => break status.code().unwrap_or(-1),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("El CLI tardó demasiado en responder".to_owned());
            }
            None => std::thread::sleep(Duration::from_millis(15)),
        }
    };
    let bytes = reader.join().unwrap_or_default();
    Ok((code, String::from_utf8_lossy(&bytes).into_owned()))
}

/// Everything one chat turn needs; the same options `agent chat` takes.
#[derive(Clone, Debug, Default)]
pub struct TurnSpec {
    pub agent: String,
    pub provider: String,
    pub project: PathBuf,
    pub prompt: String,
    /// Recap built by Rust when a restored provider session no longer exists.
    pub resume_recap: String,
    /// Recent provider memory requests recovered before opening a new Studio session.
    pub recovered_memories: String,
    pub session: Option<String>,
    /// `read-only`, `workspace-write`, `workspace-commands` or `full-access`.
    pub mode: String,
    pub network: bool,
    /// Rules already approved (`--allow`).
    pub allow: Vec<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// Images attached to this turn, already prepared (`--image`).
    pub images: Vec<PathBuf>,
    /// The agent as configured (program and arguments) and where it was found, for providers
    /// Rust runs itself.
    pub agent_command: Vec<String>,
    pub executable: Option<PathBuf>,
    pub agent_cwd: PathBuf,
    /// The caller verified local trust for this project; nothing runs without it.
    pub trusted: bool,
}

pub struct Turn {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    events: Receiver<Value>,
    stderr: Arc<Mutex<VecDeque<String>>>,
    provider: String,
    kind: TurnKind,
    handoff_text: String,
    consume_handoff_session: Option<String>,
    context_tokens: Option<i64>,
    context_window: Option<i64>,
    cancelled: bool,
    pending_claude: Arc<Mutex<HashMap<String, Value>>>,
    pub exit: Option<i32>,
    /// Events produced by Rust itself before the provider printed anything.
    pre_events: Vec<Value>,
    /// Present when Rust runs the provider directly.
    native: Option<claude_runner::NativeRun>,
    /// Present when the local node runs the turn and Studio only follows it.
    remote: Option<node_runner::RemoteRun>,
}

#[derive(Clone, Debug)]
enum TurnKind {
    Chat,
    Handoff { session_id: String },
}

/// Reads a provider's output on its own thread and forwards what Studio shows.
///
/// Without `wrap`, each line is already a `chxchx.agent-event` (what the Python CLI prints);
/// with a provider name, each line is that provider's raw record, as a process run by Rust prints.
/// Claude's control requests are remembered here so answers can be built from the original input.
fn spawn_event_reader(
    stdout: impl Read + Send + 'static,
    sender: std::sync::mpsc::Sender<Value>,
    pending_claude: Arc<Mutex<HashMap<String, Value>>>,
    wrap: Option<&'static str>,
) {
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).split(b'\n').map_while(Result::ok) {
            let Ok(mut value) = serde_json::from_slice::<Value>(&line) else {
                continue;
            };
            if let Some(provider) = wrap {
                value = serde_json::json!({
                    "type": "provider_record", "provider": provider, "record": value
                });
            }
            if value["type"] == "provider_record" {
                let provider = value["provider"].as_str().unwrap_or_default();
                let record = &value["record"];
                if provider == "Claude"
                    && let Some(request_id) = record["request_id"].as_str()
                {
                    match record["type"].as_str() {
                        Some("control_request") => {
                            if let Ok(mut pending) = pending_claude.lock() {
                                pending.insert(request_id.to_owned(), record.clone());
                            }
                            if !request_id.is_empty()
                                && record["request"]["subtype"].as_str() != Some("can_use_tool")
                            {
                                let internal = serde_json::json!({
                                    "type":"chxchx_internal_claude_control",
                                    "request_id":request_id
                                });
                                if sender.send(internal).is_err() {
                                    return;
                                }
                                super::wake::notify();
                                continue;
                            }
                        }
                        Some("control_cancel_request") => {
                            if let Ok(mut pending) = pending_claude.lock() {
                                pending.remove(request_id);
                            }
                        }
                        _ => {}
                    }
                }
                if provider == "Claude"
                    && record["type"].as_str() == Some("result")
                    && let Ok(mut pending) = pending_claude.lock()
                {
                    pending.clear();
                }
                for event in super::agent_stream::normalize(provider, record) {
                    if sender.send(event).is_err() {
                        return;
                    }
                    super::wake::notify();
                }
                // The turn is over: the host closes the input so the provider can exit.
                if wrap.is_some() && record["type"].as_str() == Some("result") {
                    let _ = sender.send(serde_json::json!({"type": INTERNAL_TURN_RESULT}));
                    super::wake::notify();
                }
                continue;
            }
            if sender.send(value).is_err() {
                break;
            }
            super::wake::notify();
        }
        super::wake::notify_after_end();
    });
}

/// Keeps the last lines a provider wrote to stderr, for the message when it fails.
fn spawn_stderr_reader(stderr: impl Read + Send + 'static, tail: Arc<Mutex<VecDeque<String>>>) {
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
            if let Ok(mut lines) = tail.lock() {
                if lines.len() == 20 {
                    lines.pop_front();
                }
                lines.push_back(String::from_utf8_lossy(&line).into_owned());
            }
        }
    });
}

/// Printed by the reader when a natively run turn delivered its result.
const INTERNAL_TURN_RESULT: &str = "chxchx_internal_turn_result";

impl Turn {
    /// Events printed since the last call.
    pub fn poll(&mut self) -> Vec<Value> {
        if let Some(remote) = &self.remote {
            // Whether the job is over is read first, then the events: whatever was sent before
            // it finished is already in the channel by then.
            let over = remote.finished();
            let found: Vec<Value> = self.events.try_iter().collect();
            if self.exit.is_none()
                && let Some((code, cancelled)) = over
            {
                self.exit = Some(code);
                self.cancelled |= cancelled;
            }
            return found;
        }
        let mut found = std::mem::take(&mut self.pre_events);
        found.extend(self.consume_internal_events(self.events.try_iter().collect()));
        if self.exit.is_none()
            && let Some(child) = self.child.as_mut()
            && let Ok(Some(status)) = child.try_wait()
        {
            self.exit = Some(status.code().unwrap_or(-1));
            // Whatever was printed right before exiting.
            found.extend(self.consume_internal_events(drain_after_exit(&self.events)));
            let closing = self.closing_events();
            found.extend(self.consume_internal_events(closing));
        }
        found
    }

    pub fn is_running(&self) -> bool {
        self.exit.is_none()
    }

    pub fn is_handoff(&self) -> bool {
        matches!(self.kind, TurnKind::Handoff { .. })
    }

    pub fn handoff_target_session(&self) -> Option<&str> {
        match &self.kind {
            TurnKind::Handoff { session_id } => Some(session_id),
            TurnKind::Chat => None,
        }
    }

    pub fn take_handoff_text(&mut self) -> String {
        std::mem::take(&mut self.handoff_text)
    }

    pub fn take_consumed_handoff_session(&mut self) -> Option<String> {
        self.consume_handoff_session.take()
    }

    pub fn was_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn needs_context_release(&self) -> bool {
        crate::features::chat_handoff::needs_release(self.context_tokens, self.context_window)
    }

    fn write_line(&mut self, line: &str) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("El turno ya terminó")?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .and_then(|()| stdin.flush())
            .map_err(|error| error.to_string())
    }

    /// Answers a live permission or question: `{"type": "answer", "request_id": …, …}`.
    pub fn answer(&mut self, answer: &Value) -> Result<(), String> {
        if let Some(remote) = &self.remote {
            return remote.answer(answer);
        }
        if self.provider == "Claude" && answer["type"].as_str() == Some("answer") {
            let request_id = answer["request_id"]
                .as_str()
                .filter(|request_id| !request_id.is_empty())
                .ok_or_else(|| "La respuesta de Claude no contiene request_id".to_owned())?;
            let original = self
                .pending_claude
                .lock()
                .map_err(|_| "No se pudo leer la solicitud pendiente de Claude".to_owned())?
                .remove(request_id)
                .ok_or_else(|| "La solicitud de Claude ya no está pendiente".to_owned())?;
            let response = super::agent_live::claude_response(
                request_id,
                &original["request"]["input"],
                answer,
            );
            return self.write_line(&response.to_string());
        }
        self.write_line(&answer.to_string())
    }

    pub fn cancel(&mut self) {
        if let Some(remote) = &self.remote {
            self.cancelled = true;
            remote.cancel();
            return;
        }
        if self.stop_native() {
            return;
        }
        let _ = self.write_line("cancel");
    }

    pub fn stderr_tail(&self) -> String {
        self.stderr
            .lock()
            .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_default()
    }
}

impl Turn {
    fn consume_internal_events(&mut self, events: Vec<Value>) -> Vec<Value> {
        let mut visible = Vec::with_capacity(events.len());
        for event in events {
            if event["type"].as_str() == Some(INTERNAL_TURN_RESULT) {
                self.stdin = None; // closing the input lets the provider exit
                continue;
            }
            let Some(event) = self.translate_service_event(event) else {
                continue;
            };
            self.observe_native(&event);
            if self.is_handoff() {
                if event["type"].as_str() == Some("handoff")
                    && event["schema"].as_str() == Some("chxchx.agent-event")
                    && event["schema_version"].as_i64() == Some(1)
                {
                    self.handoff_text = event["text"]
                        .as_str()
                        .unwrap_or_default()
                        .chars()
                        .take(crate::features::chat_handoff::MAX_HANDOFF_CHARS)
                        .collect();
                }
                continue;
            }
            match event["type"].as_str() {
                Some("handoff") => {
                    continue;
                }
                Some("limits") => {
                    if let Some(tokens) = event["context_tokens"].as_i64().filter(|n| *n > 0) {
                        self.context_tokens = Some(tokens);
                    }
                    if let Some(window) = event["context_window"].as_i64().filter(|n| *n > 0) {
                        self.context_window = Some(window);
                    }
                }
                Some("done") => {
                    self.cancelled = event["cancelled"].as_bool().unwrap_or(false);
                }
                _ => {}
            }
            if event["type"].as_str() != Some("chxchx_internal_claude_control") {
                visible.push(event);
                continue;
            }
            let Some(request_id) = event["request_id"].as_str() else {
                continue;
            };
            if let Ok(mut pending) = self.pending_claude.lock() {
                pending.remove(request_id);
            }
            let response = super::agent_live::claude_unsupported_response(request_id);
            if let Err(error) = self.write_line(&response.to_string()) {
                visible.push(serde_json::json!({
                    "schema":"chxchx.agent-event", "schema_version":1,
                    "type":"notice", "error":true,
                    "message":format!("No se pudo responder a Claude: {error}")
                }));
            }
        }
        visible
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        if self.exit.is_some() {
            return;
        }
        if self.remote.is_some() {
            // The node runs this turn. Closing the window only stops following it: the job goes on
            // and a new Studio picks it up again.
            return;
        }
        self.cancel();
        self.stdin = None; // EOF also stops the agent
        // Give the CLI time to stop the agent cleanly; never leave it running.
        if let Some(mut child) = self.child.take() {
            std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(8);
                while Instant::now() < deadline {
                    if matches!(child.try_wait(), Ok(Some(_))) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                let _ = child.kill();
                let _ = child.wait();
            });
        }
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// The conversation a chat saved the last time (`agent chat-state`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SavedChat {
    pub session_id: String,
    pub resume_recap: String,
    /// `("user" | "assistant", text)`
    pub messages: Vec<(String, String)>,
}

pub fn parse_saved_chat(output: &str) -> Result<SavedChat, String> {
    let value: Value = serde_json::from_str(output.trim()).map_err(|error| error.to_string())?;
    if value.get("schema").and_then(Value::as_str) != Some("chxchx.chat-state") {
        return Err("El CLI no conoce el estado del chat".to_owned());
    }
    Ok(SavedChat {
        session_id: text(&value, "session_id"),
        resume_recap: String::new(),
        messages: value
            .get("messages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|message| {
                let role = text(message, "role");
                let body = text(message, "text");
                (matches!(role.as_str(), "user" | "assistant") && !body.is_empty())
                    .then_some((role, body))
            })
            .collect(),
    })
}

/// Reads the saved conversation and marks its session to be checked by the next turn.
pub fn restore_chat(project: &Path, agent: &str) -> SavedChat {
    let state = crate::features::chat_state::payload(project, agent, true);
    SavedChat {
        session_id: state.session_id,
        resume_recap: String::new(),
        messages: state
            .messages
            .into_iter()
            .map(|message| (message.role, message.text))
            .collect(),
    }
}

pub fn restore_chat_for_provider(
    project: &Path,
    agent: &str,
    provider: &str,
    executable: &str,
) -> SavedChat {
    let state = crate::features::chat_state::payload(project, agent, true);
    let (session_id, resume_recap) = super::chat_resume::resolve_resume(
        project,
        provider,
        agent,
        (!state.session_id.is_empty()).then_some(state.session_id.as_str()),
        executable,
    );
    SavedChat {
        session_id: session_id.unwrap_or_default(),
        resume_recap,
        messages: state
            .messages
            .into_iter()
            .map(|message| (message.role, message.text))
            .collect(),
    }
}

/// A new chat starts from nothing.
pub fn reset_chat(project: &Path, agent: &str) {
    crate::features::chat_state::reset(project, agent);
}

#[cfg(all(test, unix))]
#[path = "chat_bridge_tests.rs"]
mod tests;
