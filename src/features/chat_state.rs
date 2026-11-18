//! Project-scoped persistence for Studio conversations.
//!
//! Only user and assistant text is kept. Tool cards and provider payloads are omitted, messages
//! are bounded, and text that resembles a credential is replaced before it reaches disk.
use serde::Serialize;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub const SCHEMA: &str = "chxchx.chat-state";
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_MESSAGES: usize = 80;
pub const MAX_MESSAGE_CHARS: usize = 8_000;
const OMITTED: &str = "(omitido: posible secreto)";
const RECAP_MESSAGES: usize = 24;
const RECAP_CHARS: usize = 14_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Message {
    pub role: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ChatState {
    pub schema: String,
    pub schema_version: u32,
    pub agent: String,
    pub provider: String,
    pub session_id: String,
    pub updated: String,
    pub verified: bool,
    pub messages: Vec<Message>,
}

pub fn state_path(project: &Path, agent_id: &str) -> PathBuf {
    let safe: String = agent_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let safe = if safe.is_empty() { "agent" } else { &safe };
    project
        .join(".ai")
        .join("chat-state")
        .join(format!("{safe}.json"))
}

/// Reads only valid conversational messages. Missing, malformed, and partially written state is
/// treated as an empty conversation, matching the legacy recovery behavior.
pub fn load(project: &Path, agent_id: &str) -> Option<ChatState> {
    let bytes = fs::read(state_path(project, agent_id)).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    let object = value.as_object()?;
    let messages = object.get("messages")?.as_array()?;
    let messages = messages
        .iter()
        .filter_map(|message| {
            let role = message.get("role")?.as_str()?;
            let text = message.get("text")?.as_str()?;
            matches!(role, "user" | "assistant").then(|| Message {
                role: role.to_owned(),
                text: text.to_owned(),
            })
        })
        .collect();
    Some(ChatState {
        schema: string_at(object, "schema"),
        schema_version: object
            .get("schema_version")
            .and_then(Value::as_u64)
            .and_then(|version| u32::try_from(version).ok())
            .unwrap_or_default(),
        agent: string_at(object, "agent"),
        provider: string_at(object, "provider"),
        session_id: string_at(object, "session_id"),
        updated: string_at(object, "updated"),
        verified: object
            .get("verified")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        messages,
    })
}

fn string_at(object: &serde_json::Map<String, Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Returns the Studio contract and marks a restored session for a one-time provider check.
pub fn payload(project: &Path, agent_id: &str, restoring: bool) -> ChatState {
    if restoring {
        let _ = mark_unverified(project, agent_id);
    }
    let state = load(project, agent_id).unwrap_or_default();
    ChatState {
        schema: SCHEMA.to_owned(),
        schema_version: SCHEMA_VERSION,
        agent: agent_id.to_owned(),
        provider: state.provider,
        session_id: state.session_id,
        updated: state.updated,
        verified: state.verified,
        messages: state.messages,
    }
}

/// Versioned public shape used by `agent chat-state`; internal verification state stays private.
pub fn contract(project: &Path, agent_id: &str, restoring: bool) -> serde_json::Value {
    let state = payload(project, agent_id, restoring);
    serde_json::json!({
        "schema": state.schema,
        "schema_version": state.schema_version,
        "agent": state.agent,
        "provider": state.provider,
        "session_id": state.session_id,
        "updated": state.updated,
        "messages": state.messages,
    })
}

pub fn reset(project: &Path, agent_id: &str) {
    let _ = fs::remove_file(state_path(project, agent_id));
}

pub fn mark_unverified(project: &Path, agent_id: &str) -> Result<(), String> {
    let Some(mut state) = load(project, agent_id) else {
        return Ok(());
    };
    if state.verified {
        state.verified = false;
        write(project, agent_id, &state)?;
    }
    Ok(())
}

/// Saves the user's message before the provider starts, so closing Studio mid-turn keeps it.
pub fn begin_turn(
    project: &Path,
    agent_id: &str,
    provider: &str,
    session_id: Option<&str>,
    user_text: &str,
) -> Result<(), String> {
    record(
        project,
        agent_id,
        provider,
        session_id,
        [("user", user_text)],
    )
}

pub fn end_turn<'a>(
    project: &Path,
    agent_id: &str,
    provider: &str,
    session_id: Option<&str>,
    replies: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    record(
        project,
        agent_id,
        provider,
        session_id,
        replies.into_iter().map(|text| ("assistant", text)),
    )
}

fn record<'a>(
    project: &Path,
    agent_id: &str,
    provider: &str,
    session_id: Option<&str>,
    additions: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), String> {
    let old = load(project, agent_id).unwrap_or_default();
    let mut messages = old.messages;
    messages.extend(
        additions
            .into_iter()
            .filter(|(_, text)| !text.trim().is_empty())
            .map(|(role, text)| Message {
                role: role.to_owned(),
                text: clean(text),
            }),
    );
    let updated = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .map_err(|error| error.to_string())?
        .format(&Rfc3339)
        .map_err(|error| error.to_string())?;
    let state = ChatState {
        schema: SCHEMA.to_owned(),
        schema_version: SCHEMA_VERSION,
        agent: agent_id.to_owned(),
        provider: provider.to_owned(),
        session_id: session_id
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .unwrap_or(old.session_id),
        updated,
        verified: true,
        messages: messages
            .into_iter()
            .rev()
            .take(MAX_MESSAGES)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
    };
    write(project, agent_id, &state)
}

