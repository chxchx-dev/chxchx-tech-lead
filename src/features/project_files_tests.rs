use super::*;

static NEXT_FIXTURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        // The clock alone can repeat on Windows, so parallel tests need a counter too.
        // macOS temp folders live under `/var`, a link to `/private/var`: backups refuse paths with links,
        // so the fixture uses the real path there (Windows keeps its plain paths: `canonicalize` adds `\\?\`).
        let base = if cfg!(target_os = "macos") {
            std::env::temp_dir()
                .canonicalize()
                .unwrap_or_else(|_| std::env::temp_dir())
        } else {
            std::env::temp_dir()
        };
        let path = base.join(format!(
            "chxchx-editor-test-{}-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos(),
            NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn save_preserves_original_backup_and_rejects_external_changes() {
    let fixture = Fixture::new();
    let root = fixture.0.join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.rs"), "first\r\n").unwrap();
    let original = read(&root, Path::new("main.rs")).unwrap();
    let saved = save_with_home(&root, &original, "second\r\n", &fixture.0.join("home")).unwrap();
    assert_eq!(saved.text, "second\r\n");
    let backups = fs::read_dir(fixture.0.join("home/editor-backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let backup = fs::read_dir(backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read_to_string(backup).unwrap(), "first\r\n");
    fs::write(root.join("main.rs"), "external").unwrap();
    assert!(save_with_home(&root, &saved, "overwrite", &fixture.0.join("home")).is_err());
    assert_eq!(
        fs::read_to_string(root.join("main.rs")).unwrap(),
        "external"
    );
}

#[test]
fn paths_binary_and_sensitive_environment_files_are_rejected() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join(".env"), "fixture only").unwrap();
    fs::write(fixture.0.join("binary"), [0, 1, 2]).unwrap();
    assert!(read(&fixture.0, Path::new("../outside")).is_err());
    assert!(read(&fixture.0, Path::new(".env")).is_err());
    assert!(read(&fixture.0, Path::new("binary")).is_err());
    assert!(
        !list(&fixture.0, Path::new(""))
            .unwrap()
            .iter()
            .any(|e| e.relative == Path::new(".env"))
    );
}

#[test]
fn editor_rejects_links_when_the_platform_can_create_them() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("target"), "content").unwrap();
    #[cfg(windows)]
    let linked =
        std::os::windows::fs::symlink_file(fixture.0.join("target"), fixture.0.join("link"));
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(fixture.0.join("target"), fixture.0.join("link"));
    if let Err(error) = linked {
        assert_eq!(error.raw_os_error(), Some(1314));
        return;
    }
    assert!(read(&fixture.0, Path::new("link")).is_err());
}
