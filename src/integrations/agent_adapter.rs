//! The adapter of the `agent-adapter` service (`services/agent-adapter/`): the process that drives
//! Codex and OpenCode in their live mode.
//!
//! Rust keeps everything that is policy: trust, configuration, the access level, the context and
//! the visible transcript. This module finds the service, checks that it speaks the contract
//! version Rust was written for, and turns one chat turn into the request that contract defines.
//! The service only runs the process Rust asked for and relays the provider's live protocol.
use crate::features::agents::Provider;
use crate::integrations::agent_access::{
    Access, codex_server_settings, opencode_agent, opencode_environment_for,
};
use crate::integrations::executable_search::{agent_path, find_in};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Variable that points Studio at the service, for installs and checkouts it cannot guess.
pub const ENV: &str = "CHXCHX_AGENT_ADAPTER";
/// The contract version this code speaks; a service that answers another one is not used.
pub const CONTRACT_VERSION: i64 = 1;
const PROGRAM: &str = "chxchx-agent-adapter";
pub const REQUEST_SCHEMA: &str = "chxchx.agent-adapter-request";

/// What `chxchx-agent-adapter contract` printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Description {
    pub version: String,
    pub providers: Vec<String>,
}

fn file_name() -> &'static str {
    if cfg!(windows) {
        "chxchx-agent-adapter.exe"
    } else {
        PROGRAM
    }
}

/// Where the service could be, most specific first: the variable, next to the running binary
/// (an installed product), a checkout's own environment, then the `PATH`.
pub fn candidates(
    path: &OsString,
    executable: Option<&Path>,
    project: Option<&Path>,
) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    if let Some(explicit) = std::env::var_os(ENV) {
        found.push(PathBuf::from(explicit));
    }
    if let Some(executable) = executable {
        for root in executable.ancestors().skip(1).take(6) {
            found.push(root.join(file_name()));
            found.push(root.join("libexec").join("chxchx").join(file_name()));
            found.push(checkout_environment(root));
        }
    }
    if let Some(project) = project {
        found.push(checkout_environment(project));
    }
    if let Some(on_path) = find_in(PROGRAM, path) {
        found.push(on_path);
    }
    let mut unique: Vec<PathBuf> = Vec::new();
    for candidate in found {
        if candidate.is_file() && !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

/// The service of a checkout: `services/agent-adapter/.venv/bin/chxchx-agent-adapter`.
fn checkout_environment(root: &Path) -> PathBuf {
    let folder = if cfg!(windows) { "Scripts" } else { "bin" };
    root.join("services")
        .join("agent-adapter")
        .join(".venv")
        .join(folder)
        .join(file_name())
}

/// Asks the service which contract it speaks. An older or newer one is an error, not a guess.
pub fn describe(service: &Path) -> Result<Description, String> {
    let mut process = Command::new(service);
    process.arg("contract").env("PATH", agent_path());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let (code, text) = super::chat_bridge::run_short(process, Duration::from_secs(20))
        .map_err(|error| format!("No pude consultar el servicio de agentes: {error}"))?;
    if code != 0 {
        return Err(format!(
            "El servicio de agentes respondió con el código {code}"
        ));
    }
    parse_description(&text)
}

pub fn parse_description(text: &str) -> Result<Description, String> {
    let value: Value = serde_json::from_str(text.trim())
        .map_err(|error| format!("El servicio de agentes no respondió JSON: {error}"))?;
    if value["schema"].as_str() != Some("chxchx.agent-adapter") {
        return Err("Ese programa no es el servicio de agentes de ChxChx".to_owned());
    }
    let version = value["contract_version"].as_i64();
    if version != Some(CONTRACT_VERSION) {
        return Err(format!(
            "El servicio de agentes habla el contrato {version:?}; esta versión de ChxChx usa el {CONTRACT_VERSION}"
        ));
    }
    Ok(Description {
        version: value["version"].as_str().unwrap_or_default().to_owned(),
        providers: value["providers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|provider| provider.as_str().map(str::to_owned))
            .collect(),
    })
}

fn located() -> &'static Mutex<Option<PathBuf>> {
    static LOCATED: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    LOCATED.get_or_init(|| Mutex::new(None))
}

/// The service to use, checked once. It runs the service, so call it off the UI thread the first time.
pub fn locate(project: Option<&Path>) -> Option<PathBuf> {
    if let Ok(known) = located().lock()
        && let Some(path) = known.as_ref()
    {
        return Some(path.clone());
    }
    let executable = std::env::current_exe().ok();
    let found = candidates(&agent_path(), executable.as_deref(), project)
        .into_iter()
        .find(|candidate| describe(candidate).is_ok_and(|d| !d.providers.is_empty()));
    if let (Some(path), Ok(mut known)) = (&found, located().lock()) {
        *known = Some(path.clone());
    }
    found
}

/// The service if it was already found, without looking again (safe on the UI thread).
pub fn known() -> Option<PathBuf> {
    located().lock().ok().and_then(|known| known.clone())
}

/// Major version of the installed OpenCode (1 or 2); `None` when it cannot be read.
pub fn opencode_major(executable: &Path) -> Option<u32> {
    static VERSIONS: OnceLock<Mutex<HashMap<PathBuf, Option<u32>>>> = OnceLock::new();
    let versions = VERSIONS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(known) = versions.lock()
        && let Some(major) = known.get(executable)
    {
        return *major;
    }
    let mut process = Command::new(executable);
    process.arg("--version").env("PATH", agent_path());
    let major = super::chat_bridge::run_short(process, Duration::from_secs(10))
        .ok()
        .and_then(|(_, text)| parse_major(&text));
    if let Ok(mut known) = versions.lock() {
        known.insert(executable.to_path_buf(), major);
    }
    major
}

/// The first number of `1.19.5`, `v2.2.6` or `opencode 2.0`.
pub fn parse_major(text: &str) -> Option<u32> {
    let digits = text.trim_start_matches(|c: char| !c.is_ascii_digit());
    let major: String = digits.chars().take_while(char::is_ascii_digit).collect();
    let rest = &digits[major.len()..];
    (rest.starts_with('.') && !major.is_empty())
        .then(|| major.parse().ok())
        .flatten()
}

/// What one turn needs, already decided by Rust.
pub struct TurnRequest<'a> {
    pub provider: Provider,
    /// The configured command: program and arguments, as in `.ai/chxchx-tech.toml`.
    pub command: &'a [String],
    pub executable: &'a Path,
    pub cwd: &'a Path,
    pub prompt: &'a str,
    pub session: Option<&'a str>,
    pub model: Option<&'a str>,
    pub effort: Option<&'a str>,
    pub images: &'a [PathBuf],
    pub access: &'a Access,
    pub opencode_major: Option<u32>,
}

