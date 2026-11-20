//! Bounded, project-local summaries of completed Studio conversations.

use super::chat_state::{self, ChatState};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub const JOURNAL: &str = ".ai/history/SESSIONS.md";
const TITLE: &str = "# Sesiones recientes de agentes\n\n";
const MAX_ENTRIES: usize = 30;
const MAX_JOURNAL_BYTES: u64 = 256 * 1024;
const MAX_PROMPT_CHARS: usize = 8_000;
const REDACTED: &str = "(omitido: posible secreto)";

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionSummary {
    provider: String,
    session_id: String,
    date: String,
    opening: String,
    requests: Vec<String>,
    outcome: String,
}

/// Record the latest visible Studio conversation without reading a provider transcript.
pub fn record_latest_chat(project: &Path, agent_id: &str) -> Result<Option<PathBuf>, String> {
    let Some(state) = chat_state::load(project, agent_id) else {
        return Ok(None);
    };
    if state.session_id.is_empty() {
        return Ok(None);
    }
    let Some(summary) = summarize(&state) else {
        return Ok(None);
    };
    let path = writable_journal_path(project)?;
    let existing = read_journal(&path)?;
    let marker = marker(&summary.provider, &summary.session_id);
    let mut entries = entries(&existing)
        .into_iter()
        .filter(|entry| !entry.contains(&marker));
    let mut output = String::from(TITLE);
    output.push_str(&render(&summary, &marker));
    for entry in entries.by_ref().take(MAX_ENTRIES - 1) {
        if output.len().saturating_add(entry.len()) > MAX_JOURNAL_BYTES as usize {
            break;
        }
        output.push_str(&entry);
    }
    atomic_write(&path, &output)?;
    Ok(Some(path))
}

