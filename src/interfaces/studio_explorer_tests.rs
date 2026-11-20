use super::*;
use crate::features::project_files::{Document, Entry};
use iced::keyboard::Modifiers;

struct Project {
    state: Dashboard,
    root: PathBuf,
}

fn project(name: &str) -> Project {
    let base = std::env::temp_dir().join(format!("chxchx-explorer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("proyecto");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "lib").unwrap();
    std::fs::write(root.join("src/main.rs"), "main").unwrap();
    std::fs::write(root.join("README.md"), "leeme").unwrap();
    let root = root.canonicalize().unwrap();
    let mut state = editor::fixture();
    state.root = root.clone();
    list(&mut state, "");
    list(&mut state, "src");
    state.editor.expanded.insert("src".into());
    Project { state, root }
}

/// Fills the tree cache the way a finished listing would.
fn list(state: &mut Dashboard, folder: &str) {
    let entries = crate::features::project_files::list(&state.root, Path::new(folder)).unwrap();
    state.editor.folders.insert(folder.into(), entries);
}

fn open_tab(state: &mut Dashboard, name: &str, dirty: bool) {
    let document = Document {
        relative: name.into(),
        text: "x".into(),
        fingerprint: vec![],
    };
    let _ = editor::update(state, editor::Event::Loaded(Ok(document)));
    state.editor.tabs.last_mut().unwrap().dirty = dirty;
}

fn names(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .map(|e| e.relative.to_string_lossy().replace('\\', "/"))
        .collect()
}

fn run_job(p: &mut Project, job: ops::Job) {
    let outcome = ops::execute(&p.root, job);
    let _ = ops::finished(&mut p.state, outcome);
}

#[test]
fn a_click_selects_and_ctrl_or_shift_extend_the_selection_in_tree_order() {
    let mut p = project("select");
    let _ = update(&mut p.state, Event::Click("README.md".into(), false));
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("README.md")])
    );
    assert!(p.state.explorer.focused);

    p.state.explorer.modifiers = Modifiers::CTRL;
    let _ = update(&mut p.state, Event::Click("src/main.rs".into(), false));
    assert_eq!(p.state.explorer.selection.len(), 2);
    let _ = update(&mut p.state, Event::Click("src/main.rs".into(), false));
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("README.md")]),
        "ctrl toggles"
    );

    // Tree order: src, src/lib.rs, src/main.rs, README.md (folders first).
    p.state.explorer.modifiers = Modifiers::empty();
    let _ = update(&mut p.state, Event::Click("src".into(), true));
    p.state.explorer.modifiers = Modifiers::SHIFT;
    let _ = update(&mut p.state, Event::Click("src/main.rs".into(), false));
    let expected: BTreeSet<PathBuf> = ["src", "src/lib.rs", "src/main.rs"]
        .map(PathBuf::from)
        .into();
    assert_eq!(p.state.explorer.selection, expected);
}

#[test]
fn a_right_click_selects_what_is_not_selected_and_keeps_a_selection_it_belongs_to() {
    let mut p = project("context");
    p.state.explorer.pointer = Point::new(30.0, 40.0);
    let _ = update(&mut p.state, Event::Click("README.md".into(), false));

    let _ = update(
        &mut p.state,
        Event::Context(Target::Item {
            path: "src/lib.rs".into(),
            directory: false,
        }),
    );
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("src/lib.rs")])
    );
    assert_eq!(
        p.state.explorer.menu.as_ref().map(|(_, at)| *at),
        Some(Point::new(30.0, 40.0))
    );

    p.state.explorer.selection = ["README.md", "src/lib.rs"].map(PathBuf::from).into();
    let _ = update(
        &mut p.state,
        Event::Context(Target::Item {
            path: "README.md".into(),
            directory: false,
        }),
    );
    assert_eq!(
        p.state.explorer.selection.len(),
        2,
        "acting on the selection"
    );

    let _ = update(&mut p.state, Event::Context(Target::Root));
    assert!(matches!(p.state.explorer.menu, Some((Target::Root, _))));
}

