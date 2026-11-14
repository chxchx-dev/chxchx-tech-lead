use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-prefs-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn preferences_round_trip_and_stay_bounded() {
    let home = scratch("round");
    let root = Path::new("C:/work/demo");
    let mut preferences = Preferences::default();
    preferences.layout.dock = Some("terminal".into());
    preferences.layout.explorer_width = 9999.0;
    preferences.touch_recent_project(root);
    preferences.set_session(
        root,
        ProjectSession {
            tabs: vec![
                "src/main.rs".into(),
                "../escape".into(),
                "C:/abs".into(),
                "src/main.rs".into(),
            ],
            active: Some("src/main.rs".into()),
            recent_files: (0..40).map(|index| format!("f{index}.rs")).collect(),
            ..Default::default()
        },
    );
    save_to(&home, &preferences).unwrap();
    let loaded = load_from(&home);
    assert!(loaded.writable && loaded.warning.is_none());
    let loaded = loaded.preferences;
    assert_eq!(loaded.layout.explorer_width, 520.0);
    assert_eq!(loaded.layout.dock.as_deref(), Some("terminal"));
    let session = loaded.session(root);
    assert_eq!(session.tabs, vec!["src/main.rs"]);
    assert_eq!(session.recent_files.len(), MAX_RECENT_FILES);
    assert_eq!(session.page, "home");
    assert_eq!(loaded.recent_projects.len(), 1);
    assert!(!fs::read_dir(&home).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")
    }));
}

#[test]
fn damaged_or_newer_files_are_never_overwritten_silently() {
    let home = scratch("damaged");
    fs::write(home.join(FILE_NAME), b"{ not json").unwrap();
    let loaded = load_from(&home);
    assert!(loaded.warning.unwrap().contains("dañado"));
    assert!(home.join("studio-preferences.json.corrupt").exists());
    assert_eq!(loaded.preferences, Preferences::default());

    let home = scratch("newer");
    fs::write(home.join(FILE_NAME), br#"{"version": 99}"#).unwrap();
    let loaded = load_from(&home);
    assert!(!loaded.writable);
    assert_eq!(
        fs::read(home.join(FILE_NAME)).unwrap(),
        br#"{"version": 99}"#
    );
}

#[test]
fn recent_lists_keep_order_uniqueness_and_limit() {
    let mut list = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
    push_recent(&mut list, "c", 3);
    push_recent(&mut list, "d", 3);
    assert_eq!(list, ["d", "c", "a"]);
    assert!(is_safe_relative("src/lib.rs"));
    assert!(!is_safe_relative("../x") && !is_safe_relative("/etc/passwd") && !is_safe_relative(""));
}
