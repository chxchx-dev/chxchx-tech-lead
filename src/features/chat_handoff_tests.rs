use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn scratch() -> PathBuf {
    let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("chxchx-handoff-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn context_release_requires_a_known_window_and_starts_at_ninety_percent() {
    assert!(!needs_release(None, Some(100)));
    assert!(!needs_release(Some(100), None));
    assert!(!needs_release(Some(0), Some(100)));
    assert!(!needs_release(Some(89), Some(100)));
    assert!(needs_release(Some(90), Some(100)));
    assert!(needs_release(Some(100), Some(100)));
}

#[test]
fn saved_handoffs_are_redacted_bounded_and_consumed_once() {
    let root = scratch();
    let text = format!(
        "Objetivo\napi key: no-guardar\n{}",
        "x".repeat(MAX_HANDOFF_CHARS)
    );

    save(&root, "Claude", "session-1", &text).unwrap();
    assert!(save(&root, "Claude", "session-1", "nuevo resumen").is_err());
    let saved = load(&root, "Claude", "session-1").unwrap();
    assert!(saved.contains("(línea omitida: posible secreto)"));
    assert!(!saved.contains("no-guardar"));
    assert!(saved.chars().count() <= MAX_HANDOFF_CHARS);
    assert!(consume(&root, "Claude", "session-1").is_ok());
    assert!(load(&root, "Claude", "session-1").is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn unicode_handoff_limit_also_fits_the_bounded_byte_reader() {
    let root = scratch();
    let summary = "🧠".repeat(MAX_HANDOFF_CHARS + 10);

    save(&root, "OpenCode", "session-2", &summary).unwrap();

    let path = root.join(DIRECTORY).join("opencode-session-2.md");
    let saved = load(&root, "OpenCode", "session-2").unwrap();
    assert!(saved.chars().count() <= MAX_HANDOFF_CHARS);
    assert!(fs::metadata(path).unwrap().len() <= MAX_HANDOFF_BYTES);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn legacy_handoff_reads_are_redacted_and_clipped_before_context_use() {
    let root = scratch();
    let directory = root.join(DIRECTORY);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("claude-legacy-session.md"),
        format!(
            "api key: legacy-secret\n{}",
            "x".repeat(MAX_HANDOFF_CHARS * 2)
        ),
    )
    .unwrap();

    let loaded = load(&root, "Claude", "legacy-session").unwrap();

    assert!(!loaded.contains("legacy-secret"));
    assert!(loaded.contains("(línea omitida: posible secreto)"));
    assert!(loaded.chars().count() <= MAX_HANDOFF_CHARS);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn unknown_providers_and_path_like_session_ids_are_rejected() {
    let root = scratch();
    assert!(save(&root, "Otro", "session", "contenido").is_err());
    assert!(save(&root, "Codex", "../outside", "contenido").is_err());
    assert!(save(&root, "Codex", "", "contenido").is_err());
    assert!(!root.join("outside.md").exists());
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn handoff_paths_do_not_follow_symlinked_project_directories_or_files() {
    use std::os::unix::fs::symlink;

    let root = scratch();
    let outside = scratch();
    symlink(&outside, root.join(".ai")).unwrap();
    assert!(save(&root, "Codex", "s-1", "contenido").is_err());
    assert!(!outside.join("chat-handoffs").exists());

    fs::remove_file(root.join(".ai")).unwrap();
    fs::create_dir_all(root.join(DIRECTORY)).unwrap();
    symlink(
        outside.join("sentinel.md"),
        root.join(DIRECTORY).join("codex-s-1.md"),
    )
    .unwrap();
    assert!(save(&root, "Codex", "s-1", "contenido").is_err());
    assert!(load(&root, "Codex", "s-1").is_none());
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
}
