//! Verify a restored provider session before resuming it; recover the visible chat if it is gone.

use crate::features::chat_state;
use crate::integrations::chat_history;
use serde_json::Value;
use std::path::Path;

pub fn resolve_resume(
    project: &Path,
    provider: &str,
    agent_id: &str,
    session_id: Option<&str>,
    executable: &str,
) -> (Option<String>, String) {
    resolve_resume_with(
        project,
        provider,
        agent_id,
        session_id,
        |project, provider, session| session_exists(project, provider, session, executable),
    )
}

fn resolve_resume_with(
    project: &Path,
    provider: &str,
    agent_id: &str,
    session_id: Option<&str>,
    mut exists: impl FnMut(&Path, &str, &str) -> Option<bool>,
) -> (Option<String>, String) {
    let Some(session_id) = session_id.filter(|session| !session.is_empty()) else {
        return (None, String::new());
    };
    let Some(state) = chat_state::load(project, agent_id) else {
        return (Some(session_id.to_owned()), String::new());
    };
    if state.session_id != session_id || state.verified {
        return (Some(session_id.to_owned()), String::new());
    }
    if exists(project, provider, session_id) == Some(false) && !state.messages.is_empty() {
        return (None, chat_state::recap_block(&state));
    }
    (Some(session_id.to_owned()), String::new())
}

pub fn session_exists(
    project: &Path,
    provider: &str,
    session_id: &str,
    executable: &str,
) -> Option<bool> {
    match provider {
        "Codex" | "Claude" => chat_history::session_exists(project, provider, session_id),
        "OpenCode" => opencode_session_exists(project, session_id, executable),
        _ => None,
    }
}

fn opencode_session_exists(project: &Path, session_id: &str, executable: &str) -> Option<bool> {
    if executable.trim().is_empty() {
        return None;
    }
    let command = vec![
        executable.to_owned(),
        "session".into(),
        "list".into(),
        "--max-count".into(),
        "200".into(),
        "--format".into(),
        "json".into(),
    ];
    let output = crate::integrations::mcp_clients::run(&command, project).ok()?;
    if output.status != 0 {
        return None;
    }
    let sessions: Value = serde_json::from_str(&output.stdout).ok()?;
    let sessions = sessions.as_array()?;
    Some(
        sessions
            .iter()
            .any(|session| session.get("id").and_then(Value::as_str) == Some(session_id)),
    )
}

#[cfg(test)]
#[path = "chat_resume_tests.rs"]
mod tests;
