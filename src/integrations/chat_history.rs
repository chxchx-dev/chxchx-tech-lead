//! Read local Codex and Claude transcripts for one project.
//!
//! Provider histories are user data. This adapter only reads regular JSONL files under the
//! configured provider directories, verifies the transcript's project path, and bounds discovery,
//! file reads, message count, and message size.

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_TRANSCRIPTS_PER_PROVIDER: usize = 500;
const MAX_DISCOVERED_FILES: usize = 20_000;
const MAX_DIRECTORY_DEPTH: usize = 12;
const MAX_TRANSCRIPT_BYTES: u64 = 32 * 1024 * 1024;
const MAX_METADATA_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_MESSAGES: usize = 2_000;
const MAX_MESSAGE_CHARS: usize = 16_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    pub provider: String,
    pub session_id: String,
    pub title: String,
    pub modified_at: SystemTime,
    pub project_root: PathBuf,
    pub transcript_path: PathBuf,
    pub messages: Vec<ConversationMessage>,
}

impl Conversation {
    pub fn preview(&self) -> String {
        self.messages
            .iter()
            .rev()
            .find(|message| message.role == "user")
            .map(|message| truncate_text(&message.text.replace('\n', " "), 180))
            .unwrap_or_default()
    }
}

pub fn list_conversations(project_root: &Path, query: &str, limit: usize) -> Vec<Conversation> {
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".codex"));
    let claude_config = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".claude"));
    list_conversations_in(project_root, &codex_home, &claude_config, query, limit)
}

pub fn session_exists(project_root: &Path, provider: &str, session_id: &str) -> Option<bool> {
    if !matches!(provider, "Codex" | "Claude") {
        return None;
    }
    Some(
        list_conversations(project_root, "", MAX_TRANSCRIPTS_PER_PROVIDER * 2)
            .iter()
            .any(|conversation| {
                conversation.provider == provider && conversation.session_id == session_id
            }),
    )
}

/// Explicit provider roots keep transcript discovery deterministic in tests and callers.
pub fn list_conversations_in(
    project_root: &Path,
    codex_home: &Path,
    claude_config_dir: &Path,
    query: &str,
    limit: usize,
) -> Vec<Conversation> {
    let Ok(project_root) = project_root.canonicalize() else {
        return Vec::new();
    };
    let mut conversations = codex_conversations(&project_root, codex_home);
    conversations.extend(claude_conversations(&project_root, claude_config_dir));
    let query = query.trim().to_lowercase();
    if !query.is_empty() {
        conversations.retain(|conversation| searchable_text(conversation).contains(&query));
    }
    conversations.sort_by_key(|conversation| std::cmp::Reverse(conversation.modified_at));
    conversations.truncate(limit);
    conversations
}

fn codex_conversations(project_root: &Path, codex_home: &Path) -> Vec<Conversation> {
    let sessions = codex_home.join("sessions");
    if !regular_directory(&sessions) {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    collect_jsonl(&sessions, 0, &mut candidates);
    candidates.retain(|(path, _)| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"))
    });
    candidates.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    candidates
        .into_iter()
        .take(MAX_TRANSCRIPTS_PER_PROVIDER)
        .filter_map(|(path, modified_at)| {
            let (cwd, session_id) = codex_metadata(&path)?;
            if !same_path(Path::new(&cwd), project_root) {
                return None;
            }
            read_codex(&path, project_root, &session_id, modified_at)
        })
        .collect()
}

