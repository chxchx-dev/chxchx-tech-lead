//! Claude Code turns run by Rust itself, with no Python CLI in between.
//!
//! `claude -p --output-format stream-json` is spawned directly. Its records already go through
//! the same reader, normalizer and control-answer code as the ones the adapter used to forward,
//! so the Studio chat cannot tell the difference; this module only builds the process, the first
//! message, and the `start`/`done` events the adapter used to print around a turn.
use super::*;
use crate::features::agents::Provider;
use crate::integrations::agent_access::{Access, claude_turn_arguments};
use std::sync::atomic::{AtomicBool, Ordering};

/// Longest handoff the hidden summary turn may produce.
const MAX_SUMMARY_CHARS: usize = 24_000;
const ATTACHMENTS: [&str; 2] = [".ai", "chat-attachments"];

/// What a natively run turn needs after it started.
pub(super) struct NativeRun {
    pid: u32,
    /// The session the turn resumed, reported again if Claude never announces another.
    planned_session: Option<String>,
    session: Option<String>,
    /// Assistant text of a hidden handoff turn.
    summary: Vec<String>,
    /// Set before the process is stopped so its exit is reported as a cancellation, not a failure.
    stopping: Arc<AtomicBool>,
}

/// Whether this agent's turns can run without the Python CLI.
pub fn runs_natively(agent: &crate::features::agents::Agent) -> bool {
    agent.provider == Provider::Claude && agent.executable.is_some()
}

fn agent_event(kind: &str, fields: Value) -> Value {
    let mut event = serde_json::json!({
        "schema": "chxchx.agent-event", "schema_version": 1, "type": kind,
    });
    if let (Some(target), Some(extra)) = (event.as_object_mut(), fields.as_object()) {
        target.extend(extra.clone());
    }
    event
}

/// Copies the turn's images inside the project (so every access mode can read them) and lists
/// them for the prompt, because Claude takes attachments as paths in the text.
fn stage_images(project: &Path, images: &[PathBuf]) -> Result<String, String> {
    if images.is_empty() {
        return Ok(String::new());
    }
    let mut folder = project.to_path_buf();
    for part in ATTACHMENTS {
        folder.push(part);
        match std::fs::symlink_metadata(&folder) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err(format!("{} no es una carpeta normal", folder.display()));
            }
            Ok(_) => {}
            Err(_) => std::fs::create_dir(&folder).map_err(|error| error.to_string())?,
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let mut listed = String::new();
    for (index, source) in images.iter().enumerate() {
        let name = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "imagen".to_owned());
        let target = folder.join(format!("{stamp}-{index}-{name}"));
        std::fs::copy(source, &target)
            .map_err(|error| format!("No pude adjuntar {name}: {error}"))?;
        listed.push_str(&format!("\n- .ai/chat-attachments/{stamp}-{index}-{name}"));
    }
    Ok(listed)
}

/// The text written to Claude for a chat turn, plus what the person should be told about it.
/// A new session starts with the project context; a resumed one only carries the request.
fn chat_prompt(
    spec: &TurnSpec,
    session: Option<&str>,
    handoff: Option<&str>,
) -> Result<(String, Vec<Value>), String> {
    let mut notices = Vec::new();
    let mut request = spec.prompt.clone();
    let staged = stage_images(&spec.project, &spec.images)?;
    if !staged.is_empty() {
        request.push_str(&format!(
            "\n\nImágenes adjuntas por el usuario (ábrelas con la herramienta Read antes de responder):{staged}"
        ));
    }
    if session.is_some() {
        return Ok((request, notices));
    }
    let compacted = match crate::features::context_compaction::compact_context(&spec.project) {
        Ok(result) => result.moved_count(),
        Err(error) => {
            notices.push(agent_event(
                "notice",
                serde_json::json!({"message": format!("No pude compactar el contexto: {error}"), "error": true}),
            ));
            0
        }
    };
    if compacted > 0 {
        notices.push(agent_event(
            "notice",
            serde_json::json!({
                "message": format!("Contexto compactado: {compacted} sección(es) antiguas movidas a .ai/history/."),
                "error": false,
            }),
        ));
    }
    let mut context = crate::integrations::chat_context::initial_context_with_memories(
        &spec.project,
        &spec.provider,
        &spec.recovered_memories,
    );
    // A handoff is richer than a recap: it wins.
    match handoff {
        Some(text) => context.push_str(&crate::integrations::chat_context::handoff_block(text)),
        None => context.push_str(&spec.resume_recap),
    }
    Ok((
        format!(
            "{context}\n\n{}\n{request}",
            crate::integrations::chat_context::CHAT_PANEL_REQUEST_MARKER
        ),
        notices,
    ))
}

