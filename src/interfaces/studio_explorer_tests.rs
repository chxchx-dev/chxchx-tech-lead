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
fn copy_and_cut_fill_the_clipboard_and_a_paste_without_one_looks_at_the_system() {
    let mut p = project("clipboard");
    let before = p.state.pending;
    let _ = update(&mut p.state, Event::Do(Action::Paste));
    assert_eq!(
        p.state.pending,
        before + 1,
        "the system clipboard is read off the UI thread"
    );

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

fn press(p: &mut Project, path: &str, directory: bool) {
    let _ = update(&mut p.state, Event::Press(path.into(), directory));
}

#[test]
fn a_press_selects_and_a_release_in_place_is_a_click_that_opens() {
    let mut p = project("press-click");
    p.state.explorer.pointer = Point::new(10.0, 10.0);
    press(&mut p, "README.md", false);
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("README.md")])
    );
    assert!(p.state.explorer.pressed.is_some() && !p.state.explorer.dragging);

    // A little jitter is still a click.
    let _ = update(&mut p.state, Event::Pointer(Point::new(13.0, 12.0)));
    assert!(!p.state.explorer.dragging);
    let _ = update(&mut p.state, Event::Release);
    assert!(p.state.explorer.pressed.is_none() && p.state.pending == 0);
}

#[test]
fn moving_the_pointer_far_enough_with_a_row_held_starts_a_drag() {
    let mut p = project("press-drag");
    p.state.explorer.pointer = Point::new(10.0, 10.0);
    press(&mut p, "README.md", false);
    let _ = update(&mut p.state, Event::Pointer(Point::new(40.0, 10.0)));

    assert!(p.state.explorer.dragging);
    // Rows only report hovering while something is held.
    let _ = update(&mut p.state, Event::Hover(Some(("src".into(), true))));
    assert_eq!(p.state.explorer.hover, Some((PathBuf::from("src"), true)));
    let mut idle = project("hover-idle");
    let _ = update(&mut idle.state, Event::Hover(Some(("src".into(), true))));
    assert!(idle.state.explorer.hover.is_none());
}

#[test]
fn dropping_moves_the_row_into_the_folder_under_the_pointer_or_copies_with_ctrl() {
    let mut p = project("drop");
    p.state.explorer.pointer = Point::new(10.0, 10.0);
    press(&mut p, "README.md", false);
    let _ = update(&mut p.state, Event::Pointer(Point::new(50.0, 40.0)));
    let _ = update(&mut p.state, Event::Hover(Some(("src".into(), true))));
    let before = p.state.pending;
    let _ = update(&mut p.state, Event::Release);
    assert_eq!(p.state.pending, before + 1, "the move runs as a job");
    assert!(
        !p.state.explorer.dragging
            && p.state.explorer.pressed.is_none()
            && p.state.explorer.hover.is_none()
    );
    assert!(p.state.editor.expanded.contains(Path::new("src")));

    // The jobs the drop builds, on other files: the one it started runs on its own thread.
    run_job(
        &mut p,
        ops::Job::Paste(
            Clipboard {
                paths: vec!["src/lib.rs".into()],
                cut: true,
            },
            "".into(),
        ),
    );
    assert!(p.root.join("lib.rs").is_file() && !p.root.join("src/lib.rs").exists());
    run_job(
        &mut p,
        ops::Job::Paste(
            Clipboard {
                paths: vec!["src/main.rs".into()],
                cut: false,
            },
            "".into(),
        ),
    );
    assert!(p.root.join("main.rs").is_file() && p.root.join("src/main.rs").is_file());
}

#[test]
fn a_drag_over_a_file_lands_in_its_folder_and_over_nothing_in_the_project_root() {
    let target = |hover: Option<(&str, bool)>| {
        hover.map_or_else(PathBuf::new, |(path, directory)| {
            folder_of(&Target::Item {
                path: path.into(),
                directory,
            })
        })
    };
    assert_eq!(target(Some(("src/lib.rs", false))), Path::new("src"));
    assert_eq!(target(Some(("src", true))), Path::new("src"));
    assert_eq!(target(None), Path::new(""));
}

#[test]
fn dragging_a_selection_drags_all_of_it_and_a_ctrl_press_only_extends() {
    let mut p = project("drag-many");
    p.state.explorer.selection = ["README.md", "src/lib.rs"].map(PathBuf::from).into();
    press(&mut p, "README.md", false);
    assert_eq!(
        p.state.explorer.selection.len(),
        2,
        "pressing inside the selection keeps it"
    );

    let _ = update(&mut p.state, Event::Release);
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("README.md")]),
        "a click narrows it"
    );

    p.state.explorer.modifiers = Modifiers::CTRL;
    press(&mut p, "src/main.rs", false);
    assert_eq!(p.state.explorer.selection.len(), 2);
    assert!(!p.state.explorer.pressed.as_ref().unwrap().plain);
    let _ = update(&mut p.state, Event::Release);
    assert_eq!(
        p.state.explorer.selection.len(),
        2,
        "releasing a ctrl press opens nothing and changes nothing"
    );
}

#[test]
fn a_drop_from_outside_is_copied_in_only_while_the_explorer_is_showing() {
    let mut p = project("outside");
    let outside = p.root.parent().unwrap().join("fuera.txt");
    let other = p.root.parent().unwrap().join("otro-fuera.txt");
    std::fs::write(&outside, "F").unwrap();
    std::fs::write(&other, "O").unwrap();

    p.state.editor.explorer = false;
    let before = p.state.pending;
    let _ = update(&mut p.state, Event::Dropped(outside.clone()));
    assert_eq!(p.state.pending, before, "a hidden explorer ignores drops");

    p.state.editor.explorer = true;
    p.state.shell.side = shell::Side::Explorer;
    p.state.explorer.selection = BTreeSet::from([PathBuf::from("src/lib.rs")]);
    let _ = update(&mut p.state, Event::Dropped(outside.clone()));
    assert_eq!(p.state.pending, before + 1);

    // The job it starts: into the folder of the selected file.
    run_job(&mut p, ops::Job::Import(vec![outside], "src".into()));
    assert_eq!(
        std::fs::read_to_string(p.root.join("src/fuera.txt")).unwrap(),
        "F"
    );
    assert_eq!(
        p.state.explorer.selection,
        BTreeSet::from([PathBuf::from("src/fuera.txt")])
    );
}

#[test]
fn importing_nothing_or_something_unsafe_fails_with_a_reason_instead_of_doing_it() {
    let p = project("import-fail");
    let nothing = ops::import_all(&p.root, &[], Path::new(""));
    assert!(matches!(nothing, ops::Outcome::Failed(why) if why.contains("nada copiado")));
    let missing = ops::import_all(&p.root, &[p.root.join("../no-existe")], Path::new(""));
    assert!(matches!(missing, ops::Outcome::Failed(_)));
}