/// Include only the newest two bounded entries in a new conversation's first message.
pub fn recent_sessions_prompt(project: &Path) -> String {
    let Some(path) = readable_journal_path(project) else {
        return String::new();
    };
    let Ok(text) = read_journal(&path) else {
        return String::new();
    };
    let mut remaining = MAX_PROMPT_CHARS;
    let mut recent = Vec::new();
    for entry in entries(&text).into_iter().take(2) {
        let body = entry
            .lines()
            .filter(|line| !line.starts_with("<!-- session:") || !line.ends_with(" -->"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned();
        let body = clip(&body, remaining);
        remaining = remaining.saturating_sub(body.chars().count());
        if !body.is_empty() {
            recent.push(body);
        }
        if remaining == 0 {
            break;
        }
    }
    if recent.is_empty() {
        return String::new();
    }
    format!(
        "\n\nResumen automático de las últimas sesiones de agentes en este proyecto (`{JOURNAL}`). Úsalo para continuar; si contradice el handoff, prioriza el archivo más reciente y verifica en el código.\n{}",
        recent.join("\n\n")
    )
}

fn summarize(state: &ChatState) -> Option<SessionSummary> {
    let users: Vec<_> = state
        .messages
        .iter()
        .filter(|message| message.role == "user")
        .map(|message| &message.text)
        .collect();
    let opening = summary_text(users.first()?, 240);
    if opening.is_empty() {
        return None;
    }
    let requests = users
        .iter()
        .skip(1)
        .rev()
        .take(3)
        .rev()
        .map(|text| summary_text(text, 200))
        .filter(|text| !text.is_empty())
        .collect();
    let latest_user = state
        .messages
        .iter()
        .rposition(|message| message.role == "user")?;
    let outcome = state
        .messages
        .iter()
        .skip(latest_user + 1)
        .rev()
        .find(|message| message.role == "assistant")
        .map(|message| summary_text(&message.text, 600))
        .unwrap_or_default();
    let date = date_prefix(&state.updated);
    Some(SessionSummary {
        provider: state.provider.clone(),
        session_id: state.session_id.clone(),
        date,
        opening,
        requests,
        outcome,
    })
}

fn summary_text(text: &str, limit: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if chat_state::contains_secret(&text) {
        return REDACTED.to_owned();
    }
    clip(&text, limit)
}

fn date_prefix(updated: &str) -> String {
    let date: String = updated.chars().take(10).collect();
    let valid = date.len() == 10
        && date.as_bytes()[4] == b'-'
        && date.as_bytes()[7] == b'-'
        && date
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit());
    if valid {
        date
    } else {
        OffsetDateTime::now_utc().date().to_string()
    }
}

fn render(summary: &SessionSummary, marker: &str) -> String {
    let mut lines = vec![
        format!(
            "## {} · {} · `{}`",
            summary.date,
            safe_inline(&summary.provider),
            safe_inline(&summary.session_id)
        ),
        marker.to_owned(),
        String::new(),
        format!("- Inicio: {}", summary.opening),
    ];
    if !summary.requests.is_empty() {
        lines.push("- Últimos pedidos:".to_owned());
        lines.extend(
            summary
                .requests
                .iter()
                .map(|request| format!("  - {request}")),
        );
    }
    if !summary.outcome.is_empty() {
        lines.push(format!("- Cierre del agente: {}", summary.outcome));
    }
    lines.join("\n") + "\n\n"
}

fn safe_inline(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .map(|character| match character {
            '`' | '[' | ']' | '\\' => ' ',
            character if character.is_control() => ' ',
            character => character,
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn marker(provider: &str, session_id: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(provider.as_bytes());
    hash.update([0]);
    hash.update(session_id.as_bytes());
    let digest = hash.finalize();
    let short = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("<!-- session:{short} -->")
}

fn entries(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current = None::<String>;
    for line in text.split_inclusive('\n') {
        if line.starts_with("## ") {
            if let Some(entry) = current.replace(line.to_owned()) {
                found.push(entry);
            }
        } else if let Some(entry) = &mut current {
            entry.push_str(line);
        }
    }
    if let Some(entry) = current {
        found.push(entry);
    }
    found
}

fn readable_journal_path(project: &Path) -> Option<PathBuf> {
    let root = project.canonicalize().ok()?;
    let ai = root.join(".ai");
    if !regular_directory(&ai, false) {
        return None;
    }
    let history = ai.join("history");
    if !regular_directory(&history, false) {
        return None;
    }
    let path = history.join("SESSIONS.md");
    fs::symlink_metadata(&path)
        .ok()
        .filter(|metadata| metadata.file_type().is_file())
        .map(|_| path)
}

fn writable_journal_path(project: &Path) -> Result<PathBuf, String> {
    let root = project
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver la raíz del proyecto: {error}"))?;
    let ai = root.join(".ai");
    ensure_directory(&ai)?;
    let history = ai.join("history");
    ensure_directory(&history)?;
    let path = history.join("SESSIONS.md");
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err("No se reemplazó el diario porque no es un archivo regular".to_owned());
        }
        Ok(_) | Err(_) => {}
    }
    Ok(path)
}

fn regular_directory(path: &Path, create: bool) -> bool {
    match fs::symlink_metadata(path) {
        Ok(metadata) => metadata.is_dir() && !metadata.file_type().is_symlink(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
            fs::create_dir(path).is_ok() && regular_directory(path, false)
        }
        _ => false,
    }
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(format!("{} no es una carpeta segura", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|error| format!("No se pudo crear {}: {error}", path.display()))?;
            ensure_directory(path)
        }
        Err(error) => Err(format!(
            "No se pudo inspeccionar {}: {error}",
            path.display()
        )),
    }
}

fn read_journal(path: &Path) -> Result<String, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("No se pudo inspeccionar el diario: {error}")),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err("El diario no es un archivo regular".to_owned())
        }
        Ok(metadata) if metadata.len() > MAX_JOURNAL_BYTES => {
            Err("El diario supera el límite de lectura".to_owned())
        }
        Ok(_) => fs::read(path)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|error| format!("No se pudo leer el diario: {error}")),
    }
}

fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta del diario".to_owned())?;
    let temporary = parent.join(format!(
        ".sessions-{}-{}.tmp",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("No se pudo preparar el diario temporal: {error}"))?;
    if let Err(error) = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
    {
        let _ = fs::remove_file(&temporary);
        return Err(format!("No se pudo escribir el diario temporal: {error}"));
    }
    drop(file);
    let backup = if path.exists() {
        let backup = parent.join(format!(
            ".sessions-backup-{}-{}.tmp",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        if let Err(error) = fs::rename(path, &backup) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("No se pudo conservar el diario previo: {error}"));
        }
        Some(backup)
    } else {
        None
    };
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        if let Some(backup) = &backup {
            let _ = fs::rename(backup, path);
        }
        return Err(format!("No se pudo guardar el diario: {error}"));
    }
    if let Some(backup) = backup {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn clip(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut clipped: String = text.chars().take(limit.saturating_sub(1)).collect();
    clipped.push('…');
    clipped
}

#[cfg(test)]
#[path = "chat_journal_tests.rs"]
mod tests;