fn spawn(spec: &TurnSpec, args: Vec<String>) -> Result<Child, String> {
    let executable = spec
        .executable
        .as_ref()
        .ok_or("Claude Code no está instalado")?;
    let cwd = if spec.agent_cwd.as_os_str().is_empty() {
        &spec.project
    } else {
        &spec.agent_cwd
    };
    let mut process = Command::new(executable);
    process
        .args(args)
        .current_dir(cwd)
        .env("PATH", agent_path())
        .envs(crate::integrations::process_memory::LEAN_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        process.process_group(0); // one group, so stopping it also stops what Claude started
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    process
        .spawn()
        .map_err(|error| format!("No pude iniciar {}: {error}", executable.display()))
}

fn access_for(spec: &TurnSpec) -> Result<Access, String> {
    Access::new(spec.mode.clone(), spec.network, spec.allow.clone())
}

fn configured_arguments(spec: &TurnSpec) -> &[String] {
    spec.agent_command.get(1..).unwrap_or_default()
}

fn require_trust(spec: &TurnSpec) -> Result<(), String> {
    if spec.trusted {
        Ok(())
    } else {
        Err("Revisa y concede confianza al proyecto antes de usar agentes.".to_owned())
    }
}

/// Stops the process and everything it started: politely first, by force after a few seconds.
fn stop(pid: u32) {
    std::thread::spawn(move || {
        #[cfg(unix)]
        {
            let group = format!("-{pid}");
            let _ = Command::new("/bin/kill")
                .args(["-TERM", "--", &group])
                .status();
            let until = Instant::now() + Duration::from_secs(3);
            let alive = || {
                Command::new("/bin/kill")
                    .args(["-0", "--", &group])
                    .stderr(Stdio::null())
                    .status()
                    .is_ok_and(|status| status.success())
            };
            while alive() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(50));
            }
            if alive() {
                let _ = Command::new("/bin/kill")
                    .args(["-KILL", "--", &group])
                    .status();
            }
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output();
        }
    });
}

impl Turn {
    /// Starts a chat turn: Claude runs here, any other provider goes through `cli`.
    pub fn launch(cli: Option<&Path>, spec: &TurnSpec) -> Result<Self, String> {
        if spec.provider == "Claude" && spec.executable.is_some() {
            return Self::start_claude(spec);
        }
        Self::start(cli.ok_or_else(missing_cli_hint)?, spec)
    }

    /// Starts the hidden summary turn of a nearly full session.
    pub fn launch_handoff(cli: Option<&Path>, spec: &TurnSpec) -> Result<Self, String> {
        if spec.provider == "Claude" && spec.executable.is_some() {
            return Self::start_claude_handoff(spec);
        }
        Self::start_handoff(cli.ok_or_else(missing_cli_hint)?, spec)
    }

    fn start_claude(spec: &TurnSpec) -> Result<Self, String> {
        require_trust(spec)?;
        let access = access_for(spec)?;
        let (pending_handoff, session) = session_plan(spec);
        let (prompt, notices) = chat_prompt(spec, session.as_deref(), pending_handoff.as_deref())?;
        let args = claude_turn_arguments(
            configured_arguments(spec),
            &access,
            session.as_deref(),
            spec.model.as_deref(),
            spec.effort.as_deref(),
            false,
            true,
        );
        let mut child = spawn(spec, args)?;
        let first = serde_json::json!({
            "type": "user", "message": {"role": "user", "content": prompt}
        });
        let mut stdin = child.stdin.take();
        let written = stdin.as_mut().map(|stdin| {
            stdin
                .write_all(first.to_string().as_bytes())
                .and_then(|()| stdin.write_all(b"\n"))
                .and_then(|()| stdin.flush())
        });
        let mut turn = Self::adopt(child, stdin, spec, TurnKind::Chat, session)?;
        // The handoff is only retired once the new session has answered (see the Studio chat).
        turn.consume_handoff_session = pending_handoff.and(spec.session.clone());
        turn.pre_events = vec![agent_event(
            "start",
            serde_json::json!({"agent": spec.agent, "provider": "Claude", "mode": spec.mode}),
        )];
        turn.pre_events.extend(notices);
        if let Some(Err(error)) = written {
            // The process is gone already; its exit will report it.
            turn.pre_events.push(agent_event(
                "notice",
                serde_json::json!({"message": format!("No pude enviar el mensaje a Claude: {error}"), "error": true}),
            ));
        }
        Ok(turn)
    }