fn codex_metadata(path: &Path) -> Option<(String, String)> {
    let fallback_id = path
        .file_stem()?
        .to_str()?
        .strip_prefix("rollout-")?
        .to_owned();
    let mut metadata = None;
    for_each_jsonl(path, MAX_METADATA_BYTES, |record| {
        if metadata.is_some() || record.get("type").and_then(Value::as_str) != Some("session_meta")
        {
            return;
        }
        let payload = record.get("payload").and_then(Value::as_object);
        let Some(cwd) = payload
            .and_then(|payload| payload.get("cwd"))
            .and_then(Value::as_str)
        else {
            return;
        };
        let session_id = payload
            .and_then(|payload| payload.get("id"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(&fallback_id)
            .to_owned();
        metadata = Some((cwd.to_owned(), session_id));
    });
    metadata
}

fn read_codex(
    path: &Path,
    project_root: &Path,
    session_id: &str,
    modified_at: SystemTime,
) -> Option<Conversation> {
    let mut messages = Vec::new();
    for_each_jsonl(path, MAX_TRANSCRIPT_BYTES, |record| {
        if messages.len() >= MAX_MESSAGES {
            return;
        }
        let Some(payload) = record.get("payload").and_then(Value::as_object) else {
            return;
        };
        let record_type = record
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if record_type == "event_msg" {
            let kind = payload
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let role = match kind {
                "user_message" => "user",
                "agent_message" | "assistant_message" => "assistant",
                _ => return,
            };
            append_message(&mut messages, role, payload.get("message"));
        } else if record_type == "response_item" {
            let item = payload
                .get("item")
                .and_then(Value::as_object)
                .unwrap_or(payload);
            let role = item.get("role").and_then(Value::as_str).unwrap_or_default();
            if item.get("type").and_then(Value::as_str) == Some("message")
                && matches!(role, "user" | "assistant")
            {
                append_message(&mut messages, role, item.get("content"));
            }
        }
    });
    conversation(
        "Codex",
        session_id,
        path,
        project_root,
        modified_at,
        messages,
    )
}

fn claude_conversations(project_root: &Path, config_dir: &Path) -> Vec<Conversation> {
    let mut files = Vec::new();
    for folder in claude_project_dirs(config_dir, project_root) {
        collect_jsonl(&folder, 0, &mut files);
    }
    files.retain(|(path, _)| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"));
    files.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    files
        .into_iter()
        .take(MAX_TRANSCRIPTS_PER_PROVIDER)
        .filter_map(|(path, modified_at)| read_claude(&path, project_root, modified_at))
        .collect()
}

fn claude_project_dirs(config_dir: &Path, project_root: &Path) -> Vec<PathBuf> {
    let projects = config_dir.join("projects");
    if !regular_directory(&projects) {
        return Vec::new();
    }
    let raw = project_root.to_string_lossy();
    let sanitized = raw
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let legacy = raw.replace(['\\', '/', ':'], "-");
    let mut names = vec![sanitized.clone(), legacy];
    if sanitized.len() > 200 {
        let prefix = &sanitized[..200];
        if let Ok(entries) = fs::read_dir(&projects) {
            names.extend(entries.flatten().filter_map(|entry| {
                let name = entry.file_name();
                let name = name.to_str()?;
                name.starts_with(&format!("{prefix}-"))
                    .then(|| name.to_owned())
            }));
        }
    }
    let mut folders = Vec::new();
    for name in names {
        let path = projects.join(name);
        if regular_directory(&path) && !folders.contains(&path) {
            folders.push(path);
        }
    }
    folders
}

fn read_claude(path: &Path, project_root: &Path, modified_at: SystemTime) -> Option<Conversation> {
    let mut messages = Vec::new();
    let mut cwd_checked = false;
    let mut custom_title = None;
    let mut belongs_to_project = true;
    for_each_jsonl(path, MAX_TRANSCRIPT_BYTES, |record| {
        if !belongs_to_project || messages.len() >= MAX_MESSAGES {
            return;
        }
        if !cwd_checked && let Some(cwd) = record.get("cwd").and_then(Value::as_str) {
            cwd_checked = true;
            belongs_to_project = same_path(Path::new(cwd), project_root);
            if !belongs_to_project {
                return;
            }
        }
        if record.get("type").and_then(Value::as_str) == Some("custom-title") {
            let title = record
                .get("customTitle")
                .and_then(Value::as_str)
                .filter(|title| !title.is_empty())
                .or_else(|| {
                    record
                        .get("title")
                        .and_then(Value::as_str)
                        .filter(|title| !title.is_empty())
                });
            if let Some(title) = title {
                custom_title = Some(title.to_owned());
            }
            return;
        }
        if record.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            return;
        }
        let kind = record
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(kind, "user" | "assistant") {
            return;
        }
        let message = record.get("message");
        let role = message
            .and_then(|message| message.get("role"))
            .and_then(Value::as_str)
            .unwrap_or(kind);
        if !matches!(role, "user" | "assistant") {
            return;
        }
        let content = message
            .and_then(|message| message.get("content"))
            .or(message);
        append_message(&mut messages, role, content);
    });
    if !belongs_to_project {
        return None;
    }
    let session_id = path.file_stem()?.to_str()?;
    let title = custom_title
        .or_else(|| {
            messages
                .iter()
                .find(|message| message.role == "user")
                .map(|message| message.text.clone())
        })
        .unwrap_or_else(|| "Chat Claude".to_owned());
    conversation(
        "Claude",
        session_id,
        path,
        project_root,
        modified_at,
        messages,
    )
    .map(|mut conversation| {
        conversation.title = compact(&title, 90);
        conversation
    })
}

fn conversation(
    provider: &str,
    session_id: &str,
    path: &Path,
    project_root: &Path,
    modified_at: SystemTime,
    messages: Vec<ConversationMessage>,
) -> Option<Conversation> {
    if messages.is_empty() {
        return None;
    }
    let title = messages
        .iter()
        .find(|message| message.role == "user")
        .map(|message| message.text.clone())
        .unwrap_or_else(|| format!("Chat {provider}"));
    Some(Conversation {
        provider: provider.to_owned(),
        session_id: session_id.to_owned(),
        title: compact(&title, 90),
        modified_at,
        project_root: project_root.to_owned(),
        transcript_path: path.to_owned(),
        messages,
    })
}

fn append_message(messages: &mut Vec<ConversationMessage>, role: &str, content: Option<&Value>) {
    if messages.len() >= MAX_MESSAGES {
        return;
    }
    let Some(content) = content else { return };
    let text = match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| match part {
                Value::String(text) => Some(text.clone()),
                Value::Object(item) => item
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| {
                        (item.get("type").and_then(Value::as_str) == Some("tool_use"))
                            .then(|| item.get("name").and_then(Value::as_str))
                            .flatten()
                            .map(|name| format!("[Herramienta: {name}]"))
                    }),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return,
    };
    let text = truncate_text(text.trim(), MAX_MESSAGE_CHARS);
    if text.is_empty()
        || messages
            .last()
            .is_some_and(|last| last.role == role && last.text == text)
    {
        return;
    }
    messages.push(ConversationMessage {
        role: role.to_owned(),
        text,
    });
}

fn for_each_jsonl(path: &Path, byte_limit: u64, mut visit: impl FnMut(Value)) {
    let Ok(file) = fs::File::open(path) else {
        return;
    };
    let mut reader = BufReader::new(file.take(byte_limit));
    let mut line = Vec::new();
    loop {
        line.clear();
        let Ok(bytes) = reader.read_until(b'\n', &mut line) else {
            break;
        };
        if bytes == 0 {
            break;
        }
        if line.len() > MAX_RECORD_BYTES {
            continue;
        }
        if let Ok(value) = serde_json::from_slice::<Value>(&line) {
            visit(value);
        }
    }
}

fn collect_jsonl(directory: &Path, depth: usize, files: &mut Vec<(PathBuf, SystemTime)>) {
    if depth > MAX_DIRECTORY_DEPTH || files.len() >= MAX_DISCOVERED_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if files.len() >= MAX_DISCOVERED_FILES {
            break;
        }
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        let kind = metadata.file_type();
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect_jsonl(&path, depth + 1, files);
        } else if kind.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("jsonl")
        {
            files.push((path, metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)));
        }
    }
}

fn regular_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

fn same_path(raw_path: &Path, project_root: &Path) -> bool {
    raw_path
        .canonicalize()
        .is_ok_and(|path| path == project_root)
}

fn searchable_text(conversation: &Conversation) -> String {
    let mut text = format!(
        "{}\n{}\n{}",
        conversation.title,
        conversation.preview(),
        conversation.provider
    );
    for message in &conversation.messages {
        text.push('\n');
        text.push_str(&message.text);
    }
    text.to_lowercase()
}

fn compact(value: &str, maximum: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_text(&normalized, maximum)
}

fn truncate_text(value: &str, maximum: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(maximum).collect::<String>();
    if chars.next().is_some() {
        let mut clipped = prefix
            .chars()
            .take(maximum.saturating_sub(1))
            .collect::<String>();
        clipped.push('…');
        clipped
    } else {
        prefix
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
#[path = "chat_history_tests.rs"]
mod tests;
