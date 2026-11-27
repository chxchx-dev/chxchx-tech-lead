//! What is left of an agent's plan (the 5 hour and weekly windows), read on demand.
//!
//! Codex writes its latest limits into the session files it already keeps, so reading them costs
//! no request and no tokens. Nothing here runs on its own: the chat's "Actualizar" button calls it.
use crate::features::chat_model::Window;
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The limits found, and when the agent recorded them (unix seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub primary: Option<Window>,
    pub secondary: Option<Window>,
    pub plan: String,
    pub observed_at: i64,
}

/// Where Codex keeps its data: `CODEX_HOME` or `~/.codex`.
pub fn codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".codex")))
}

fn newest(dir: &Path) -> Option<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    // Year, month, day and rollout names sort chronologically.
    entries.sort();
    entries.pop()
}

/// The most recent rollout file, going down year/month/day without listing every session.
fn latest_rollout(sessions: &Path) -> Option<PathBuf> {
    let mut dir = sessions.to_path_buf();
    for _ in 0..3 {
        dir = newest(&dir)?;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
        })
        .collect();
    files.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
    files.pop()
}

fn window(value: Option<&Value>) -> Option<Window> {
    let value = value?;
    let used = value.get("used_percent").and_then(Value::as_f64)?;
    (used >= 0.0).then(|| Window {
        used_percent: used,
        window_minutes: value
            .get("window_minutes")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        resets_at: value.get("resets_at").and_then(Value::as_i64).unwrap_or(0),
    })
}

/// Latest limits Codex recorded under `codex_home`, or `None` when it has none yet.
pub fn codex_snapshot(codex_home: &Path) -> Option<Snapshot> {
    let file = latest_rollout(&codex_home.join("sessions"))?;
    let observed_at = file
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);
    let mut stream = std::fs::File::open(&file).ok()?;
    let len = stream.metadata().ok()?.len();
    let start = len.saturating_sub(1_000_000);
    stream.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let mut found = None;
    for line in text.lines() {
        if !line.contains("rate_limits") {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(limits) = record
            .get("payload")
            .filter(|p| p.get("type").and_then(Value::as_str) == Some("token_count"))
            .and_then(|p| p.get("rate_limits"))
        else {
            continue;
        };
        let primary = window(limits.get("primary"));
        let secondary = window(limits.get("secondary"));
        if primary.is_none() && secondary.is_none() {
            continue;
        }
        found = Some(Snapshot {
            primary,
            secondary,
            plan: limits
                .get("plan_type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            observed_at,
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("limits-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_last_token_count_of_the_newest_session_wins() {
        let root = home("newest");
        let old = root.join("sessions/2026/11/24");
        let new = root.join("sessions/2026/11/25");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        let line = |used: f64| {
            format!(
                r#"{{"payload":{{"type":"token_count","rate_limits":{{"primary":{{"used_percent":{used},"window_minutes":300,"resets_at":10}},"secondary":{{"used_percent":5.0,"window_minutes":10080,"resets_at":20}},"plan_type":"plus"}}}}}}"#
            )
        };
        std::fs::write(old.join("rollout-a.jsonl"), line(90.0)).unwrap();
        std::fs::write(
            new.join("rollout-b.jsonl"),
            format!("{}\nnot json\n{}\n", line(10.0), line(33.0)),
        )
        .unwrap();
        let snapshot = codex_snapshot(&root).unwrap();
        assert_eq!(snapshot.primary.unwrap().used_percent, 33.0);
        assert_eq!(snapshot.secondary.unwrap().window_minutes, 10080);
        assert_eq!(snapshot.plan, "plus");
        assert!(snapshot.observed_at > 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn no_sessions_or_no_limits_is_none() {
        let root = home("none");
        assert!(codex_snapshot(&root).is_none());
        let day = root.join("sessions/2026/11/25");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(
            day.join("rollout-c.jsonl"),
            r#"{"payload":{"type":"token_count","rate_limits":{"primary":null}}}"#,
        )
        .unwrap();
        assert!(codex_snapshot(&root).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }
}