#[test]
fn escape_closes_the_menu_then_the_question_then_the_name_box() {
    let mut p = project("escape");
    p.state.explorer.menu = Some((Target::Root, Point::ORIGIN));
    p.state.explorer.confirm = Some(vec!["README.md".into()]);
    p.state.explorer.edit = Some(Edit {
        kind: EditKind::NewFile(PathBuf::new()),
        text: "x".into(),
        error: None,
    });

    let _ = update(&mut p.state, Event::Escape);
    assert!(p.state.explorer.menu.is_none() && p.state.explorer.confirm.is_some());
    let _ = update(&mut p.state, Event::Escape);
    assert!(p.state.explorer.confirm.is_none() && p.state.explorer.edit.is_some());
    let _ = update(&mut p.state, Event::Escape);
    assert!(p.state.explorer.edit.is_none());
}

#[test]
fn copy_and_cut_fill_the_clipboard_and_a_paste_without_one_says_so() {
    let mut p = project("clipboard");
    let _ = update(&mut p.state, Event::Do(Action::Paste));
    assert!(p.state.message.contains("nada copiado"));

    p.state.explorer.selection = BTreeSet::from([PathBuf::from("README.md")]);
    let _ = update(&mut p.state, Event::Do(Action::Cut));
    assert_eq!(
        p.state.explorer.clipboard,
        Some(Clipboard {
            paths: vec!["README.md".into()],
            cut: true
        })
    );
    assert!(p.state.message.contains("cortado"));
    let _ = update(&mut p.state, Event::Do(Action::Copy));
    assert_eq!(
        p.state.explorer.clipboard.as_ref().map(|c| c.cut),
        Some(false)
    );
}

#[test]
fn keyboard_shortcuts_act_only_while_the_explorer_has_the_keyboard() {
    let mut p = project("shortcuts");
    p.state.editor.explorer = true;
    p.state.explorer.selection = BTreeSet::from([PathBuf::from("README.md")]);

    p.state.explorer.focused = false;
    let _ = update(&mut p.state, Event::Shortcut(Action::Copy));
    assert!(
        p.state.explorer.clipboard.is_none(),
        "an editor or chat keeps its own Ctrl+C"
    );

    p.state.explorer.focused = true;
    let _ = update(&mut p.state, Event::Shortcut(Action::Copy));
    assert!(p.state.explorer.clipboard.is_some());

    // Typing in another part of the window hands the keyboard back.
    interacted(
        &mut p.state,
        &Message::Editor(editor::Event::Edit(
            iced::widget::text_editor::Action::SelectAll,
        )),
    );
    assert!(!p.state.explorer.focused);
    interacted(&mut p.state, &Message::Wake);
    p.state.explorer.menu = Some((Target::Root, Point::ORIGIN));
    interacted(&mut p.state, &Message::Session(session::Event::Save));
    assert!(
        p.state.explorer.menu.is_some(),
        "background work does not close the menu"
    );
    interacted(&mut p.state, &Message::Refresh);
    assert!(p.state.explorer.menu.is_none());
}

#[test]
fn the_name_box_validates_as_you_type_and_an_invalid_name_never_starts_work() {
    let mut p = project("naming");
    let _ = update(&mut p.state, Event::Do(Action::NewFile));
    assert!(
        matches!(&p.state.explorer.edit, Some(Edit { kind: EditKind::NewFile(parent), .. }) if parent.as_os_str().is_empty())
    );

    let _ = update(&mut p.state, Event::EditText("a/b".into()));
    assert!(p.state.explorer.edit.as_ref().unwrap().error.is_some());
    let before = p.state.pending;
    let _ = update(&mut p.state, Event::EditSubmit);
    assert_eq!(p.state.pending, before, "nothing was started");
    let _ = update(&mut p.state, Event::EditText("ok.rs".into()));
    assert!(p.state.explorer.edit.as_ref().unwrap().error.is_none());
    let _ = update(&mut p.state, Event::EditSubmit);
    assert_eq!(p.state.pending, before + 1, "a valid name starts the job");
}

