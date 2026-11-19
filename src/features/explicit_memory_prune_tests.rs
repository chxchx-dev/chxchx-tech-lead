use super::*;
use std::fs;

fn entry(text: &str) -> String {
    format!(
        "### Petición explícita de memoria — 2026-11-18\n\n<!-- explicit-memory:Claude:session:{:012x} -->\nOrigen: Claude, conversación `session`.\n\n> {text}\n",
        text.len()
    )
}

#[test]
fn pruning_preserves_handwritten_notes_and_removes_noise_and_duplicates() {
    let manual = "# Memoria\n\n## Nota propia\n\nUsamos Qt 6.8.\n\n";
    let content = format!(
        "{manual}{}\n{}\n{}\n## Otra nota\n\nNo tocar.\n",
        entry("Revisa el uso de memoria RAM."),
        entry("Recuerda que el deploy es los viernes."),
        entry("recuerda que el deploy es los viernes.")
    );

    let (pruned, removed) = prune_content(&content);

    assert_eq!(
        removed,
        [
            "Revisa el uso de memoria RAM.",
            "recuerda que el deploy es los viernes."
        ]
    );
    assert!(pruned.starts_with(manual));
    assert_eq!(pruned.matches("Recuerda que el deploy").count(), 1);
    assert!(pruned.ends_with("## Otra nota\n\nNo tocar.\n"));
}

#[test]
fn dry_run_does_not_create_backup_or_change_project_files() {
    let root = std::env::temp_dir().join(format!(
        "chxchx-memory-prune-dry-{}-{}",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    fs::create_dir_all(root.join(".ai/memory")).unwrap();
    let path = root.join(".ai/memory/PROJECT_MEMORY.md");
    let before = entry("Revisa el uso de memoria RAM.");
    fs::write(&path, &before).unwrap();

    let result = prune(&root, true).unwrap();

    assert_eq!(result.removed, ["Revisa el uso de memoria RAM."]);
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
    assert_eq!(result.backup, None);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn backup_inside_the_project_is_rejected_before_creating_its_directory() {
    let root = std::env::temp_dir().join(format!(
        "chxchx-memory-prune-backup-{}-{}",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    fs::create_dir_all(root.join(".ai/memory")).unwrap();
    let path = root.join(".ai/memory/PROJECT_MEMORY.md");
    fs::write(&path, entry("Revisa el uso de memoria RAM.")).unwrap();
    let local_home = root.join(".local-home");

    assert!(backup_memory_file_at(&root, &path, &local_home).is_err());
    assert!(!local_home.exists());

    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn pruning_rejects_a_memory_file_symlink() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "chxchx-memory-prune-link-{}-{}",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let outside = root.with_extension("outside.md");
    fs::create_dir_all(root.join(".ai/memory")).unwrap();
    fs::write(&outside, entry("Revisa el uso de memoria RAM.")).unwrap();
    symlink(&outside, root.join(".ai/memory/PROJECT_MEMORY.md")).unwrap();

    assert!(prune(&root, false).is_err());
    assert!(
        fs::read_to_string(&outside)
            .unwrap()
            .contains("Revisa el uso de memoria RAM.")
    );
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(outside);
}
