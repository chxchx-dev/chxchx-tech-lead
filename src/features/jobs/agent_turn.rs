//! A chat turn with an agent, run as a job: the same turn Studio runs, without a window.
//!
//! Two halves, so each can be tested on its own:
//! - `prepare` turns the request into a `TurnSpec`: the project alias must be registered, the
//!   project trusted, the agent installed, and the access level within the executor's cap.
//! - `drive` runs a started turn to its end: it forwards every event, keeps the visible history
//!   in `.ai/chat-state` (exactly like Studio, so Studio picks the conversation up), stops the
//!   turn when the job is cancelled and answers what nobody can answer yet.

use super::runner::Context;
use crate::features::agents::{self, Agent, Provider as AgentProvider};
use crate::features::project_catalog::ProjectCatalog;
use crate::integrations::chat_bridge::{Turn, TurnSpec};
use chxchx_protocol::{
    AccessMode, AgentTurnSpec, AnswerBehavior, ErrorCode, JobAnswer, JobError, JobEventBody,
    ProjectId, Provider,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Said when a permission or a question is refused because nobody can answer (`jobs run`).
pub const UNATTENDED_REFUSAL: &str = "Rechazado automáticamente: este trabajo corre sin nadie que pueda aprobarlo. Para aprobar desde otro dispositivo, envía el trabajo al nodo.";
/// Said when nobody answered in time.
pub const EXPIRED_REFUSAL: &str = "Rechazado automáticamente: nadie respondió a tiempo.";
/// How long a job waits for an answer before refusing, so a forgotten card never blocks forever.
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const POLL: Duration = Duration::from_millis(40);

/// What `drive` needs from a running turn; `chat_bridge::Turn` in the product, a fake in tests.
pub trait LiveTurn {
    fn poll(&mut self) -> Vec<Value>;
    /// The process exit code once the turn is over.
    fn exit(&self) -> Option<i32>;
    fn answer(&mut self, answer: &Value) -> Result<(), String>;
    fn cancel(&mut self);
    fn stderr_tail(&self) -> String;
}

impl LiveTurn for Turn {
    fn poll(&mut self) -> Vec<Value> {
        Turn::poll(self)
    }
    fn exit(&self) -> Option<i32> {
        self.exit
    }
    fn answer(&mut self, answer: &Value) -> Result<(), String> {
        Turn::answer(self, answer)
    }
    fn cancel(&mut self) {
        Turn::cancel(self)
    }
    fn stderr_tail(&self) -> String {
        Turn::stderr_tail(self)
    }
}

/// Runs one turn end to end.
pub fn run(
    turn: &AgentTurnSpec,
    cap: AccessMode,
    context: &Context,
) -> Result<Option<String>, JobError> {
    let catalog = crate::features::project_catalog::inspect_registered_projects()
        .map_err(|error| JobError::new(ErrorCode::Internal, error))?;
    let root = project_root(&catalog, &turn.project)?;
    let overview =
        crate::inspect_project(&root).map_err(|error| JobError::new(ErrorCode::NotFound, error))?;
    let mut spec = prepare(
        turn,
        cap,
        &root,
        overview.workspace.trusted,
        &agents::list(&root).agents,
    )?;
    // What the person allowed «en este chat» on an earlier turn of this conversation.
    let label = provider_label(turn.provider);
    spec.allow =
        super::chat_rules::rules_for(turn.project.as_str(), label, spec.session.as_deref());
    if matches!(turn.provider, Provider::Codex | Provider::OpenCode) {
        // Codex and OpenCode go through the agent service. Studio looks for it when it starts;
        // a job runs on its own thread, so it looks here (once per process, then remembered).
        let _ = crate::integrations::agent_adapter::locate(Some(&root));
    }
    let mut live = Turn::launch(&spec)
        .map_err(|error| JobError::new(ErrorCode::ProviderUnavailable, error))?;
    let _ = crate::features::chat_state::begin_turn(
        &root,
        &spec.agent,
        label,
        spec.session.as_deref(),
        &turn.prompt,
    );
    let outcome = drive(&mut live, context, POLL, ANSWER_TIMEOUT);
    super::chat_rules::remember(
        turn.project.as_str(),
        label,
        outcome.session.as_deref().or(spec.session.as_deref()),
        &outcome.remembered,
    );
    let saved = crate::features::chat_state::end_turn(
        &root,
        &spec.agent,
        label,
        outcome.session.as_deref().or(spec.session.as_deref()),
        outcome.replies.iter().map(String::as_str),
    );
    match saved {
        Ok(()) => {
            // The same follow-ups Studio does once a turn's history is saved.
            let _ = crate::features::explicit_memory::capture_studio_memories(&root);
            let _ = crate::features::chat_journal::record_latest_chat(&root, &spec.agent);
        }
        Err(error) => context.emit(JobEventBody::Progress {
            message: format!("No pude guardar el historial del chat: {error}"),
            fraction: None,
        }),
    }
    outcome.result
}

/// The registered, available project an alias names.
pub fn project_root(catalog: &ProjectCatalog, alias: &ProjectId) -> Result<PathBuf, JobError> {
    let project = catalog
        .projects
        .iter()
        .find(|project| project.alias == alias.as_str())
        .ok_or_else(|| {
            JobError::new(
                ErrorCode::NotFound,
                format!("No hay un proyecto registrado con el alias {alias}"),
            )
        })?;
    match (&project.path, project.available) {
        (Some(path), true) => Ok(PathBuf::from(path)),
        _ => Err(JobError::new(
            ErrorCode::NotFound,
            format!("La carpeta del proyecto {alias} no está disponible"),
        )),
    }
}

/// The turn Studio would start for this request, or why it must not start.
pub fn prepare(
    turn: &AgentTurnSpec,
    cap: AccessMode,
    root: &Path,
    trusted: bool,
    agents: &[Agent],
) -> Result<TurnSpec, JobError> {
    if !trusted {
        return Err(JobError::new(
            ErrorCode::UntrustedProject,
            format!(
                "El proyecto {} no es de confianza: concédela en la PC antes de hablar con un agente",
                turn.project
            ),
        ));
    }
    let wanted = match turn.provider {
        Provider::Claude => AgentProvider::Claude,
        Provider::Codex => AgentProvider::Codex,
        Provider::OpenCode => AgentProvider::OpenCode,
    };
    let agent = agents
        .iter()
        .find(|agent| agent.provider == wanted && agent.available())
        .ok_or_else(|| {
            JobError::new(
                ErrorCode::ProviderUnavailable,
                format!(
                    "{} no está instalado o no se encontró para este proyecto",
                    wanted.label()
                ),
            )
        })?;
    Ok(TurnSpec {
        agent: agent.id.clone(),
        provider: provider_label(turn.provider).to_owned(),
        project: root.to_path_buf(),
        prompt: turn.prompt.clone(),
        session: turn.session.clone(),
        mode: turn.access.capped(cap).as_str().to_owned(),
        network: turn.network,
        model: turn.model.clone(),
        effort: turn.effort.clone(),
        agent_command: agent.command.clone(),
        executable: agent.executable.clone(),
        agent_cwd: agent.cwd.clone(),
        trusted: true, // checked above
        ..Default::default()
    })
}

/// The provider names chat-state and the chat bridge use.
fn provider_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "Claude",
        Provider::Codex => "Codex",
        Provider::OpenCode => "OpenCode",
    }
}

