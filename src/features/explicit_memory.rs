//! Recover and persist only explicit, project-scoped requests to remember something.

use crate::features::chat_state::{self, ChatState};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;
use time::OffsetDateTime;

const MAX_MEMORIES: usize = 8;
const MAX_CHAT_STATE_FILES: usize = 10;
const MAX_CHAT_STATE_BYTES: u64 = 1024 * 1024;
const MAX_MEMORY_FILE_BYTES: u64 = 1024 * 1024;
const MAX_MESSAGE_CHARS: usize = 1_200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplicitMemory {
    pub provider: String,
    pub session_id: String,
    pub requested_at: String,
    pub message: String,
}

/// Recover explicit requests from bounded Rust-owned Studio transcripts.
pub fn recover_studio_memories(project: &Path) -> Vec<ExplicitMemory> {
    let root = project
        .canonicalize()
        .unwrap_or_else(|_| project.to_path_buf());
    let mut memories = Vec::new();
    let mut seen = HashSet::new();

    for state in studio_chat_states(&root) {
        for message in &state.messages {
            append_if_explicit(
                &mut memories,
                &mut seen,
                &state.provider,
                &state.session_id,
                &date_prefix(&state.updated),
                &message.role,
                &message.text,
            );
            if memories.len() == MAX_MEMORIES {
                return memories;
            }
        }
    }

    memories
}

/// Persist recovered requests once. Handwritten notes are preserved verbatim.
pub fn persist_explicit_memories(
    project: &Path,
    memories: &[ExplicitMemory],
) -> Result<usize, String> {
    if memories.is_empty() {
        return Ok(0);
    }
    let root = project
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver el proyecto: {error}"))?;
    let ai = root.join(".ai");
    ensure_directory(&ai)?;
    let memory_dir = ai.join("memory");
    ensure_directory(&memory_dir)?;
    let path = memory_dir.join("PROJECT_MEMORY.md");
    let mut existing = read_memory_file(&path)?;
    let mut additions = String::new();
    let mut seen = HashSet::new();
    let mut written = 0;

    for memory in memories.iter().take(MAX_MEMORIES) {
        let text = user_visible_text(&memory.message);
        if !is_explicit_request(&text) {
            continue;
        }
        let digest = digest(&text);
        if existing.contains(&format!(":{digest} -->")) || !seen.insert(digest.clone()) {
            continue;
        }
        let provider = safe_marker_segment(&memory.provider);
        let session = safe_marker_segment(&memory.session_id);
        additions.push_str(&format!(
            "\n### Petición explícita de memoria — {}\n\n<!-- explicit-memory:{provider}:{session}:{digest} -->\nOrigen: {}, conversación `{}`.\n\n> {text}\n",
            safe_date(&memory.requested_at),
            safe_inline(&memory.provider),
            safe_inline(&memory.session_id),
        ));
        existing.push_str(&format!(":{digest} -->"));
        written += 1;
    }

    if additions.is_empty() {
        return Ok(0);
    }
    let current = read_memory_file(&path)?;
    let new_size = current.len().saturating_add(additions.len());
    if new_size as u64 > MAX_MEMORY_FILE_BYTES {
        return Err("La memoria del proyecto superaría el límite de escritura".to_owned());
    }
    atomic_replace(&path, &format!("{current}{additions}"))?;
    Ok(written)
}

/// Persist memories before the context files are inlined, then add the exact requests to the prompt.
pub fn prepare_new_chat(project: &Path) -> String {
    let memories = recover_studio_memories(project);
    let _ = persist_explicit_memories(project, &memories);
    format_recovered_memories(&memories)
}

/// Studio owns memory capture when this is called after a Rust chat-state write.
pub fn capture_studio_memories(project: &Path) -> Result<usize, String> {
    let memories = recover_studio_memories(project);
    persist_explicit_memories(project, &memories)
}

pub fn format_recovered_memories(memories: &[ExplicitMemory]) -> String {
    if memories.is_empty() {
        return String::new();
    }
    let entries = memories
        .iter()
        .map(|memory| format!("- {}", user_visible_text(&memory.message)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n\nMemoria explícita recuperada de chats anteriores del proyecto actual. Trata estos mensajes como solicitudes literales del usuario; responde a la solicitud actual usando el contenido exacto cuando corresponda.\n{entries}"
    )
}

fn append_if_explicit(
    memories: &mut Vec<ExplicitMemory>,
    seen: &mut HashSet<String>,
    provider: &str,
    session_id: &str,
    requested_at: &str,
    role: &str,
    message: &str,
) {
    if role != "user" || memories.len() >= MAX_MEMORIES {
        return;
    }
    let message = user_visible_text(message);
    if !is_explicit_request(&message) {
        return;
    }
    let key = message.to_lowercase();
    if seen.insert(key) {
        memories.push(ExplicitMemory {
            provider: provider.to_owned(),
            session_id: session_id.to_owned(),
            requested_at: safe_date(requested_at),
            message,
        });
    }
}

fn studio_chat_states(project: &Path) -> Vec<ChatState> {
    let ai = project.join(".ai");
    let state_dir = ai.join("chat-state");
    if !regular_directory(&ai) || !regular_directory(&state_dir) {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(state_dir) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).ok()?;
            (metadata.file_type().is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("json")
                && metadata.len() <= MAX_CHAT_STATE_BYTES)
                .then(|| (path, metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)))
        })
        .collect::<Vec<_>>();
    paths.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    paths
        .into_iter()
        .take(MAX_CHAT_STATE_FILES)
        .filter_map(|(path, _)| {
            let agent = path.file_stem()?.to_str()?;
            chat_state::load(project, agent)
        })
        .collect()
}