#[test]
fn new_entries_go_in_the_folder_the_menu_was_opened_on_and_a_file_means_its_folder() {
    let mut p = project("where");
    p.state.explorer.menu = Some((
        Target::Item {
            path: "src".into(),
            directory: true,
        },
        Point::ORIGIN,
    ));
    let _ = update(&mut p.state, Event::Do(Action::NewFolder));
    assert!(
        matches!(&p.state.explorer.edit, Some(Edit { kind: EditKind::NewFolder(parent), .. }) if parent == Path::new("src"))
    );

    p.state.explorer.menu = Some((
        Target::Item {
            path: "src/lib.rs".into(),
            directory: false,
        },
        Point::ORIGIN,
    ));
    let _ = update(&mut p.state, Event::Do(Action::NewFile));
    assert!(
        matches!(&p.state.explorer.edit, Some(Edit { kind: EditKind::NewFile(parent), .. }) if parent == Path::new("src"))
    );
}

#[test]
fn renaming_starts_with_the_current_name_and_refuses_several_items() {
    let mut p = project("rename-start");
    p.state.explorer.selection = BTreeSet::from([PathBuf::from("src/lib.rs")]);
    let _ = update(&mut p.state, Event::Do(Action::Rename));
    assert_eq!(
        p.state.explorer.edit.as_ref().map(|e| e.text.as_str()),
        Some("lib.rs")
    );

    p.state.explorer.edit = None;
    p.state.explorer.selection = ["src/lib.rs", "README.md"].map(PathBuf::from).into();
    p.state.explorer.menu = Some((
        Target::Item {
            path: "README.md".into(),
            directory: false,
        },
        Point::ORIGIN,
    ));
    let _ = update(&mut p.state, Event::Do(Action::Rename));
    assert!(p.state.explorer.edit.is_none() && p.state.message.contains("de uno en uno"));
}

#[test]
fn a_new_file_appears_on_disk_is_selected_and_opened() {
    let mut p = project("create");
    run_job(
        &mut p,
        ops::Job::NewFile(PathBuf::from("src"), "nuevo.rs".into()),
    );

    assert!(p.root.join("src/nuevo.rs").is_file());
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("src/nuevo.rs")])
    );
    assert!(p.state.explorer.edit.is_none());
}

#[test]
fn a_failed_rename_keeps_the_name_box_open_with_the_reason() {
    let mut p = project("rename-fail");
    p.state.explorer.edit = Some(Edit {
        kind: EditKind::Rename("src/lib.rs".into()),
        text: "main.rs".into(),
        error: None,
    });

    run_job(
        &mut p,
        ops::Job::Rename("src/lib.rs".into(), "main.rs".into()),
    );

    let edit = p.state.explorer.edit.as_ref().expect("still editing");
    assert!(
        edit.error
            .as_deref()
            .is_some_and(|e| e.contains("Ya existe"))
    );
    assert!(p.root.join("src/lib.rs").is_file());
}

#[test]
fn renaming_a_folder_moves_its_open_tabs_and_expanded_state_with_it() {
    let mut p = project("rename-folder");
    open_tab(&mut p.state, "src/lib.rs", true);
    open_tab(&mut p.state, "README.md", false);

    run_job(&mut p, ops::Job::Rename("src".into(), "codigo".into()));

    assert!(p.root.join("codigo/lib.rs").is_file());
    let tabs: Vec<_> = p
        .state
        .editor
        .tabs
        .iter()
        .map(|t| t.document.relative.clone())
        .collect();
    assert_eq!(
        tabs,
        [PathBuf::from("codigo/lib.rs"), PathBuf::from("README.md")]
    );
    assert!(
        p.state.editor.tabs[0].dirty,
        "unsaved edits survive the rename"
    );
    assert!(
        p.state.editor.expanded.contains(Path::new("codigo"))
            && !p.state.editor.expanded.contains(Path::new("src"))
    );
    assert!(
        !p.state.editor.folders.contains_key(Path::new("src")),
        "the stale listing is dropped"
    );
}

