//! The node's audit log: who did what to the node and its jobs, kept in `audit.jsonl`.
//!
//! One JSON object per line, append-only, readable only by this user. It records decisions and
//! actors, never the text of prompts, answers or tokens, and never the account identity that
//! `tailscale serve` forwards. It rotates to `audit.jsonl.1` at 5 MiB.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;

pub const MAX_AUDIT_BYTES: u64 = 5 * 1024 * 1024;
const MAX_TEXT: usize = 200;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub at_ms: u64,
    /// `local` for the person at this PC, or a linked device's id.
    pub actor: String,
    pub action: String,
    #[serde(default)]
    pub detail: Value,
}

/// Appends an entry. Auditing never fails the request that causes it: if it cannot write, it
/// says so on stderr (the node's own output).
pub fn record(dir: &Path, actor: &str, action: &str, detail: Value) {
    static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = WRITING.lock().unwrap_or_else(|poison| poison.into_inner());
    let path = dir.join("audit.jsonl");
    if fs::metadata(&path).is_ok_and(|meta| meta.len() >= MAX_AUDIT_BYTES) {
        let _ = fs::rename(&path, dir.join("audit.jsonl.1"));
    }
    let entry = Entry {
        at_ms: crate::features::jobs::store::now_ms(),
        actor: clean(actor),
        action: clean(action),
        detail: clean_value(detail),
    };
    let Ok(line) = serde_json::to_string(&entry) else {
        return;
    };
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let written = options
        .open(&path)
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = written {
        eprintln!("Aviso: no se pudo escribir la auditoría: {error}");
    }
}

/// The newest `limit` entries, newest first.
pub fn read(dir: &Path, limit: usize) -> Vec<Entry> {
    let mut entries: Vec<Entry> = ["audit.jsonl.1", "audit.jsonl"]
        .iter()
        .filter_map(|name| fs::read_to_string(dir.join(name)).ok())
        .flat_map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str::<Entry>(line).ok())
                .collect::<Vec<_>>()
        })
        .collect();
    entries.reverse();
    entries.truncate(limit);
    entries
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(MAX_TEXT)
        .collect()
}

/// Every string, at any depth, is one line and bounded: nothing can forge another entry.
fn clean_value(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(clean(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(clean_value).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (clean(&key), clean_value(value)))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-audit-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn entries_are_read_back_newest_first_and_one_per_line() {
        let dir = scratch("order");
        record(&dir, "local", "device_paired", json!({ "device": "dev-1" }));
        record(
            &dir,
            "dev-1",
            "job_created",
            json!({ "access": "workspace-write" }),
        );
        let entries = read(&dir, 10);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].action, "job_created");
        assert_eq!(entries[1].actor, "local");
        assert_eq!(read(&dir, 1).len(), 1);
    }

    #[test]
    fn text_cannot_forge_entries_and_is_bounded() {
        let dir = scratch("forge");
        let forged = "x\n{\"at_ms\":1,\"actor\":\"local\",\"action\":\"fake\"}";
        record(
            &dir,
            "dev-1",
            forged,
            json!({ "name": forged, "long": "y".repeat(5000) }),
        );
        let text = fs::read_to_string(dir.join("audit.jsonl")).unwrap();
        assert_eq!(text.lines().count(), 1);
        let entry = &read(&dir, 10)[0];
        assert_eq!(
            entry.detail["long"].as_str().unwrap().chars().count(),
            MAX_TEXT
        );
        assert!(entry.action.chars().count() <= MAX_TEXT);
    }

    #[test]
    fn the_log_rotates_when_it_grows_and_both_files_are_read() {
        let dir = scratch("rotate");
        record(&dir, "local", "old", json!({}));
        let size = fs::metadata(dir.join("audit.jsonl")).unwrap().len();
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(dir.join("audit.jsonl"))
            .unwrap();
        file.write_all(&vec![b' '; (MAX_AUDIT_BYTES - size) as usize])
            .unwrap();
        drop(file);
        record(&dir, "local", "new", json!({}));
        assert!(dir.join("audit.jsonl.1").exists());
        let actions: Vec<String> = read(&dir, 10).into_iter().map(|e| e.action).collect();
        assert_eq!(actions, ["new", "old"]);
    }
}