fn is_explicit_request(text: &str) -> bool {
    if text.is_empty()
        || text.chars().count() > MAX_MESSAGE_CHARS
        || chat_state::contains_secret(text)
    {
        return false;
    }
    let lower = text.to_lowercase();
    if recovery_question(&lower) {
        return false;
    }
    let directives = [
        "recuerda",
        "recuérda",
        "recuérdalo",
        "recuérdala",
        "recuérdalos",
        "recuérdalas",
        "recordá",
        "acuérdate",
        "acuerdate",
        "memoriza",
        "memoríza",
        "ten presente",
        "no te olvides",
        "no olvides",
        "remember",
        "memorize",
        "keep in mind",
        "don't forget",
        "don’t forget",
        "do not forget",
    ];
    directives
        .iter()
        .any(|phrase| phrase_at_sentence_start(&lower, phrase))
        || has_spanish_memory_verb(&lower)
        || has_save_to_memory_directive(&lower)
        || [
            "quiero que te acuerdes",
            "quiero que recuerdes",
            "quiero que memorices",
            "quiero que tengas presente",
            "quiero que no olvides",
            "necesito que te acuerdes",
            "necesito que recuerdes",
            "necesito que memorices",
            "me gustaría que te acuerdes",
            "me gustaría que recuerdes",
            "i want you to remember",
            "i want you to memorize",
            "i want you to keep in mind",
            "i need you to remember",
            "i need you to memorize",
            "i need you to keep in mind",
        ]
        .iter()
        .any(|phrase| contains_phrase(&lower, phrase))
}

fn has_spanish_memory_verb(text: &str) -> bool {
    for verb in ["memoriz", "memoríz"] {
        let mut offset = 0;
        while let Some(index) = text[offset..].find(verb) {
            let start = offset + index;
            let after = start + verb.len();
            if is_boundary_before(text, start)
                && text[after..]
                    .chars()
                    .next()
                    .is_some_and(char::is_alphabetic)
                && sentence_start(text, start)
            {
                return true;
            }
            offset = after;
            if offset >= text.len() {
                break;
            }
        }
    }
    false
}

fn has_save_to_memory_directive(text: &str) -> bool {
    let mut tokens = Vec::new();
    let mut search_start = 0;
    for word in text.split_whitespace() {
        let Some(relative) = text[search_start..].find(word) else {
            continue;
        };
        let start = search_start + relative;
        tokens.push((
            start,
            word.trim_matches(|character: char| !character.is_alphanumeric())
                .to_lowercase(),
        ));
        search_start = start + word.len();
    }
    for (index, (start, verb)) in tokens.iter().enumerate() {
        if !["guarda", "guarde", "anota", "anote", "save", "store"].contains(&verb.as_str()) {
            continue;
        }
        if !sentence_start(text, *start) {
            continue;
        }
        let words = tokens
            .iter()
            .skip(index + 1)
            .take(4)
            .map(|(_, word)| word.as_str())
            .collect::<Vec<_>>();
        for (position, word) in words.iter().enumerate() {
            if !["en", "in", "to"].contains(word) {
                continue;
            }
            let mut memory_position = position + 1;
            if words
                .get(memory_position)
                .is_some_and(|next| ["la", "tu", "your"].contains(next))
            {
                memory_position += 1;
            }
            if words
                .get(memory_position)
                .is_some_and(|next| ["memoria", "memory"].contains(next))
            {
                return true;
            }
        }
    }
    false
}

fn recovery_question(text: &str) -> bool {
    [
        "recupera",
        "recuérdame",
        "recuerdame",
        "dime",
        "cuál era",
        "cual era",
        "qué te di",
        "que te di",
        "qué te dije",
        "que te dije",
        "cuál fue",
        "cual fue",
        "le puse que",
        "le dije que",
        "le pedí que",
        "le pedi que",
        "no se acuerda",
        "no recuerda",
        "no lo recuerda",
        "no la recuerda",
        "no recordó",
        "no recordo",
        "se olvidó",
        "se olvido",
        "olvidó",
        "olvido",
        "what was",
        "what i gave you",
        "forgot",
        "doesn't remember",
        "does not remember",
        "remind me",
        "retrieve",
        "recover",
    ]
    .iter()
    .any(|phrase| contains_phrase(text, phrase))
}

