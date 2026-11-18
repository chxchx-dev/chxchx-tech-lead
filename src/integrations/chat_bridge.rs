//! The visual chat's connection to `chxchx-tech agent chat`.
//!
//! The Python CLI still runs provider turns and owns provider permissions and context release.
//! Rust `chat_state` owns the visible transcript. This module finds a compatible CLI, runs a turn
//! and answers its questions; it knows nothing about windows.
pub use crate::integrations::agent_catalog::{AgentOptions, ModelOption};
use crate::integrations::executable_search::{agent_path, find_executable, find_in};
use serde_json::Value;
use std::collections::VecDeque;
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
const REQUIRED_CHAT_OPTIONS: [&str; 3] =
    ["--live", "--resume-decided-by-rust", "--resume-recap-file"];

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
    process.args(["agent", "chat", "--help"]);
    matches!(run_short(process, Duration::from_secs(20)), Ok((0, text)) if REQUIRED_CHAT_OPTIONS.iter().all(|option| text.contains(option)))
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
    pub project: PathBuf,
    pub prompt: String,
    /// Recap built by Rust when a restored provider session no longer exists.
    pub resume_recap: String,
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
    prompt_file: PathBuf,
    resume_recap_file: Option<PathBuf>,
    pub exit: Option<i32>,
}

impl Turn {
    pub fn start(cli: &Path, spec: &TurnSpec) -> Result<Self, String> {
        // The prompt travels in a file: a command line has length limits and quoting rules.
        let prompt_file = temporary_chat_file("prompt");
        if let Err(error) = std::fs::write(&prompt_file, &spec.prompt) {
            remove_turn_files(&prompt_file, None);
            return Err(error.to_string());
        }
        let resume_recap_file = if spec.resume_recap.is_empty() {
            None
        } else {
            let path = temporary_chat_file("recap");
            if let Err(error) = std::fs::write(&path, &spec.resume_recap) {
                remove_turn_files(&prompt_file, Some(&path));
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
            ]);
        if let Some(path) = &resume_recap_file {
            process.arg("--resume-recap-file").arg(path);
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
        for (flag, value) in [
            ("--session", &spec.session),
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
            // Claude stream-json and non-live Codex JSONL are normalized by Rust at this boundary.
            .env("CHXCHX_RUST_AGENT_STREAM", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                remove_turn_files(&prompt_file, resume_recap_file.as_deref());
                format!("No pude iniciar el CLI: {error}")
            })?;
        let stdin = child.stdin.take();
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            remove_turn_files(&prompt_file, resume_recap_file.as_deref());
            return Err("sin salida".into());
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill();
            let _ = child.wait();
            remove_turn_files(&prompt_file, resume_recap_file.as_deref());
            return Err("sin errores".into());
        };
        let (sender, events) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).split(b'\n').map_while(Result::ok) {
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    if value["type"] == "provider_record" {
                        let provider = value["provider"].as_str().unwrap_or_default();
                        let record = &value["record"];
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
            prompt_file,
            resume_recap_file,
            exit: None,
        })
    }

    /// Events printed since the last call.
    pub fn poll(&mut self) -> Vec<Value> {
        let mut found: Vec<Value> = self.events.try_iter().collect();
        if self.exit.is_none()
            && let Some(child) = self.child.as_mut()
            && let Ok(Some(status)) = child.try_wait()
        {
            self.exit = Some(status.code().unwrap_or(-1));
            // Whatever was printed right before exiting.
            found.extend(drain_after_exit(&self.events));
        }
        found
    }

    pub fn is_running(&self) -> bool {
        self.exit.is_none()
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

impl Drop for Turn {
    fn drop(&mut self) {
        remove_turn_files(&self.prompt_file, self.resume_recap_file.as_deref());
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

fn remove_turn_files(prompt: &Path, recap: Option<&Path>) {
    let _ = std::fs::remove_file(prompt);
    if let Some(recap) = recap {
        let _ = std::fs::remove_file(recap);
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
