//! A chat turn that runs on the local node instead of inside Studio (ADR 0017, H5).
//!
//! Studio creates a job on the node and follows its events; the node runs the agent, keeps the
//! visible history in `.ai/chat-state` and the explicit-memory and journal follow-ups, and goes on
//! when Studio closes. A `Turn` of this kind looks like any other to the chat: `poll` returns the
//! same normalized agent events, `answer` and `cancel` reach the agent, `exit` says when it is over.
//!
//! Closing Studio only stops *following*: the job keeps running on the node. A new Studio finds it
//! again with `Turn::attach_to_node`.

use super::claude_runner::agent_event;
use super::*;
use crate::features::agents::Provider as AgentProvider;
use crate::integrations::node_link::{self, Endpoint};
use chxchx_protocol::{
    AccessMode, AgentTurnSpec, AnswerBehavior, Job, JobAnswer, JobEvent, JobEventBody, JobId,
    JobSpec, JobState, ProjectId, Provider,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// How often the job is asked for news.
const FOLLOW_EVERY: Duration = Duration::from_millis(250);
/// Failures in a row before the node is given up on (about ten seconds).
const GIVE_UP_AFTER: u32 = if cfg!(test) { 4 } else { 40 };

pub(super) struct RemoteRun {
    endpoint: Endpoint,
    job: JobId,
    /// `(exit code, cancelled)` once the job is over. Set after every event was sent.
    finished: Arc<Mutex<Option<(i32, bool)>>>,
    /// Tells the follower to stop. The job itself is not touched.
    stop: Arc<AtomicBool>,
}

impl RemoteRun {
    pub(super) fn finished(&self) -> Option<(i32, bool)> {
        self.finished.lock().ok().and_then(|slot| *slot)
    }

    pub(super) fn answer(&self, answer: &Value) -> Result<(), String> {
        let behavior = match answer["behavior"].as_str() {
            Some("allow") => Some(AnswerBehavior::Allow),
            Some("deny") => Some(AnswerBehavior::Deny),
            _ => None,
        };
        let request = JobAnswer {
            request_id: answer["request_id"].as_str().unwrap_or_default().to_owned(),
            behavior,
            scope: answer["scope"].as_str().map(str::to_owned),
            answers: answer["answers"].as_object().cloned(),
        };
        node_link::answer(&self.endpoint, &self.job, &request)
    }

    /// Asks the node to stop the job, without making the window wait for the answer.
    pub(super) fn cancel(&self) {
        let (endpoint, job) = (self.endpoint.clone(), self.job.clone());
        std::thread::spawn(move || {
            let _ = node_link::cancel(&endpoint, &job);
        });
    }
}

impl Drop for RemoteRun {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn wire_provider(label: &str) -> Option<Provider> {
    match label {
        "Claude" => Some(Provider::Claude),
        "Codex" => Some(Provider::Codex),
        "OpenCode" => Some(Provider::OpenCode),
        _ => None,
    }
}

/// Whether a turn can go to the node as it is. The node runs the project's own default agent with
/// a plain message; anything Studio adds around the turn (pictures, a recap of a lost session, the
/// rules approved for it, a custom agent command) stays on the local path rather than being lost.
pub fn delegable(spec: &TurnSpec) -> bool {
    let provider =
        AgentProvider::from_command(spec.agent_command.first().map_or("", String::as_str));
    let wanted = match spec.provider.as_str() {
        "Claude" => AgentProvider::Claude,
        "Codex" => AgentProvider::Codex,
        "OpenCode" => AgentProvider::OpenCode,
        _ => return false,
    };
    spec.images.is_empty()
        && spec.resume_recap.is_empty()
        && spec.recovered_memories.is_empty()
        && spec.allow.is_empty()
        && spec.agent_command.len() == 1
        && provider == wanted
        && (spec.agent_cwd.as_os_str().is_empty() || spec.agent_cwd == spec.project)
        && AccessMode::parse(&spec.mode).is_some()
}

impl Turn {
    /// Starts the turn on the node as a job and follows it.
    pub fn launch_on_node(
        spec: &TurnSpec,
        endpoint: Endpoint,
        project: &ProjectId,
    ) -> Result<Self, String> {
        let provider = wire_provider(&spec.provider)
            .ok_or_else(|| format!("El nodo no conduce {}", spec.provider))?;
        let access = AccessMode::parse(&spec.mode)
            .ok_or_else(|| format!("Modo de acceso no válido: {}", spec.mode))?;
        let job = node_link::create_job(
            &endpoint,
            &JobSpec::AgentTurn(AgentTurnSpec {
                project: project.clone(),
                provider,
                prompt: spec.prompt.clone(),
                session: spec.session.clone().filter(|session| !session.is_empty()),
                model: spec.model.clone(),
                effort: spec.effort.clone(),
                access,
                network: spec.network,
            }),
            &node_link::fresh_key(),
        )?;
        Ok(Self::follow_job(endpoint, &job, &spec.provider))
    }

    /// Picks up a job the node is already running (one Studio started before it was closed, or
    /// one a phone started), replaying what it said so far.
    pub fn attach_to_node(endpoint: Endpoint, job: &Job, provider: &str) -> Self {
        Self::follow_job(endpoint, job, provider)
    }

    fn follow_job(endpoint: Endpoint, job: &Job, provider: &str) -> Self {
        let (sender, events) = channel();
        let finished = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        {
            let (endpoint, job, finished, stop) = (
                endpoint.clone(),
                job.id.clone(),
                finished.clone(),
                stop.clone(),
            );
            std::thread::spawn(move || follow(&endpoint, &job, &sender, &finished, &stop));
        }
        Self {
            child: None,
            stdin: None,
            events,
            stderr: Arc::new(Mutex::new(VecDeque::new())),
            provider: provider.to_owned(),
            kind: TurnKind::Chat,
            handoff_text: String::new(),
            consume_handoff_session: None,
            context_tokens: None,
            context_window: None,
            cancelled: false,
            pending_claude: Arc::new(Mutex::new(HashMap::new())),
            exit: None,
            pre_events: Vec::new(),
            native: None,
            remote: Some(RemoteRun {
                endpoint,
                job: job.id.clone(),
                finished,
                stop,
            }),
        }
    }

    /// Whether the node runs this turn. Then the node also saves the conversation, so Studio must
    /// not save it a second time.
    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    pub fn node_job(&self) -> Option<&JobId> {
        self.remote.as_ref().map(|remote| &remote.job)
    }
}

/// Reads the job's events until it ends, the follower is told to stop or the node is lost.
fn follow(
    endpoint: &Endpoint,
    job: &JobId,
    sender: &std::sync::mpsc::Sender<Value>,
    finished: &Mutex<Option<(i32, bool)>>,
    stop: &AtomicBool,
) {
    let (mut since, mut failures) = (0u64, 0u32);
    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match node_link::events(endpoint, job, since) {
            Ok(events) => {
                failures = 0;
                let mut over = None;
                for event in events {
                    since = since.max(event.seq);
                    let (shown, end) = translate(event);
                    if let Some(shown) = shown
                        && sender.send(shown).is_err()
                    {
                        return;
                    }
                    over = over.or(end);
                    super::super::wake::notify();
                }
                if let Some(result) = over {
                    // After the last event went out: `poll` reads this first, then the events.
                    if let Ok(mut slot) = finished.lock() {
                        *slot = Some(result);
                    }
                    super::super::wake::notify();
                    return;
                }
            }
            Err(_) => {
                failures += 1;
                if failures >= GIVE_UP_AFTER {
                    let _ = sender.send(agent_event(
                        "error",
                        serde_json::json!({
                            "message": "Se perdió la conexión con el nodo. El trabajo sigue allí: vuelve a abrir el chat cuando el nodo responda."
                        }),
                    ));
                    if let Ok(mut slot) = finished.lock() {
                        *slot = Some((1, false));
                    }
                    super::super::wake::notify();
                    return;
                }
            }
        }
        std::thread::sleep(FOLLOW_EVERY);
    }
}

/// A job event as the chat shows it, and the end of the job if this event is it.
fn translate(event: JobEvent) -> (Option<Value>, Option<(i32, bool)>) {
    match event.body {
        JobEventBody::Agent { event } => (Some(event), None),
        JobEventBody::Progress { message, .. } => (
            Some(agent_event(
                "notice",
                serde_json::json!({ "message": message }),
            )),
            None,
        ),
        JobEventBody::Error { error } => (
            Some(agent_event(
                "error",
                serde_json::json!({ "message": error.message }),
            )),
            None,
        ),
        JobEventBody::State { state } => (
            None,
            match state {
                JobState::Queued | JobState::Running => None,
                JobState::Succeeded => Some((0, false)),
                JobState::Cancelled => Some((0, true)),
                JobState::Failed | JobState::Interrupted => Some((1, false)),
            },
        ),
    }
}

#[cfg(test)]
#[path = "node_runner_tests.rs"]
mod tests;