fn phrase_at_sentence_start(text: &str, phrase: &str) -> bool {
    let mut offset = 0;
    while let Some(index) = text[offset..].find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if is_boundary_before(text, start)
            && is_boundary_after(text, end)
            && sentence_start(text, start)
        {
            return true;
        }
        offset = end;
        if offset >= text.len() {
            break;
        }
    }
    false
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    let mut offset = 0;
    while let Some(index) = text[offset..].find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if is_boundary_before(text, start) && is_boundary_after(text, end) {
            return true;
        }
        offset = end;
        if offset >= text.len() {
            break;
        }
    }
    false
}

fn sentence_start(text: &str, start: usize) -> bool {
    let prefix = text[..start].trim_end();
    prefix.is_empty()
        || prefix.ends_with(['.', '!', '?', '¡', '¿', ';', ':', ','])
        || [
            " y",
            " and",
            " también",
            " tambien",
            " also",
            " por favor",
            " please",
        ]
        .iter()
        .any(|connector| prefix.ends_with(connector))
}

fn is_boundary_before(text: &str, index: usize) -> bool {
    index == 0
        || text[..index]
            .chars()
            .next_back()
            .is_some_and(|character| !character.is_alphanumeric())
}

fn is_boundary_after(text: &str, index: usize) -> bool {
    index >= text.len()
        || text[index..]
            .chars()
            .next()
            .is_some_and(|character| !character.is_alphanumeric())
}

fn user_visible_text(message: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < message.len() {
        let Some(relative) = message[cursor..].find('<') else {
            output.push_str(&message[cursor..]);
            break;
        };
        let start = cursor + relative;
        output.push_str(&message[cursor..start]);
        let Some((tag, end_open)) = opening_tag(message, start) else {
            output.push('<');
            cursor = start + 1;
            continue;
        };
        let closing = format!("</{tag}>");
        if let Some(relative_close) = message[end_open..].find(&closing) {
            cursor = end_open + relative_close + closing.len();
        } else {
            output.push('<');
            cursor = start + 1;
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn opening_tag(message: &str, start: usize) -> Option<(String, usize)> {
    let bytes = message.as_bytes();
    let mut index = start + 1;
    if !bytes.get(index)?.is_ascii_alphabetic() {
        return None;
    }
    let name_start = index;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        index += 1;
    }
    if bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return None;
    }
    let name = message[name_start..index].to_owned();
    let end = message[index..].find('>')? + index + 1;
    Some((name, end))
}

fn digest(text: &str) -> String {
    let hash = Sha256::digest(text.as_bytes());
    hash[..6].iter().map(|byte| format!("{byte:02x}")).collect()
}

fn safe_marker_segment(value: &str) -> String {
    value
        .chars()
        .take(120)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn safe_inline(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .map(|character| {
            if character.is_control() || matches!(character, '`' | '[' | ']' | '\\') {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn safe_date(value: &str) -> String {
    let date = date_prefix(value);
    if valid_date(&date) { date } else { today() }
}

fn date_prefix(value: &str) -> String {
    value.chars().take(10).collect()
}

fn valid_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn today() -> String {
    OffsetDateTime::now_utc().date().to_string()
}

fn read_memory_file(path: &Path) -> Result<String, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("No se pudo inspeccionar la memoria: {error}")),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err("PROJECT_MEMORY.md no es un archivo regular".to_owned())
        }
        Ok(metadata) if metadata.len() > MAX_MEMORY_FILE_BYTES => {
            Err("PROJECT_MEMORY.md supera el límite de lectura".to_owned())
        }
        Ok(_) => fs::read_to_string(path)
            .map_err(|error| format!("No se pudo leer PROJECT_MEMORY.md: {error}")),
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

fn regular_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
}

fn atomic_replace(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta de memoria".to_owned())?;
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let temporary = parent.join(format!(
        ".project-memory-{}-{stamp}.tmp",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("No se pudo preparar la memoria temporal: {error}"))?;
    if let Err(error) = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
    {
        let _ = fs::remove_file(&temporary);
        return Err(format!("No se pudo escribir la memoria temporal: {error}"));
    }
    drop(file);
    let backup = if path.exists() {
        let backup = parent.join(format!(
            ".project-memory-backup-{}-{stamp}.tmp",
            std::process::id()
        ));
        if let Err(error) = fs::rename(path, &backup) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("No se pudo conservar la memoria anterior: {error}"));
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
        return Err(format!("No se pudo guardar PROJECT_MEMORY.md: {error}"));
    }
    if let Some(backup) = backup {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

#[cfg(test)]
#[path = "explicit_memory_tests.rs"]
mod tests;