/// `codex app-server`, keeping only the `-c key=value` overrides of the configured command.
fn codex_argv(executable: &Path, command: &[String]) -> Vec<String> {
    let mut argv = vec![executable.display().to_string(), "app-server".to_owned()];
    let configured = command.get(1..).unwrap_or_default();
    for pair in configured.windows(2) {
        if matches!(pair[0].as_str(), "-c" | "--config") {
            argv.extend([pair[0].clone(), pair[1].clone()]);
        }
    }
    argv
}

/// The model the person configured with `-m` / `--model`, used when the chat picked none.
fn configured_model(command: &[String]) -> Option<String> {
    command
        .get(1..)?
        .windows(2)
        .find(|pair| matches!(pair[0].as_str(), "-m" | "--model"))
        .map(|pair| pair[1].clone())
}

/// The request line of the contract for one live turn of Codex or OpenCode.
pub fn request(turn: &TurnRequest<'_>) -> Result<Value, String> {
    let mut env = serde_json::Map::new();
    env.insert("PATH".into(), json!(agent_path().to_string_lossy()));
    for (key, value) in crate::integrations::process_memory::LEAN_ENV {
        env.insert(key.into(), json!(value));
    }
    let mut request = json!({
        "schema": REQUEST_SCHEMA,
        "schema_version": CONTRACT_VERSION,
        "cwd": turn.cwd.to_string_lossy(),
        "prompt": turn.prompt,
        "session_id": turn.session.filter(|session| !session.is_empty()),
        "model": turn.model.filter(|model| !model.is_empty()),
        "effort": turn.effort.filter(|effort| !effort.is_empty()),
        "images": turn.images.iter().map(|image| image.to_string_lossy()).collect::<Vec<_>>(),
    });
    match turn.provider {
        Provider::Codex => {
            request["provider"] = json!("Codex");
            request["argv"] = json!(codex_argv(turn.executable, turn.command));
            request["codex"] = codex_server_settings(turn.access);
            if request["model"].is_null() {
                request["model"] = json!(configured_model(turn.command));
            }
        }
        Provider::OpenCode => {
            request["provider"] = json!("OpenCode");
            request["argv"] = json!([
                turn.executable.display().to_string(),
                "serve",
                "--port",
                "0",
                "--hostname",
                "127.0.0.1"
            ]);
            request["opencode"] = json!({
                "agent": opencode_agent(turn.access),
                "major": turn.opencode_major,
            });
            for (key, value) in opencode_environment_for(turn.access, turn.opencode_major) {
                env.insert(key, json!(value));
            }
        }
        _ => return Err("El servicio solo conduce Codex y OpenCode".to_owned()),
    }
    request["env"] = Value::Object(env);
    Ok(request)
}

#[cfg(test)]
#[path = "agent_adapter_tests.rs"]
mod tests;