pub struct Outcome {
    pub result: Result<Option<String>, JobError>,
    pub session: Option<String>,
    pub replies: Vec<String>,
    /// Rules the person allowed for the rest of this chat, from the options the agent offered.
    pub remembered: Vec<String>,
}

/// Runs a started turn to its end.
///
/// A live permission or question (one with a `request_id`) waits for an answer through the job
/// when someone can give it (the node), up to `answer_timeout`; otherwise it is refused at once.
/// Every decision is recorded as an `answered` agent event, so every client can close the card.
pub fn drive(
    turn: &mut impl LiveTurn,
    context: &Context,
    poll: Duration,
    answer_timeout: Duration,
) -> Outcome {
    let mut session = None;
    let mut replies = Vec::new();
    let mut stopping = false;
    let mut waiting: Vec<(String, Instant)> = Vec::new();
    let mut permissions: std::collections::HashMap<String, Value> =
        std::collections::HashMap::new();
    let mut remembered: Vec<String> = Vec::new();
    loop {
        if context.cancelled() && !stopping {
            stopping = true;
            turn.cancel();
        }
        for event in turn.poll() {
            match event["type"].as_str().unwrap_or_default() {
                "session" => {
                    session = event["session_id"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .map(str::to_owned)
                        .or(session);
                }
                "message" if event["role"] == "assistant" => {
                    if let Some(text) = event["text"].as_str() {
                        replies.push(text.to_owned());
                    }
                }
                _ => {}
            }
            let request = live_request(&event);
            if let Some(request_id) = &request
                && event["type"] == "permission"
            {
                permissions.insert(request_id.clone(), event.clone());
            }
            let wait = request.is_some() && context.attended() && !stopping;
            if let (true, Some(request_id)) = (wait, &request) {
                // Registered before the card goes out, so even an instant answer is accepted.
                context.expect_answer(request_id);
                waiting.push((request_id.clone(), Instant::now()));
            }
            context.emit(JobEventBody::Agent { event });
            if let (false, Some(request_id)) = (wait, &request) {
                refuse(turn, request_id, context, UNATTENDED_REFUSAL);
            }
        }
        for answer in context.answers() {
            waiting.retain(|(id, _)| *id != answer.request_id);
            if answer.behavior == Some(AnswerBehavior::Allow)
                && answer.scope.as_deref() == Some("chat")
                && let Some(card) = permissions.get(&answer.request_id)
            {
                remembered.extend(super::chat_rules::offered_for_chat(card));
            }
            deliver(turn, &answer, context);
        }
        let expired: Vec<String> = waiting
            .iter()
            .filter(|(_, since)| since.elapsed() >= answer_timeout || stopping)
            .map(|(id, _)| id.clone())
            .collect();
        for request_id in expired {
            context.settle(&request_id);
            waiting.retain(|(id, _)| *id != request_id);
            refuse(turn, &request_id, context, EXPIRED_REFUSAL);
        }
        if let Some(code) = turn.exit() {
            for (request_id, _) in waiting.drain(..) {
                context.settle(&request_id);
            }
            let result = if code == 0 || stopping {
                Ok(replies.last().map(|text| summary(text)))
            } else {
                let detail = turn.stderr_tail();
                let detail = detail.trim();
                Err(JobError::new(
                    ErrorCode::ProviderFailed,
                    if detail.is_empty() {
                        format!("El agente terminó con el código {code}")
                    } else {
                        format!("El agente terminó con el código {code}: {detail}")
                    },
                ))
            };
            return Outcome {
                result,
                session,
                replies,
                remembered,
            };
        }
        std::thread::sleep(poll);
    }
}

/// The `request_id` of a permission or question the turn is blocked on. Cards without one are
/// only shown, as in Studio.
fn live_request(event: &Value) -> Option<String> {
    matches!(event["type"].as_str(), Some("permission" | "question"))
        .then(|| event["request_id"].as_str())
        .flatten()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

/// Passes a person's answer to the agent.
fn deliver(turn: &mut impl LiveTurn, answer: &JobAnswer, context: &Context) {
    let mut record = answer.to_bridge();
    record["type"] = json!("answered");
    if let Err(error) = turn.answer(&answer.to_bridge()) {
        record["error"] = json!(error);
    }
    context.emit(JobEventBody::Agent { event: record });
}

/// Denies a request nobody is going to answer, saying why.
fn refuse(turn: &mut impl LiveTurn, request_id: &str, context: &Context, reason: &str) {
    let answer = json!({ "type": "answer", "request_id": request_id, "behavior": "deny" });
    let mut record = json!({
        "type": "answered", "request_id": request_id, "behavior": "deny",
        "automatic": true, "reason": reason,
    });
    if let Err(error) = turn.answer(&answer) {
        record["error"] = json!(error);
    }
    context.emit(JobEventBody::Agent { event: record });
}

fn summary(text: &str) -> String {
    const LIMIT: usize = 280;
    let text = text.trim();
    if text.chars().count() <= LIMIT {
        return text.to_owned();
    }
    let cut: String = text.chars().take(LIMIT).collect();
    format!("{cut}…")
}
