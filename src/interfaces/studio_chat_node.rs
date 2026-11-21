//! Chat turns that live on the local node (ADR 0017, H5): handing a turn to it, and picking up the
//! turns it is already running (the ones Studio started before it was closed, or a phone started).
//!
//! It only happens when the person turned it on in Settings and the node answers. Anything the
//! node cannot carry (pictures, a recap, approved rules, a custom agent command) runs here, as
//! before, and if the node fails the turn falls back to running here.
use super::*;
use crate::integrations::node_link::{self, Endpoint};
use chxchx_protocol::{Job, JobSpec, Provider};

/// The provider names the chat uses.
fn label(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "Claude",
        Provider::Codex => "Codex",
        Provider::OpenCode => "OpenCode",
    }
}

/// Whether Studio is set to use the node.
pub(super) fn enabled(state: &Dashboard) -> bool {
    state.session.preferences().node.delegate
}

/// Starts the turn on the node when it should go there, otherwise (or if the node refuses)
/// here. `wanted` is whether this turn is one a person typed: hidden retries stay local, because
/// the node would show them as something the person wrote.
pub(super) fn launch(spec: &TurnSpec, wanted: bool) -> Result<Turn, String> {
    if wanted
        && bridge::delegable(spec)
        && let Some(endpoint) = node_link::endpoint()
        && let Some(project) = node_link::alias_for(&spec.project)
        && let Ok(turn) = Turn::launch_on_node(spec, endpoint, &project)
    {
        return Ok(turn);
    }
    Turn::launch(spec)
}

type Found = Option<(Endpoint, Vec<Job>)>;

/// The turns of the project at `root` that the node is running. Blocking: off the UI thread.
pub(in crate::interfaces::studio) fn search(root: &std::path::Path) -> Found {
    let endpoint = node_link::endpoint()?;
    let project = node_link::alias_for(root)?;
    let running = node_link::list_jobs(&endpoint)
        .ok()?
        .into_iter()
        .filter(|job| !job.state.is_terminal())
        .filter(|job| matches!(&job.spec, JobSpec::AgentTurn(turn) if turn.project == project))
        .collect();
    Some((endpoint, running))
}

/// Looks, in the background, for turns of this project that the node is running and that no chat
/// is following yet.
pub(super) fn adopt(state: &mut Dashboard) -> Task<Message> {
    let idle = state
        .chat
        .chats
        .iter()
        .any(|chat| chat.turn.is_none() && !chat.preparing && chat.problem.is_none());
    if !enabled(state) || !idle {
        return Task::none();
    }
    let root = state.root.clone();
    jobs::quiet(
        move || search(&root),
        |state, found| {
            attach(state, found);
            Task::none()
        },
    )
}

pub(in crate::interfaces::studio) fn attach(state: &mut Dashboard, found: Found) {
    let Some((endpoint, running)) = found else {
        return;
    };
    for chat in &mut state.chat.chats {
        if chat.turn.is_some() || chat.preparing || chat.problem.is_some() {
            continue; // it may have started a turn of its own meanwhile
        }
        let label = super::turns::state_provider(chat.agent.provider);
        let Some((job, prompt)) = running.iter().rev().find_map(|job| match &job.spec {
            JobSpec::AgentTurn(turn) if self::label(turn.provider) == label => {
                Some((job, turn.prompt.clone()))
            }
            _ => None,
        }) else {
            continue;
        };
        // A chat restored after the turn began already shows what was asked; one that was open
        // when a phone asked something does not.
        let last_asked = chat
            .model
            .entries
            .iter()
            .rev()
            .find(|entry| entry.kind == Kind::User)
            .map(|entry| entry.text.trim().to_owned());
        if last_asked.as_deref() != Some(prompt.trim()) {
            chat.model.add_user(prompt);
        }
        chat.turn = Some(Turn::attach_to_node(endpoint.clone(), job, label));
        chat.turn_started = Some(std::time::Instant::now());
        chat.turn_replies.clear();
        super::prepare::sync(chat);
    }
}