    fn start_claude_handoff(spec: &TurnSpec) -> Result<Self, String> {
        require_trust(spec)?;
        let session = spec
            .session
            .clone()
            .filter(|session| !session.is_empty())
            .ok_or("El handoff requiere una sesión activa")?;
        // Reading only: the summary must never be able to change the project.
        let access = Access::new("read-only", false, Vec::new())?;
        let args = claude_turn_arguments(
            configured_arguments(spec),
            &access,
            Some(&session),
            spec.model.as_deref(),
            spec.effort.as_deref(),
            false,
            false,
        );
        let mut child = spawn(spec, args)?;
        let mut stdin = child.stdin.take();
        if let Some(pipe) = stdin.as_mut() {
            let _ = pipe.write_all(crate::features::chat_handoff::PROMPT.as_bytes());
        }
        drop(stdin); // the prompt is plain text: the end of the input starts the turn
        Self::adopt(
            child,
            None,
            spec,
            TurnKind::Handoff {
                session_id: session.clone(),
            },
            Some(session),
        )
    }

    /// Wires the readers around a spawned Claude process.
    fn adopt(
        mut child: Child,
        stdin: Option<ChildStdin>,
        spec: &TurnSpec,
        kind: TurnKind,
        session: Option<String>,
    ) -> Result<Self, String> {
        let stopping = Arc::new(AtomicBool::new(false));
        let pid = child.id();
        let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Claude no abrió sus canales de salida".to_owned());
        };
        let (sender, events) = channel();
        let pending_claude = Arc::new(Mutex::new(HashMap::new()));
        spawn_event_reader(stdout, sender, pending_claude.clone(), Some("Claude"));
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        spawn_stderr_reader(stderr, tail.clone());
        Ok(Self {
            child: Some(child),
            stdin,
            events,
            stderr: tail,
            provider: spec.provider.clone(),
            kind,
            handoff_text: String::new(),
            consume_handoff_session: None,
            context_tokens: None,
            context_window: None,
            cancelled: false,
            pending_claude,
            prompt_file: PathBuf::new(),
            resume_recap_file: None,
            rust_context_file: None,
            exit: None,
            pre_events: Vec::new(),
            native: Some(NativeRun {
                pid,
                planned_session: session,
                session: None,
                summary: Vec::new(),
                stopping,
            }),
        })
    }

    /// Stops a natively run turn; its exit is then reported as a cancellation.
    pub(super) fn stop_native(&mut self) -> bool {
        let Some(native) = self.native.as_ref() else {
            return false;
        };
        if self.exit.is_none() {
            native.stopping.store(true, Ordering::SeqCst);
            stop(native.pid);
        }
        true
    }

    /// Follows what a native turn prints: the session it announced and the text of a summary.
    pub(super) fn observe_native(&mut self, event: &Value) {
        let Some(native) = self.native.as_mut() else {
            return;
        };
        match event["type"].as_str() {
            Some("session") => {
                if let Some(id) = event["session_id"].as_str().filter(|id| !id.is_empty()) {
                    native.session = Some(id.to_owned());
                }
            }
            Some("message") if matches!(self.kind, TurnKind::Handoff { .. }) => {
                if let Some(text) = event["text"].as_str() {
                    native.summary.push(text.to_owned());
                }
            }
            _ => {}
        }
    }

    /// The events the adapter used to print when a turn ended.
    pub(super) fn closing_events(&mut self) -> Vec<Value> {
        let code = self.exit.unwrap_or(-1);
        let Some(native) = self.native.as_mut() else {
            return Vec::new();
        };
        let cancelled = native.stopping.load(Ordering::SeqCst);
        if matches!(self.kind, TurnKind::Handoff { .. }) {
            let summary: String = native
                .summary
                .join("\n\n")
                .trim()
                .chars()
                .take(MAX_SUMMARY_CHARS)
                .collect();
            if code == 0 && !cancelled && !summary.is_empty() {
                return vec![agent_event("handoff", serde_json::json!({"text": summary}))];
            }
            return Vec::new();
        }
        let session = native
            .session
            .clone()
            .or_else(|| native.planned_session.clone());
        vec![agent_event(
            "done",
            serde_json::json!({"exit_code": code, "cancelled": cancelled, "session_id": session}),
        )]
    }
}

/// The handoff waiting for this session, and the session to resume (none when a handoff starts a new one).
fn session_plan(spec: &TurnSpec) -> (Option<String>, Option<String>) {
    let pending = spec.session.as_deref().and_then(|session| {
        crate::features::chat_handoff::load(&spec.project, &spec.provider, session)
    });
    let session = if pending.is_some() {
        None
    } else {
        spec.session.clone().filter(|session| !session.is_empty())
    };
    (pending, session)
}

#[cfg(all(test, unix))]
#[path = "claude_runner_tests.rs"]
mod tests;
