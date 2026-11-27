//! Lexical search over the project's own documents (ADR 0018, phase C3): FTS5 with BM25, chunked by
//! headings, indexed incrementally. Nothing leaves the machine and nothing new is installed.
//!
//! What gets indexed: Markdown and text files that git would track (or that are not ignored), never
//! credential files, and every chunk goes through the credential redactor before it is stored.
use crate::features::error_cache::redact;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_FILE: u64 = 512 * 1024;
const MAX_CHUNK: usize = 1500;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct IndexStats {
    pub files: usize,
    pub chunks: usize,
    pub unchanged: usize,
    pub removed: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct Hit {
    pub path: String,
    pub heading: String,
    pub snippet: String,
}

/// Files that never enter the index, whatever git says.
fn is_private(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_lowercase();
    name.starts_with(".env")
        || name.contains("credential")
        || name.contains("secret")
        || name.starts_with("id_rsa")
        || [".pem", ".key", ".p12", ".pfx"]
            .iter()
            .any(|ext| name.ends_with(ext))
}

fn is_document(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".txt")
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if ![".git", "target", "node_modules", ".venv"].contains(&name.as_str()) {
                walk(&path, base, out);
            }
        } else if let Ok(relative) = path.strip_prefix(base) {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// The candidate files: git's own view (tracked plus untracked, minus ignored) or, outside a
/// repository, a walk that skips the usual build folders.
fn candidates(root: &Path) -> Vec<String> {
    let listed = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "-co", "--exclude-standard"])
        .output()
        .ok()
        .filter(|out| out.status.success());
    let mut files: Vec<String> = match listed {
        Some(out) => String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        None => {
            let mut all = Vec::new();
            walk(root, root, &mut all);
            all
        }
    };
    files.retain(|p| is_document(p) && !is_private(p));
    files.sort();
    files.dedup();
    files
}

/// Splits a document at its headings, and long sections at blank lines.
fn chunks(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let (mut heading, mut body) = (String::new(), String::new());
    let mut flush = |heading: &str, body: &mut String| {
        let mut piece = String::new();
        for paragraph in body.split("\n\n") {
            if !piece.is_empty() && piece.len() + paragraph.len() > MAX_CHUNK {
                out.push((heading.to_owned(), std::mem::take(&mut piece)));
            }
            if !piece.is_empty() {
                piece.push_str("\n\n");
            }
            piece.push_str(paragraph);
        }
        if !piece.trim().is_empty() {
            out.push((heading.to_owned(), piece));
        }
        body.clear();
    };
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if !fenced && line.starts_with('#') {
            flush(&heading, &mut body);
            heading = line.trim_start_matches('#').trim().to_owned();
        }
        body.push_str(line);
        body.push('\n');
    }
    flush(&heading, &mut body);
    out
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Brings the index up to date with the files on disk: only changed files are read again.
pub fn index(db: &Connection, root: &Path) -> Result<IndexStats, String> {
    let message = |e: rusqlite::Error| e.to_string();
    let known: BTreeMap<String, String> = {
        let mut statement = db
            .prepare("SELECT path, hash FROM rag_doc")
            .map_err(message)?;
        statement
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(message)?
            .collect::<Result<_, _>>()
            .map_err(message)?
    };
    let mut stats = IndexStats::default();
    let mut seen = Vec::new();
    for relative in candidates(root) {
        let file: PathBuf = root.join(&relative);
        let Ok(meta) = fs::metadata(&file) else {
            continue;
        };
        if meta.len() > MAX_FILE {
            continue;
        }
        let Ok(bytes) = fs::read(&file) else {
            continue;
        };
        seen.push(relative.clone());
        let hash = digest(&bytes);
        if known.get(&relative) == Some(&hash) {
            stats.unchanged += 1;
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let pieces = chunks(&text);
        let transaction = db.unchecked_transaction().map_err(message)?;
        transaction
            .execute("DELETE FROM rag_chunk WHERE path = ?1", [&relative])
            .map_err(message)?;
        for (heading, body) in &pieces {
            transaction
                .execute(
                    "INSERT INTO rag_chunk (path, heading, body) VALUES (?1, ?2, ?3)",
                    params![relative, redact(heading), redact(body)],
                )
                .map_err(message)?;
        }
        transaction
            .execute(
                "INSERT INTO rag_doc (path, hash) VALUES (?1, ?2)
                 ON CONFLICT(path) DO UPDATE SET hash = excluded.hash",
                params![relative, hash],
            )
            .map_err(message)?;
        transaction.commit().map_err(message)?;
        stats.files += 1;
        stats.chunks += pieces.len();
    }
    for gone in known.keys().filter(|p| !seen.contains(p)) {
        db.execute("DELETE FROM rag_chunk WHERE path = ?1", [gone])
            .map_err(message)?;
        db.execute("DELETE FROM rag_doc WHERE path = ?1", [gone])
            .map_err(message)?;
        stats.removed += 1;
    }
    Ok(stats)
}

/// Turns what the person typed into an FTS5 query: every word quoted (so punctuation can never be
/// read as syntax), joined with OR, and ranked by BM25.
fn match_query(text: &str) -> Option<String> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{w}\""))
        .collect();
    (!words.is_empty()).then(|| words.join(" OR "))
}

pub fn search(db: &Connection, query: &str, limit: usize) -> Result<Vec<Hit>, String> {
    let Some(expression) = match_query(query) else {
        return Ok(Vec::new());
    };
    let message = |e: rusqlite::Error| e.to_string();
    let mut statement = db
        .prepare(
            "SELECT path, heading, snippet(rag_chunk, 2, '[', ']', '…', 24)
             FROM rag_chunk WHERE rag_chunk MATCH ?1 ORDER BY bm25(rag_chunk) LIMIT ?2",
        )
        .map_err(message)?;
    statement
        .query_map(params![expression, limit as i64], |r| {
            Ok(Hit {
                path: r.get(0)?,
                heading: r.get(1)?,
                snippet: r.get::<_, String>(2)?.replace('\n', " "),
            })
        })
        .map_err(message)?
        .collect::<Result<_, _>>()
        .map_err(message)
}

#[cfg(test)]
#[path = "rag_tests.rs"]
mod tests;
