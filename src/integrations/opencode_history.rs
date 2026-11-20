//! Read bounded, project-scoped OpenCode history through its supported CLI export.

use crate::integrations::{chat_history, executable_search, mcp_clients};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

const MAX_SESSIONS: usize = 10;
const MAX_MESSAGES_PER_SESSION: usize = 200;
const MAX_MESSAGE_CHARS: usize = 16_000;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const COMMAND_OUTPUT_LIMIT: usize = 4 * 1024 * 1024;

/// Return user-only messages from recent OpenCode sessions belonging to this project.
pub fn recent_user_conversations(project: &Path) -> Vec<chat_history::Conversation> {
    let Some(executable) = executable_search::find_executable("opencode") else {
        return Vec::new();
    };
    recent_user_conversations_with(project, &executable, MAX_SESSIONS, run_json)
}

fn recent_user_conversations_with(
    project: &Path,
    executable: &Path,
    limit: usize,
    mut run: impl FnMut(&[String], &Path) -> Option<Value>,
) -> Vec<chat_history::Conversation> {
    let Ok(root) = project.canonicalize() else {
        return Vec::new();
    };
    if !root.is_dir() {
        return Vec::new();
    }
    let count = limit.min(MAX_SESSIONS);
    if count == 0 {
        return Vec::new();
    }
    let binary = executable.to_string_lossy().into_owned();
    let list = command(
        &binary,
        [
            "session",
            "list",
            "--max-count",
            &count.to_string(),
            "--format",
            "json",
        ],
    );
    let Some(sessions) = run(&list, &root).and_then(|value| value.as_array().cloned()) else {
        return Vec::new();
    };

    let mut conversations = Vec::new();
    for session in sessions.iter().take(count) {
        let Some(session_id) = session.get("id").and_then(Value::as_str) else {
            continue;
        };
        if !valid_session_id(session_id) {
            continue;
        }
        let Some(directory) = session.get("directory").and_then(Value::as_str) else {
            continue;
        };
        if !belongs_to_project(directory, &root) {
            continue;
        }
        let Some(exported) = export_session(&binary, session_id, &root, &mut run) else {
            continue;
        };
        if let Some(exported_directory) = exported_directory(&exported)
            && !belongs_to_project(exported_directory, &root)
        {
            continue;
        }
        let messages = exported
            .get("messages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(user_message)
            .take(MAX_MESSAGES_PER_SESSION)
            .map(|text| chat_history::ConversationMessage {
                role: "user".to_owned(),
                text: text.chars().take(MAX_MESSAGE_CHARS).collect(),
            })
            .collect::<Vec<_>>();
        if messages.is_empty() {
            continue;
        }
        let title = messages[0]
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        conversations.push(chat_history::Conversation {
            provider: "OpenCode".to_owned(),
            session_id: session_id.to_owned(),
            title: title.chars().take(90).collect(),
            modified_at: updated_at(session),
            project_root: root.clone(),
            // OpenCode owns its session database; there is no project transcript file to expose.
            transcript_path: PathBuf::new(),
            messages,
        });
    }
    conversations
}

fn export_session(
    binary: &str,
    session_id: &str,
    root: &Path,
    run: &mut impl FnMut(&[String], &Path) -> Option<Value>,
) -> Option<Value> {
    for arguments in [
        vec!["session", "export", session_id, "--sanitize"],
        vec!["export", session_id, "--sanitize"],
    ] {
        let command = command(binary, arguments);
        let Some(exported) = run(&command, root) else {
            continue;
        };
        if exported.get("messages").and_then(Value::as_array).is_some() {
            return Some(exported);
        }
    }
    None
}

fn command(binary: &str, arguments: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    std::iter::once(binary.to_owned())
        .chain(arguments.into_iter().map(|part| part.as_ref().to_owned()))
        .collect()
}

fn valid_session_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn belongs_to_project(directory: &str, project: &Path) -> bool {
    let path = expand_home(directory);
    path.canonicalize().is_ok_and(|path| path == project)
}

fn expand_home(directory: &str) -> PathBuf {
    let Some(home) = std::env::home_dir() else {
        return PathBuf::from(directory);
    };
    if directory == "~" {
        home
    } else if let Some(rest) = directory
        .strip_prefix("~/")
        .or_else(|| directory.strip_prefix("~\\"))
    {
        home.join(rest)
    } else {
        PathBuf::from(directory)
    }
}

fn exported_directory(exported: &Value) -> Option<&str> {
    let info = exported.get("info")?;
    info.get("directory").and_then(Value::as_str).or_else(|| {
        info.get("location")
            .and_then(|location| location.get("directory"))
            .and_then(Value::as_str)
    })
}

fn user_message(message: &Value) -> Option<String> {
    let role = message
        .get("info")
        .and_then(|info| info.get("role"))
        .and_then(Value::as_str);
    if role == Some("user") {
        return joined_text(message.get("parts")?.as_array()?);
    }
    if message.get("type").and_then(Value::as_str) == Some("user") {
        return message
            .get("text")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    None
}

fn joined_text(parts: &[Value]) -> Option<String> {
    let text = parts
        .iter()
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

fn updated_at(session: &Value) -> SystemTime {
    let updated = session
        .get("time")
        .and_then(|time| time.get("updated"))
        .filter(|value| !value.is_null())
        .or_else(|| session.get("updated"));
    let Some(updated) = updated else {
        return SystemTime::now();
    };
    let timestamp = if let Some(number) = updated.as_f64() {
        if number > 10_000_000_000.0 {
            number / 1000.0
        } else {
            number
        }
    } else if let Some(text) = updated.as_str() {
        let Ok(date) = OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
        else {
            return SystemTime::now();
        };
        return system_time(date.unix_timestamp());
    } else {
        return SystemTime::now();
    };
    if !timestamp.is_finite() || timestamp < i64::MIN as f64 || timestamp > i64::MAX as f64 {
        return SystemTime::now();
    }
    system_time(timestamp as i64)
}

fn system_time(seconds: i64) -> SystemTime {
    if seconds >= 0 {
        UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds as u64))
            .unwrap_or_else(SystemTime::now)
    } else {
        UNIX_EPOCH
            .checked_sub(Duration::from_secs(seconds.unsigned_abs()))
            .unwrap_or(UNIX_EPOCH)
    }
}

fn run_json(command: &[String], cwd: &Path) -> Option<Value> {
    let output =
        mcp_clients::run_with_limits(command, cwd, COMMAND_TIMEOUT, COMMAND_OUTPUT_LIMIT).ok()?;
    if output.status != 0 {
        return None;
    }
    serde_json::from_str(&output.stdout).ok()
}

#[cfg(test)]
#[path = "opencode_history_tests.rs"]
mod tests;
