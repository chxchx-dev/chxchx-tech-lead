//! A small, bounded cache of the errors the product reported, per machine, with secrets removed.
//!
//! `workspace errors` lists them and `workspace report-error` adds one. The file is
//! `<home>/cache/errors.jsonl`: the last 200 entries, each message clipped and redacted before it
//! is written, replaced atomically, and never written through a symbolic link.
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub const MAX_ENTRIES: usize = 200;
const MAX_MESSAGE_CHARS: usize = 2400;
const REDACTED: &str = "[REDACTED]";
const SECRET_NAMES: [&str; 8] = [
    "api_key",
    "api-key",
    "apikey",
    "access_token",
    "access-token",
    "refresh_token",
    "refresh-token",
    "authorization",
];
const SHORT_SECRET_NAMES: [&str; 4] = ["token", "password", "passwd", "secret"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedError {
    pub occurred_at: String,
    pub project: String,
    pub project_path: String,
    pub operation: String,
    pub message: String,
}

pub fn cache_path(home: &Path) -> PathBuf {
    home.join("cache").join("errors.jsonl")
}

fn clip(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._~+/=-".contains(c)
}

/// Replaces what looks like a credential: `key=value` pairs, bearer tokens and well-known token shapes.
pub fn redact(message: &str) -> String {
    let chars: Vec<char> = message.chars().collect();
    let lower: String = message.to_ascii_lowercase();
    let lower: Vec<char> = lower.chars().collect();
    let mut out = String::with_capacity(message.len());
    let mut i = 0;
    while i < chars.len() {
        let at_word_start = i == 0 || !is_word_char(chars[i - 1]);
        if at_word_start {
            if let Some(end) = bearer_token_end(&lower, &chars, i) {
                out.push_str("Bearer ");
                out.push_str(REDACTED);
                i = end;
                continue;
            }
            if let Some((name_end, value_end)) = secret_assignment(&lower, &chars, i) {
                out.extend(&chars[i..name_end]);
                out.push_str(REDACTED);
                i = value_end;
                continue;
            }
            if let Some(end) = known_token_end(&chars, i) {
                out.push_str(REDACTED);
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn starts_with(lower: &[char], at: usize, word: &str) -> bool {
    word.chars()
        .enumerate()
        .all(|(offset, c)| lower.get(at + offset) == Some(&c))
}

fn bearer_token_end(lower: &[char], chars: &[char], at: usize) -> Option<usize> {
    if !starts_with(lower, at, "bearer ") {
        return None;
    }
    let start = at + "bearer ".len();
    let end = (start..chars.len())
        .find(|&index| !is_token_char(chars[index]))
        .unwrap_or(chars.len());
    (end > start).then_some(end)
}

/// `name`, optional spaces, `:` or `=`, optional spaces, then a value up to a space, comma or semicolon.
/// Returns where the name (with its separator) ends and where the value ends.
fn secret_assignment(lower: &[char], chars: &[char], at: usize) -> Option<(usize, usize)> {
    let name = SECRET_NAMES
        .iter()
        .chain(SHORT_SECRET_NAMES.iter())
        .find(|name| {
            starts_with(lower, at, name)
                && !lower.get(at + name.len()).is_some_and(|c| is_word_char(*c))
        })?;
    let mut index = at + name.len();
    while chars.get(index) == Some(&' ') {
        index += 1;
    }
    if !matches!(chars.get(index), Some(':' | '=')) {
        return None;
    }
    index += 1;
    while chars.get(index) == Some(&' ') {
        index += 1;
    }
    let value_start = index;
    let ends_value = |i: usize| chars[i].is_whitespace() || chars[i] == ',' || chars[i] == ';';
    let mut value_end = (value_start..chars.len())
        .find(|&i| ends_value(i))
        .unwrap_or(chars.len());
    // `Authorization: Bearer <token>`: the secret is the token after the scheme, not the scheme.
    if starts_with(lower, value_start, "bearer ") && value_end == value_start + "bearer".len() {
        let token_start = value_end + 1;
        value_end = (token_start..chars.len())
            .find(|&i| !is_token_char(chars[i]))
            .unwrap_or(chars.len());
    }
    (value_end > value_start).then_some((value_start, value_end))
}

fn known_token_end(chars: &[char], at: usize) -> Option<usize> {
    let rest = &chars[at..];
    let (prefix, minimum, allowed): (usize, usize, fn(char) -> bool) =
        if rest.starts_with(&['s', 'k', '-']) {
            (3, 16, |c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        } else if rest.len() > 4
            && rest[0] == 'g'
            && rest[1] == 'h'
            && "pousr".contains(rest[2])
            && rest[3] == '_'
        {
            (4, 20, |c| c.is_ascii_alphanumeric())
        } else {
            return None;
        };
    let length = rest[prefix..].iter().take_while(|c| allowed(**c)).count();
    (length >= minimum).then_some(at + prefix + length)
}

fn read_entries(path: &Path) -> Vec<Value> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut entries: Vec<Value> = contents
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(Value::is_object)
        .collect();
    let excess = entries.len().saturating_sub(MAX_ENTRIES);
    entries.drain(..excess);
    entries
}

fn write_entries(path: &Path, entries: &[Value]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("La ruta del registro no tiene carpeta")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("No pude crear {}: {error}", parent.display()))?;
    if fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err("El registro de errores es un enlace simbólico; no se modificó".to_owned());
    }
    let temp = parent.join(format!(".errors-{}.tmp", std::process::id()));
    let mut file = fs::File::create(&temp).map_err(|error| error.to_string())?;
    for entry in entries {
        writeln!(file, "{entry}").map_err(|error| error.to_string())?;
    }
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        error.to_string()
    })
}

/// Adds an entry, keeping the newest [`MAX_ENTRIES`].
pub fn record(
    home: &Path,
    project: &str,
    project_path: &Path,
    operation: &str,
    message: &str,
) -> Result<(), String> {
    let path = cache_path(home);
    let mut entries = read_entries(&path);
    entries.push(json!({
        "occurred_at": OffsetDateTime::now_utc().format(&Rfc3339).map_err(|error| error.to_string())?,
        "project": clip(project, 120),
        "project_path": clip(&project_path.display().to_string(), 500),
        "operation": clip(operation, 120),
        "message": clip(&redact(message), MAX_MESSAGE_CHARS),
    }));
    let excess = entries.len().saturating_sub(MAX_ENTRIES);
    entries.drain(..excess);
    write_entries(&path, &entries)
}

/// The most recent entries first, optionally only those of one project.
pub fn list(home: &Path, project_path: Option<&Path>, limit: usize) -> Vec<CachedError> {
    let wanted = project_path.map(|path| {
        fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
            .to_string()
    });
    let text = |entry: &Value, key: &str| entry[key].as_str().map(str::to_owned);
    let mut found: Vec<CachedError> = read_entries(&cache_path(home))
        .iter()
        .filter(|entry| wanted.as_deref().is_none_or(|w| entry["project_path"] == w))
        .filter_map(|entry| {
            Some(CachedError {
                occurred_at: text(entry, "occurred_at")?,
                project: text(entry, "project")?,
                project_path: text(entry, "project_path")?,
                operation: text(entry, "operation")?,
                message: text(entry, "message")?,
            })
        })
        .collect();
    found.reverse();
    found.truncate(limit);
    found
}

#[cfg(test)]
#[path = "error_cache_tests.rs"]
mod tests;
