//! Codex and OpenCode turns, run by the agent-adapter service (`services/agent-adapter/`).
//!
//! Rust prepares everything (trust, access, context, prompt, images) and sends the service one
//! request; the service drives the provider's live protocol and prints `chxchx.agent-event` lines
//! that go through the same reader as any other turn. Answers and `cancel` travel on its stdin.
use super::claude_runner::{
    ChatPrompt, access_for, agent_event, chat_prompt, require_trust, session_plan,
};
use super::*;
use crate::features::agents::Provider;
use crate::integrations::agent_access::Access;
use crate::integrations::agent_adapter::{self, TurnRequest};

fn spawn_service(adapter: &Path, project: &Path) -> Result<Child, String> {
    let mut process = Command::new(adapter);
    process
        .arg("run")
        .current_dir(project)
        .env("PATH", agent_path())
        // The contract is UTF-8 whatever the console's code page.
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .envs(crate::integrations::process_memory::LEAN_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    process
        .spawn()
        .map_err(|error| format!("No pude iniciar el servicio de agentes: {error}"))
}

fn provider_of(spec: &TurnSpec) -> Result<Provider, String> {
    match spec.provider.as_str() {
        "Codex" => Ok(Provider::Codex),
        "OpenCode" => Ok(Provider::OpenCode),
        other => Err(format!("El servicio no conduce {other}")),
    }
}

fn request_for(
    spec: &TurnSpec,
    access: &Access,
    session: Option<&str>,
    prompt: &str,
    images: &[PathBuf],
) -> Result<Value, String> {
    let executable = spec
        .executable
        .as_deref()
        .ok_or("El agente no está instalado")?;
    let provider = provider_of(spec)?;
    let cwd = if spec.agent_cwd.as_os_str().is_empty() {
        &spec.project
    } else {
        &spec.agent_cwd
    };
    agent_adapter::request(&TurnRequest {
        provider,
        command: &spec.agent_command,
        executable,
        cwd,
        prompt,
        session,
        model: spec.model.as_deref(),
        effort: spec.effort.as_deref(),
        images,
        access,
        opencode_major: (provider == Provider::OpenCode)
            .then(|| agent_adapter::opencode_major(executable))
            .flatten(),
    })
}

/// Starts the service and gives it the request; the input stays open for the answers.
fn begin(
    adapter: &Path,
    spec: &TurnSpec,
    request: &Value,
) -> Result<(Child, Option<ChildStdin>), String> {
    let mut child = spawn_service(adapter, &spec.project)?;
    let mut stdin = child.stdin.take();
    let written = stdin.as_mut().map(|pipe| {
        pipe.write_all(request.to_string().as_bytes())
            .and_then(|()| pipe.write_all(b"\n"))
            .and_then(|()| pipe.flush())
    });
    if let Some(Err(error)) = written {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "No pude enviar la solicitud al servicio de agentes: {error}"
        ));
    }
    Ok((child, stdin))
}

impl Turn {
    pub(super) fn start_service(adapter: &Path, spec: &TurnSpec) -> Result<Self, String> {
        require_trust(spec)?;
        let access = access_for(spec)?;
        let (pending_handoff, session) = session_plan(spec);
        let ChatPrompt {
            text,
            notices,
            staged,
        } = chat_prompt(spec, session.as_deref(), pending_handoff.as_deref(), false)?;
        let request = request_for(spec, &access, session.as_deref(), &text, &staged)?;
        let (child, stdin) = begin(adapter, spec, &request)?;
        let mut turn = Self::adopt(child, stdin, spec, TurnKind::Chat, session, true)?;
        // The handoff is only retired once the new session has answered (see the Studio chat).
        turn.consume_handoff_session = pending_handoff.and(spec.session.clone());
        turn.pre_events = vec![agent_event(
            "start",
            serde_json::json!({"agent": spec.agent, "provider": spec.provider, "mode": spec.mode}),
        )];
        turn.pre_events.extend(notices);
        Ok(turn)
    }

    pub(super) fn start_service_handoff(adapter: &Path, spec: &TurnSpec) -> Result<Self, String> {
        require_trust(spec)?;
        let session = spec
            .session
            .clone()
            .filter(|session| !session.is_empty())
            .ok_or("El handoff requiere una sesión activa")?;
        // Reading only: the summary must never be able to change the project.
        let access = Access::new("read-only", false, Vec::new())?;
        let request = request_for(
            spec,
            &access,
            Some(&session),
            crate::features::chat_handoff::PROMPT,
            &[],
        )?;
        let (child, stdin) = begin(adapter, spec, &request)?;
        Self::adopt(
            child,
            stdin,
            spec,
            TurnKind::Handoff {
                session_id: session.clone(),
            },
            Some(session),
            true,
        )
    }

    /// What the service prints that Studio must not show as it is: its closing line becomes the
    /// `done` event of a turn, and nobody is there to answer the cards of a hidden summary.
    pub(super) fn translate_service_event(&mut self, event: Value) -> Option<Value> {
        let is_handoff = self.is_handoff();
        let Some(native) = self.native.as_ref().filter(|native| native.service) else {
            return Some(event); // not a turn of the service: nothing to translate
        };
        let session = native.session.clone();
        match event["type"].as_str() {
            Some("adapter_done") => {
                let code = event["exit_code"]
                    .as_i64()
                    .and_then(|code| i32::try_from(code).ok())
                    .unwrap_or(-1);
                let cancelled = event["cancelled"].as_bool().unwrap_or(false);
                if let Some(native) = self.native.as_mut() {
                    native.reported = Some((code, cancelled));
                }
                self.stdin = None; // the turn is over: nothing is left to answer
                (!is_handoff).then(|| {
                    agent_event(
                        "done",
                        serde_json::json!({
                            "exit_code": code,
                            "cancelled": cancelled,
                            "session_id": event["session_id"].as_str().map(str::to_owned).or(session),
                        }),
                    )
                })
            }
            Some("permission" | "question") if is_handoff => {
                if let Some(request_id) = event["request_id"].as_str() {
                    let refusal = if event["type"] == "question" {
                        serde_json::json!({"type": "answer", "request_id": request_id, "answers": {}})
                    } else {
                        serde_json::json!({"type": "answer", "request_id": request_id, "behavior": "deny"})
                    };
                    let _ = self.write_line(&refusal.to_string());
                }
                None
            }
            _ => Some(event),
        }
    }
}

#[cfg(all(test, unix))]
#[path = "service_runner_tests.rs"]
mod tests;
