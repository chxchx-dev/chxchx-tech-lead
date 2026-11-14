//! The visual chat's connection to `chxchx-tech agent chat`.
//!
//! The legacy Python CLI is still the authority for agent turns (permissions, live approvals, context
//! release, memory, session persistence); Studio runs it as a child and speaks its JSON lines.
//! This module finds a CLI that understands the protocol, runs one turn and answers its questions.
//! It knows nothing about windows.
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
const PROTOCOL_MARK: &str = "--live";

fn command(cli: &Path) -> Command {
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

/// Runs a short command and returns its stdout, giving up after `limit`.
fn run_short(mut process: Command, limit: Duration) -> Result<(i32, String), String> {
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
    matches!(run_short(process, Duration::from_secs(20)), Ok((0, text)) if text.contains(PROTOCOL_MARK))
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
    let mut unique: Vec<PathBuf> = Vec::new();
    for candidate in found {
        if candidate.is_file() && !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
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
    pub exit: Option<i32>,
}

impl Turn {
    pub fn start(cli: &Path, spec: &TurnSpec) -> Result<Self, String> {
        // The prompt travels in a file: a command line has length limits and quoting rules.
        let prompt_file = std::env::temp_dir().join(format!(
            "chxchx-chat-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        std::fs::write(&prompt_file, &spec.prompt).map_err(|error| error.to_string())?;
        let mut process = command(cli);
        process
            .args(["agent", "chat", &spec.agent, "--path"])
            .arg(&spec.project)
            .arg("--prompt-file")
            .arg(&prompt_file)
            .args(["--cancel-on-stdin", "--live", "--mode", &spec.mode]);
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
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                let _ = std::fs::remove_file(&prompt_file);
                format!("No pude iniciar el CLI: {error}")
            })?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().ok_or("sin salida")?;
        let stderr = child.stderr.take().ok_or("sin errores")?;
        let (sender, events) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).split(b'\n').map_while(Result::ok) {
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
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
            found.extend(self.events.try_iter());
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
        let _ = std::fs::remove_file(&self.prompt_file);
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

/// A model the agent offers, with the effort levels it supports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelOption {
    pub id: String,
    pub label: String,
    pub efforts: Vec<String>,
    pub default_effort: String,
}

/// What the agent can do on this machine (`bridge agent-options`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentOptions {
    pub available: bool,
    pub install_hint: String,
    pub default_model: String,
    pub default_effort: String,
    pub models: Vec<ModelOption>,
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_owned))
        .collect()
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

pub fn parse_options(output: &str) -> Result<AgentOptions, String> {
    let value: Value = serde_json::from_str(output.trim())
        .map_err(|error| format!("Respuesta de opciones inválida: {error}"))?;
    if value.get("schema").and_then(Value::as_str) != Some("chxchx.agent-options") {
        return Err(text(&value, "error").if_empty("El CLI no devolvió las opciones del agente"));
    }
    Ok(AgentOptions {
        available: value
            .get("available")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        install_hint: text(&value, "install_hint"),
        default_model: text(&value, "default_model"),
        default_effort: text(&value, "default_effort"),
        models: value
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|model| ModelOption {
                id: text(model, "id"),
                label: text(model, "label").if_empty(&text(model, "id")),
                efforts: strings(model.get("efforts")),
                default_effort: text(model, "default_effort"),
            })
            .filter(|model| !model.id.is_empty())
            .collect(),
    })
}

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_owned()
        } else {
            self
        }
    }
}

/// Models and efforts of an agent. Blocking (it asks the agent's own CLI): run it off the UI thread.
pub fn agent_options(cli: &Path, project: &Path, agent: &str) -> Result<AgentOptions, String> {
    let mut process = command(cli);
    process
        .args(["bridge", "agent-options"])
        .arg(project)
        .args(["--agent", agent]);
    let (_, output) = run_short(process, Duration::from_secs(25))?;
    parse_options(&output)
}

/// The conversation a chat saved the last time (`agent chat-state`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SavedChat {
    pub session_id: String,
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
pub fn restore_chat(cli: &Path, project: &Path, agent: &str) -> Result<SavedChat, String> {
    let mut process = command(cli);
    process
        .args(["agent", "chat-state", agent, "--path"])
        .arg(project)
        .arg("--restore");
    let (code, output) = run_short(process, Duration::from_secs(20))?;
    if code != 0 {
        return Err("El CLI no pudo leer el chat guardado".to_owned());
    }
    parse_saved_chat(&output)
}

/// A new chat starts from nothing.
pub fn reset_chat(cli: &Path, project: &Path, agent: &str) {
    let mut process = command(cli);
    process
        .args(["agent", "chat-state", agent, "--path"])
        .arg(project)
        .arg("--reset");
    let _ = run_short(process, Duration::from_secs(20));
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
