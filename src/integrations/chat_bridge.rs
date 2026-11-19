//! The visual chat's connection to `chxchx-tech agent chat`.
//!
//! The Python CLI still runs provider turns. Rust owns the visible transcript and Claude's live
//! control responses while this module finds a compatible CLI, runs a turn and answers its cards.
pub use crate::integrations::agent_catalog::{AgentOptions, ModelOption};
use crate::integrations::executable_search::{agent_path, find_executable, find_in};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Variable that points Studio at the CLI to use (the same one the legacy Studio reads).
pub const CLI_ENV: &str = "CHXCHX_TECH_CLI";

/// What a CLI must print for `agent chat --help` to be the one that speaks the chat protocol.
const REQUIRED_CHAT_OPTIONS: [&str; 6] = [
    "--live",
    "--resume-decided-by-rust",
    "--resume-recap-file",
    "--rust-context-file",
    "--rust-chat-handoff",
    "--no-auto-release",
];

pub(super) fn command(cli: &Path) -> Command {
    let mut command = Command::new(cli);
    // Python writes the console's code page to a pipe on Windows; the protocol is UTF-8.
    command
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1");
    command.env("PATH", agent_path());
    command.envs(super::process_memory::LEAN_ENV);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

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

/// Whether this CLI is the one that runs chat turns (the Rust CLI, also named `chxchx-tech`, is not).
pub fn supports_chat(cli: &Path) -> bool {
    let mut process = command(cli);
    // Typer/Rich shortens long options in narrow piped help output, which made a compatible CLI
    // look too old to Studio. The probe needs the complete option names, not a terminal layout.
    process.env("COLUMNS", "200");
    process.args(["agent", "chat", "--help"]);
    let chat = matches!(run_short(process, Duration::from_secs(20)), Ok((0, text)) if REQUIRED_CHAT_OPTIONS.iter().all(|option| text.contains(option)));
    if !chat {
        return false;
    }
    let mut handoff = command(cli);
    handoff.args(["agent", "chat-handoff", "--help"]);
    matches!(run_short(handoff, Duration::from_secs(20)), Ok((0, text)) if text.contains("--session") && text.contains("--prompt-file"))
}

fn candidates(project: &Path, path: &OsString) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    if let Some(explicit) = std::env::var_os(CLI_ENV) {
        found.push(PathBuf::from(explicit));
    }
    if let Some(on_path) = find_in("chxchx-tech", path) {
        found.push(on_path);
    }
    // A checkout's own environment, next to the project or above the running binary.
    let venv = |root: &Path| {
        let folder = if cfg!(windows) { "Scripts" } else { "bin" };
        root.join(".venv").join(folder).join(if cfg!(windows) {
            "chxchx-tech.exe"
        } else {
            "chxchx-tech"
        })
    };
    // The Python CLI that still serves the chat lives in `old-version/` until it is ported.
    let old = |root: &Path| venv(&root.join("old-version"));
    found.push(venv(project));
    found.push(old(project));
    if let Ok(exe) = std::env::current_exe() {
        for root in exe.ancestors().skip(1).take(6) {
            found.push(venv(root));
            found.push(old(root));
        }
    }
    // Never ourselves: `chxchx-tech` on the PATH may be this very program, which hands the
    // legacy commands to the CLI this function is looking for.
    let own = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.canonicalize().ok());
    let mut unique: Vec<PathBuf> = Vec::new();
    for candidate in found {
        let is_own = own
            .as_ref()
            .is_some_and(|own| candidate.canonicalize().ok().as_ref() == Some(own));
        if candidate.is_file() && !is_own && !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

/// The legacy CLI that serves the commands not ported yet, without probing it.
pub fn legacy_cli(project: &Path) -> Option<PathBuf> {
    let found = candidates(project, &agent_path());
    // The one the installer names, then an environment of its own (`.venv`), and only then
    // whatever `chxchx-tech` the PATH offers, which may be a shim that leads back here.
    let explicit = std::env::var_os(CLI_ENV).map(PathBuf::from);
    explicit
        .filter(|path| found.contains(path))
        .or_else(|| {
            found
                .iter()
                .find(|c| c.components().any(|part| part.as_os_str() == ".venv"))
                .cloned()
        })
        .or_else(|| found.into_iter().next())
}

/// The first CLI that speaks the chat protocol, or `None`.
pub fn find_cli(project: &Path) -> Option<PathBuf> {
    candidates(project, &agent_path())
        .into_iter()
        .find(|cli| supports_chat(cli))
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
    prompt_file: PathBuf,
    resume_recap_file: Option<PathBuf>,
    rust_context_file: Option<PathBuf>,
    pub exit: Option<i32>,
}

#[derive(Clone, Debug)]
enum TurnKind {
    Chat,
    Handoff { session_id: String },
}

impl Turn {
    pub fn start(cli: &Path, spec: &TurnSpec) -> Result<Self, String> {
        // The prompt travels in a file: a command line has length limits and quoting rules.
        let prompt_file = temporary_chat_file("prompt");
        if let Err(error) = std::fs::write(&prompt_file, &spec.prompt) {
            remove_turn_files(&prompt_file, None, None);
            return Err(error.to_string());
        }
        // Studio prepares context only for a new provider session. If compaction cannot be done,
        // leave the option out so the Python adapter can use its existing fallback path.
        let pending_handoff = spec.session.as_deref().and_then(|session| {
            crate::features::chat_handoff::load(&spec.project, &spec.provider, session)
                .map(|text| (session.to_owned(), text))
        });
        let effective_session = if pending_handoff.is_some() {
            None
        } else {
            spec.session
                .as_deref()
                .filter(|session| !session.is_empty())
        };
        let needs_new_context = effective_session.is_none();
        let rust_context_file = if needs_new_context {
            match crate::features::context_compaction::compact_context(&spec.project) {
                Ok(result) => {
                    let compacted_sections = result.moved_count();
                    let path = temporary_chat_file("context");
                    let context = super::chat_context::initial_context_with_memories(
                        &spec.project,
                        &spec.provider,
                        &spec.recovered_memories,
                    );
                    let payload = serde_json::json!({
                        "schema": "chxchx.studio-chat-context",
                        "schema_version": 1,
                        "context": context,
                        "compacted_sections": compacted_sections,
                    });
                    match std::fs::write(&path, payload.to_string()) {
                        Ok(()) => Some(path),
                        Err(_) => {
                            let _ = std::fs::remove_file(path);
                            None
                        }
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };
        let resume_recap = pending_handoff
            .as_ref()
            .map(|(_, text)| super::chat_context::handoff_block(text))
            .unwrap_or_else(|| spec.resume_recap.clone());
        let resume_recap_file = if resume_recap.is_empty() {
            None
        } else {
            let path = temporary_chat_file("recap");
            if let Err(error) = std::fs::write(&path, &resume_recap) {
                remove_turn_files(&prompt_file, Some(&path), rust_context_file.as_deref());
                return Err(error.to_string());
            }
            Some(path)
        };
        let mut process = command(cli);
        process
            .args(["agent", "chat", &spec.agent, "--path"])
            .arg(&spec.project)
            .arg("--prompt-file")
            .arg(&prompt_file)
            .args([
                "--cancel-on-stdin",
                "--live",
                "--mode",
                &spec.mode,
                "--resume-decided-by-rust",
                "--rust-chat-handoff",
                "--no-auto-release",
            ]);
        if let Some(path) = &resume_recap_file {
            process.arg("--resume-recap-file").arg(path);
        }
        if let Some(path) = &rust_context_file {
            process.arg("--rust-context-file").arg(path);
        }
        if spec.network {
            process.arg("--network");
        }
        for image in &spec.images {
            process.arg("--image").arg(image);
        }
        for rule in &spec.allow {
            process.args(["--allow", rule]);
        }
        let effective_session = effective_session.map(str::to_owned);
        for (flag, value) in [
            ("--session", &effective_session),
            ("--model", &spec.model),
            ("--effort", &spec.effort),
        ] {
            if let Some(value) = value.as_deref().filter(|v| !v.is_empty()) {
                process.args([flag, value]);
            }
        }
        let mut child = process
            .current_dir(&spec.project)
            // Rust Studio owns the visible transcript; the Python adapter only runs the provider.
            .env("CHXCHX_RUST_CHAT_STATE", "1")
            // Rust also prepares and records the Studio session journal.
            .env("CHXCHX_RUST_CHAT_JOURNAL", "1")
            // Rust recovers and persists explicit project-memory requests for Studio.
            .env("CHXCHX_RUST_EXPLICIT_MEMORY", "1")
            // Claude stream-json and non-live Codex JSONL are normalized by Rust at this boundary.
            .env("CHXCHX_RUST_AGENT_STREAM", "1")
            // Studio owns handoff persistence and release thresholds.
            .env("CHXCHX_RUST_CHAT_HANDOFF", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                remove_turn_files(
                    &prompt_file,
                    resume_recap_file.as_deref(),
                    rust_context_file.as_deref(),
                );
                format!("No pude iniciar el CLI: {error}")
            })?;
        let stdin = child.stdin.take();
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            remove_turn_files(
                &prompt_file,
                resume_recap_file.as_deref(),
                rust_context_file.as_deref(),
            );
            return Err("sin salida".into());
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill();
            let _ = child.wait();
            remove_turn_files(
                &prompt_file,
                resume_recap_file.as_deref(),
                rust_context_file.as_deref(),
            );
            return Err("sin errores".into());
        };
        let (sender, events) = channel();
        let pending_claude = Arc::new(Mutex::new(HashMap::new()));
        let reader_pending_claude = pending_claude.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).split(b'\n').map_while(Result::ok) {
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    if value["type"] == "provider_record" {
                        let provider = value["provider"].as_str().unwrap_or_default();
                        let record = &value["record"];
                        if provider == "Claude"
                            && let Some(request_id) = record["request_id"].as_str()
                        {
                            match record["type"].as_str() {
                                Some("control_request") => {
                                    if let Ok(mut pending) = reader_pending_claude.lock() {
                                        pending.insert(request_id.to_owned(), record.clone());
                                    }
                                    if !request_id.is_empty()
                                        && record["request"]["subtype"].as_str()
                                            != Some("can_use_tool")
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
                                    if let Ok(mut pending) = reader_pending_claude.lock() {
                                        pending.remove(request_id);
                                    }
                                }
                                Some("result") => {
                                    if let Ok(mut pending) = reader_pending_claude.lock() {
                                        pending.clear();
                                    }
                                }
                                _ => {}
                            }
                        }
                        for event in super::agent_stream::normalize(provider, record) {
                            if sender.send(event).is_err() {
                                return;
                            }
                            super::wake::notify();
                        }
                        continue;
                    }
                    if sender.send(value).is_err() {
                        break;
                    }
                    super::wake::notify();
                }
            }
            super::wake::notify_after_end();
        });
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let collector = tail.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
                if let Ok(mut lines) = collector.lock() {
                    if lines.len() == 20 {
                        lines.pop_front();
                    }
                    lines.push_back(String::from_utf8_lossy(&line).into_owned());
                }
            }
        });
        Ok(Self {
            child: Some(child),
            stdin,
            events,
            stderr: tail,
            provider: spec.provider.clone(),
            kind: TurnKind::Chat,
            handoff_text: String::new(),
            consume_handoff_session: pending_handoff.map(|(session, _)| session),
            context_tokens: None,
            context_window: None,
            cancelled: false,
            pending_claude,
            prompt_file,
            resume_recap_file,
            rust_context_file,
            exit: None,
        })
    }

    /// Starts the hidden, read-only provider turn that summarizes a nearly full session.
    pub fn start_handoff(cli: &Path, spec: &TurnSpec) -> Result<Self, String> {
        let session = spec
            .session
            .as_deref()
            .filter(|session| !session.is_empty())
            .ok_or("El handoff requiere una sesión activa")?;
        let prompt_file = temporary_chat_file("handoff");
        if let Err(error) = std::fs::write(&prompt_file, crate::features::chat_handoff::PROMPT) {
            let _ = std::fs::remove_file(&prompt_file);
            return Err(error.to_string());
        }
        let mut process = command(cli);
        process
            .args(["agent", "chat-handoff", &spec.agent, "--path"])
            .arg(&spec.project)
            .args(["--prompt-file"])
            .arg(&prompt_file)
            .arg("--cancel-on-stdin")
            .args(["--session", session]);
        for (flag, value) in [("--model", &spec.model), ("--effort", &spec.effort)] {
            if let Some(value) = value.as_deref().filter(|value| !value.is_empty()) {
                process.args([flag, value]);
            }
        }
        let mut child = process
            .current_dir(&spec.project)
            .env("CHXCHX_RUST_CHAT_HANDOFF", "1")
            // The adapter must normalize provider output so its internal command can capture text.
            .env("CHXCHX_RUST_AGENT_STREAM", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                let _ = std::fs::remove_file(&prompt_file);
                format!("No pude iniciar el handoff del chat: {error}")
            })?;
        let stdin = child.stdin.take();
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&prompt_file);
            return Err("sin salida para el handoff".into());
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&prompt_file);
            return Err("sin errores para el handoff".into());
        };
        let (sender, events) = channel();
        std::thread::spawn(move || {
            let bounded =
                stdout.take((crate::features::chat_handoff::MAX_HANDOFF_CHARS * 4 + 4096) as u64);
            for line in BufReader::new(bounded).split(b'\n').map_while(Result::ok) {
                if let Ok(value) = serde_json::from_slice::<Value>(&line)
                    && sender.send(value).is_err()
                {
                    return;
                }
            }
            super::wake::notify_after_end();
        });
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let collector = tail.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
                if let Ok(mut lines) = collector.lock() {
                    if lines.len() == 20 {
                        lines.pop_front();
                    }
                    lines.push_back(String::from_utf8_lossy(&line).into_owned());
                }
            }
        });
        Ok(Self {
            child: Some(child),
            stdin,
            events,
            stderr: tail,
            provider: spec.provider.clone(),
            kind: TurnKind::Handoff {
                session_id: session.to_owned(),
            },
            handoff_text: String::new(),
            consume_handoff_session: None,
            context_tokens: None,
            context_window: None,
            cancelled: false,
            pending_claude: Arc::new(Mutex::new(HashMap::new())),
            prompt_file,
            resume_recap_file: None,
            rust_context_file: None,
            exit: None,
        })
    }

    /// Events printed since the last call.
    pub fn poll(&mut self) -> Vec<Value> {
        let mut found = self.consume_internal_events(self.events.try_iter().collect());
        if self.exit.is_none()
            && let Some(child) = self.child.as_mut()
            && let Ok(Some(status)) = child.try_wait()
        {
            self.exit = Some(status.code().unwrap_or(-1));
            // Whatever was printed right before exiting.
            found.extend(self.consume_internal_events(drain_after_exit(&self.events)));
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
        remove_turn_files(
            &self.prompt_file,
            self.resume_recap_file.as_deref(),
            self.rust_context_file.as_deref(),
        );
        if self.exit.is_some() {
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

fn remove_turn_files(prompt: &Path, recap: Option<&Path>, context: Option<&Path>) {
    for file in [Some(prompt), recap, context].into_iter().flatten() {
        let _ = std::fs::remove_file(file);
    }
}

fn temporary_chat_file(kind: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "chxchx-chat-{kind}-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ))
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

/// Where Studio would look for the CLI, for the message when none is found.
pub fn missing_cli_hint() -> String {
    let path = find_executable("chxchx-tech")
        .map(|found| {
            format!(
                " (en el PATH está {}, pero no habla el protocolo del chat)",
                found.display()
            )
        })
        .unwrap_or_default();
    format!(
        "No encontré el CLI de agentes (chxchx-tech con `agent chat`){path}. Instala ChxChx Studio 0.5 o define {CLI_ENV} con su ruta."
    )
}

#[cfg(all(test, unix))]
#[path = "chat_bridge_tests.rs"]
mod tests;