fn write(project: &Path, agent_id: &str, state: &ChatState) -> Result<(), String> {
    let target = state_path(project, agent_id);
    let parent = target
        .parent()
        .ok_or_else(|| "La ruta del chat no tiene carpeta".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.{}.tmp",
        target.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(&mut file, state).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        fs::rename(&temporary, &target).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn clean(text: &str) -> String {
    let text = text.trim();
    if contains_secret(text) {
        return OMITTED.to_owned();
    }
    let mut chars = text.chars();
    let clipped: String = chars.by_ref().take(MAX_MESSAGE_CHARS).collect();
    if chars.next().is_some() {
        format!("{clipped}…")
    } else {
        clipped
    }
}

/// Mirrors the legacy detector's conservative credential markers.
pub fn contains_secret(text: &str) -> bool {
    let lower = text.to_lowercase();
    let normalized: String = lower
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    if [
        "password",
        "contraseña",
        "token",
        "api key",
        "secret",
        "secreto",
        "private key",
        "clave privada",
    ]
    .iter()
    .any(|marker| contains_phrase(&normalized, marker))
    {
        return true;
    }
    let bytes = text.as_bytes();
    bytes.windows(3).enumerate().any(|(offset, marker)| {
        marker.eq_ignore_ascii_case(b"sk-")
            && bytes[offset + 3..]
                .iter()
                .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                .count()
                >= 16
    })
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    text == phrase
        || text.starts_with(&format!("{phrase} "))
        || text.ends_with(&format!(" {phrase}"))
        || text.contains(&format!(" {phrase} "))
}

/// Rebuilds a bounded context when the provider has discarded a restored session.
pub fn recap_block(state: &ChatState) -> String {
    let mut lines = Vec::new();
    let mut budget = RECAP_CHARS;
    for message in state.messages.iter().rev().take(RECAP_MESSAGES) {
        let who = if message.role == "user" {
            "Persona"
        } else {
            "Agente"
        };
        let clipped = clip(&message.text, 2_000.min(budget.max(400)));
        budget = budget.saturating_sub(clipped.chars().count());
        lines.push(format!("{who}: {clipped}"));
        if budget == 0 {
            break;
        }
    }
    lines.reverse();
    format!(
        "\n\nEsta conversación continúa de una sesión anterior que el agente ya no conserva. Retómala desde el historial de abajo como si nada hubiera cambiado: no lo menciones, no pidas confirmación ni repitas lo ya hecho, y responde al último mensaje de la persona.\n<archivo-de-contexto ruta=\"conversación anterior\">\n{}\n</archivo-de-contexto>",
        lines.join("\n\n")
    )
}

fn clip(text: &str, limit: usize) -> String {
    let length = text.chars().count();
    if length <= limit {
        return text.to_owned();
    }
    let tail = 300.min(limit / 3);
    let head_len = limit.saturating_sub(tail + 3);
    let head: String = text.chars().take(head_len).collect();
    let ending: String = text.chars().skip(length - tail).collect();
    format!("{head}…\n…{ending}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "chxchx-chat-state-{name}-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn turn_messages_are_saved_bounded_and_secrets_are_omitted() {
        let root = project("bounds");
        for index in 0..100 {
            begin_turn(
                &root,
                "claude",
                "Claude",
                Some("s-1"),
                &format!("mensaje {index}"),
            )
            .unwrap();
        }
        end_turn(
            &root,
            "claude",
            "Claude",
            Some("s-1"),
            ["x".repeat(MAX_MESSAGE_CHARS + 1).as_str()],
        )
        .unwrap();
        begin_turn(
            &root,
            "claude",
            "Claude",
            Some("s-1"),
            "mi clave es sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD",
        )
        .unwrap();

        let state = load(&root, "claude").unwrap();
        assert_eq!(state.messages.len(), MAX_MESSAGES);
        assert_eq!(state.messages[0].text, "mensaje 22");
        assert_eq!(
            state.messages[MAX_MESSAGES - 2].text.chars().count(),
            MAX_MESSAGE_CHARS + 1
        );
        assert_eq!(state.messages[MAX_MESSAGES - 1].text, OMITTED);
        assert!(
            !fs::read_to_string(state_path(&root, "claude"))
                .unwrap()
                .contains("sk-")
        );
    }

    #[test]
    fn provider_and_session_survive_turn_boundaries_and_restore_marks_unverified() {
        let root = project("restore");
        begin_turn(&root, "codex", "Codex", None, "pregunta").unwrap();
        end_turn(&root, "codex", "Codex", Some("thread-1"), ["respuesta"]).unwrap();

        let restored = payload(&root, "codex", true);
        assert_eq!(restored.schema, SCHEMA);
        assert_eq!(restored.session_id, "thread-1");
        assert_eq!(restored.provider, "Codex");
        assert!(!load(&root, "codex").unwrap().verified);
        assert_eq!(restored.messages.len(), 2);
    }

    #[test]
    fn reset_and_corrupt_files_are_recoverable() {
        let root = project("broken");
        let path = state_path(&root, "a/b");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ roto").unwrap();
        assert!(load(&root, "a/b").is_none());
        assert!(payload(&root, "a/b", false).messages.is_empty());

        fs::write(
            &path,
            r#"{"messages":[{"role":"user","text":"conservar"},17,{"role":"tool","text":"descartar"},{"role":"assistant","text":false}]}"#,
        )
        .unwrap();
        assert_eq!(
            load(&root, "a/b").unwrap().messages,
            [Message {
                role: "user".into(),
                text: "conservar".into()
            }]
        );

        begin_turn(&root, "a/b", "Agente", None, "nuevo turno").unwrap();
        assert!(state_path(&root, "a/b").is_file());
        reset(&root, "a/b");
        assert!(!state_path(&root, "a/b").exists());
    }

    #[test]
    fn recap_preserves_order_and_limits_each_message() {
        let state = ChatState {
            messages: vec![
                Message {
                    role: "user".into(),
                    text: "migrar".into(),
                },
                Message {
                    role: "assistant".into(),
                    text: "x".repeat(5_000),
                },
            ],
            ..Default::default()
        };
        let recap = recap_block(&state);
        assert!(recap.find("Persona: migrar").unwrap() < recap.find("Agente:").unwrap());
        assert!(recap.contains("…\n…"));
        assert!(recap.ends_with("</archivo-de-contexto>"));
    }
}