#[test]
fn deleting_closes_the_tabs_of_what_went_and_can_be_undone() {
    let mut p = project("delete");
    // The application home for the trash, isolated from the real one.
    let home = p.root.parent().unwrap().join("home");
    std::fs::create_dir_all(&home).unwrap();
    open_tab(&mut p.state, "src/lib.rs", true);
    open_tab(&mut p.state, "README.md", false);
    p.state.editor.selected = Some(0);

    let outcome = ops::Outcome::Deleted(
        vec![crate::features::project_fs::trash(&p.root, &home, Path::new("src")).unwrap()],
        None,
    );
    let _ = ops::finished(&mut p.state, outcome);

    assert!(!p.root.join("src").exists());
    let tabs: Vec<_> = p
        .state
        .editor
        .tabs
        .iter()
        .map(|t| t.document.relative.clone())
        .collect();
    assert_eq!(tabs, [PathBuf::from("README.md")]);
    assert_eq!(p.state.editor.selected, Some(0));
    assert_eq!(undo_depth(&p.state), 1);
    assert!(p.state.explorer.selection.is_empty());
    assert!(p.state.message.contains("papelera"));

    let batch = p.state.explorer.trashed.pop().unwrap();
    let restored = ops::execute(&p.root, ops::Job::Restore(batch));
    let _ = ops::finished(&mut p.state, restored);
    assert_eq!(
        std::fs::read_to_string(p.root.join("src/lib.rs")).unwrap(),
        "lib"
    );
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("src")])
    );
}

#[test]
fn only_the_last_ten_deletions_can_be_undone() {
    let mut explorer = State::default();
    for number in 0..15 {
        remember_deletion(
            &mut explorer,
            vec![Trashed {
                original: format!("{number}").into(),
                location: "/tmp/x".into(),
            }],
        );
    }
    assert_eq!(explorer.trashed.len(), 10);
    assert_eq!(explorer.trashed[0][0].original, Path::new("5"));
}

#[test]
fn pasting_a_cut_moves_the_tab_and_empties_the_clipboard_but_a_copy_keeps_it() {
    let mut p = project("paste");
    open_tab(&mut p.state, "README.md", false);
    p.state.explorer.clipboard = Some(Clipboard {
        paths: vec!["README.md".into()],
        cut: true,
    });

    let cut = p.state.explorer.clipboard.clone().unwrap();
    run_job(&mut p, ops::Job::Paste(cut, "src".into()));

    assert!(p.root.join("src/README.md").is_file() && !p.root.join("README.md").exists());
    assert_eq!(
        p.state.editor.tabs[0].document.relative,
        Path::new("src/README.md")
    );
    assert!(p.state.explorer.clipboard.is_none());

    p.state.explorer.clipboard = Some(Clipboard {
        paths: vec!["src/lib.rs".into()],
        cut: false,
    });
    let copy = p.state.explorer.clipboard.clone().unwrap();
    run_job(&mut p, ops::Job::Paste(copy, "".into()));
    assert!(p.root.join("lib.rs").is_file() && p.root.join("src/lib.rs").is_file());
    assert!(
        p.state.explorer.clipboard.is_some(),
        "a copy can be pasted again"
    );
}

#[test]
fn names_and_paths_are_what_the_clipboard_actions_offer() {
    assert_eq!(name_of(Path::new("src/lib.rs")), "lib.rs");
    assert_eq!(slashes(Path::new("src\\lib.rs")), "src/lib.rs");
    let p = project("tree");
    assert_eq!(
        names(p.state.editor.folders.get(Path::new("")).unwrap()),
        ["src", "README.md"]
    );
    assert!(
        is_directory(&p.state, Path::new("src")) && !is_directory(&p.state, Path::new("README.md"))
    );
}
