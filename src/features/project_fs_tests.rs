use super::*;

struct Sandbox {
    root: PathBuf,
    home: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!("chxchx-fs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("proyecto");
        let home = base.join("home");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&home).unwrap();
        Self {
            root: root.canonicalize().unwrap(),
            home: home.canonicalize().unwrap(),
        }
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.root.join(relative)).unwrap()
    }

    fn has(&self, relative: &str) -> bool {
        exists(&self.root.join(relative))
    }
}

fn paths(items: &[&str]) -> Vec<PathBuf> {
    items.iter().map(PathBuf::from).collect()
}

#[test]
fn names_that_would_break_on_some_system_are_refused_with_a_reason() {
    for good in [
        "a.txt",
        "README.md",
        "mi carpeta",
        ".gitignore",
        "ñandú.rs",
        "a.b.c",
        "COM10",
    ] {
        assert!(validate_name(good).is_ok(), "{good}");
    }
    for (bad, why) in [
        ("", "Escribe"),
        ("..", "Escribe"),
        ("a/b", "no puede contener"),
        ("a\\b", "no puede contener"),
        ("a:b", "no puede contener"),
        ("a?b", "no puede contener"),
        ("tab\t", "no puede contener"),
        (" a", "espacio"),
        ("a ", "espacio"),
        ("a.", "punto"),
        ("con", "reservado"),
        ("NUL.txt", "reservado"),
        (".env", "entorno"),
        (".env.local", "entorno"),
        (".git", "oculta"),
        ("node_modules", "oculta"),
    ] {
        let error = validate_name(bad).unwrap_err();
        assert!(error.contains(why), "{bad:?}: {error}");
    }
    assert!(
        validate_name(&"x".repeat(256))
            .unwrap_err()
            .contains("largo")
    );
}

#[test]
fn files_and_folders_are_created_where_asked_and_never_replace_anything() {
    let s = Sandbox::new("create");
    s.write("src/lib.rs", "uno");

    assert_eq!(
        create_file(&s.root, Path::new("src"), "nuevo.rs").unwrap(),
        Path::new("src/nuevo.rs")
    );
    assert_eq!(
        create_folder(&s.root, Path::new(""), "docs").unwrap(),
        Path::new("docs")
    );
    assert_eq!(
        create_file(&s.root, Path::new("docs"), "a.md").unwrap(),
        Path::new("docs/a.md")
    );
    assert!(s.has("src/nuevo.rs") && s.has("docs/a.md"));

    assert!(
        create_file(&s.root, Path::new("src"), "lib.rs")
            .unwrap_err()
            .contains("Ya existe")
    );
    assert_eq!(s.read("src/lib.rs"), "uno");
    assert!(
        create_folder(&s.root, Path::new("src"), "lib.rs")
            .unwrap_err()
            .contains("Ya existe")
    );
    assert!(
        create_file(&s.root, Path::new("src/lib.rs"), "x")
            .unwrap_err()
            .contains("no es una carpeta")
    );
    assert!(create_file(&s.root, Path::new("no-existe"), "x").is_err());
}

#[test]
fn renaming_keeps_the_content_and_refuses_to_overwrite() {
    let s = Sandbox::new("rename");
    s.write("a.txt", "A");
    s.write("b.txt", "B");
    s.write("carpeta/dentro.txt", "D");

    assert_eq!(
        rename(&s.root, Path::new("a.txt"), "c.txt").unwrap(),
        Path::new("c.txt")
    );
    assert_eq!((s.read("c.txt"), s.has("a.txt")), ("A".to_owned(), false));
    assert!(
        rename(&s.root, Path::new("c.txt"), "b.txt")
            .unwrap_err()
            .contains("Ya existe")
    );
    assert_eq!(s.read("b.txt"), "B");
    assert_eq!(
        rename(&s.root, Path::new("carpeta"), "otra").unwrap(),
        Path::new("otra")
    );
    assert_eq!(s.read("otra/dentro.txt"), "D");
    assert_eq!(
        rename(&s.root, Path::new("b.txt"), "b.txt").unwrap(),
        Path::new("b.txt")
    );
    assert!(rename(&s.root, Path::new("b.txt"), "a/b").is_err());
    assert!(
        rename(&s.root, Path::new(""), "raiz")
            .unwrap_err()
            .contains("raíz")
    );
}

#[test]
fn a_free_name_is_found_next_to_the_original_keeping_the_extension() {
    let s = Sandbox::new("unique");
    s.write("a.txt", "");
    assert_eq!(unique_name(&s.root, "nuevo.txt"), "nuevo.txt");
    assert_eq!(unique_name(&s.root, "a.txt"), "a copia.txt");
    s.write("a copia.txt", "");
    assert_eq!(unique_name(&s.root, "a.txt"), "a copia 2.txt");
    s.write("Makefile", "");
    assert_eq!(unique_name(&s.root, "Makefile"), "Makefile copia");
    s.write(".gitignore", "");
    assert_eq!(unique_name(&s.root, ".gitignore"), ".gitignore copia");
}

#[test]
fn duplicating_and_pasting_copies_whole_trees_and_never_overwrite() {
    let s = Sandbox::new("copy");
    s.write("src/a.rs", "A");
    s.write("src/sub/b.rs", "B");
    s.write("otra/x.txt", "X");
    fs::create_dir_all(s.root.join("vacia")).unwrap();

    assert_eq!(
        duplicate(&s.root, Path::new("src/a.rs")).unwrap(),
        Path::new("src/a copia.rs")
    );
    assert_eq!(s.read("src/a copia.rs"), "A");
    assert_eq!(
        duplicate(&s.root, Path::new("src")).unwrap(),
        Path::new("src copia")
    );
    assert_eq!(s.read("src copia/sub/b.rs"), "B");

    let pasted = copy_into(&s.root, &paths(&["src/a.rs", "src/sub"]), Path::new("otra")).unwrap();
    assert_eq!(pasted, paths(&["otra/a.rs", "otra/sub"]));
    assert_eq!(s.read("otra/sub/b.rs"), "B");
    // The same again does not overwrite: free variants appear.
    let again = copy_into(&s.root, &paths(&["src/a.rs"]), Path::new("otra")).unwrap();
    assert_eq!(again, paths(&["otra/a copia.rs"]));
    assert_eq!(s.read("src/a.rs"), "A");
    assert_eq!(
        copy_into(&s.root, &paths(&["vacia"]), Path::new("otra")).unwrap(),
        paths(&["otra/vacia"])
    );
}

#[test]
fn a_folder_cannot_be_copied_or_moved_into_itself_or_its_children() {
    let s = Sandbox::new("self");
    s.write("a/b/c.txt", "C");
    for destination in ["a", "a/b"] {
        let error = copy_into(&s.root, &paths(&["a"]), Path::new(destination)).unwrap_err();
        assert!(error.contains("dentro de sí misma"), "{error}");
        assert!(move_into(&s.root, &paths(&["a"]), Path::new(destination)).is_err());
    }
    assert!(!s.has("a/a"));
}

#[test]
fn moving_relocates_refuses_conflicts_and_keeps_a_same_folder_move_a_noop() {
    let s = Sandbox::new("move");
    s.write("a.txt", "A");
    s.write("dir/b.txt", "B");
    s.write("dir/a.txt", "otro");
    fs::create_dir_all(s.root.join("destino")).unwrap();

    assert!(
        move_into(&s.root, &paths(&["a.txt"]), Path::new("dir"))
            .unwrap_err()
            .contains("Ya existe")
    );
    assert_eq!(
        (s.read("a.txt"), s.read("dir/a.txt")),
        ("A".to_owned(), "otro".to_owned())
    );
    let moved = move_into(&s.root, &paths(&["dir"]), Path::new("destino")).unwrap();
    assert_eq!(moved, paths(&["destino/dir"]));
    assert_eq!(s.read("destino/dir/b.txt"), "B");
    assert!(!s.has("dir"));
    assert_eq!(
        move_into(&s.root, &paths(&["a.txt"]), Path::new("")).unwrap(),
        paths(&["a.txt"])
    );
    assert_eq!(s.read("a.txt"), "A");
    assert!(move_into(&s.root, &[], Path::new("")).is_err());
}

#[test]
fn nothing_leaves_the_project_or_touches_what_the_explorer_hides() {
    let s = Sandbox::new("limits");
    s.write("a.txt", "A");
    s.write(".git/config", "g");
    s.write(".env", "SECRETO=1");
    s.write("node_modules/m.js", "m");

    for relative in [
        "../fuera.txt",
        "/etc/passwd",
        ".git",
        ".git/config",
        ".env",
        "node_modules/m.js",
    ] {
        assert!(
            rename(&s.root, Path::new(relative), "x").is_err(),
            "{relative}"
        );
        assert!(
            duplicate(&s.root, Path::new(relative)).is_err(),
            "{relative}"
        );
        assert!(
            trash(&s.root, &s.home, Path::new(relative)).is_err(),
            "{relative}"
        );
    }
    assert!(create_file(&s.root, Path::new(".git"), "x").is_err());
    assert!(copy_into(&s.root, &paths(&["a.txt"]), Path::new("../")).is_err());
    assert_eq!(s.read(".env"), "SECRETO=1");
}

#[test]
fn environment_files_inside_a_copied_folder_are_left_out() {
    let s = Sandbox::new("env-copy");
    s.write("app/main.rs", "m");
    s.write("app/.env", "TOKEN=1");
    s.write("app/.env.local", "TOKEN=2");

    copy_into(&s.root, &paths(&["app"]), Path::new("")).unwrap();

    assert!(s.has("app copia/main.rs"));
    assert!(!s.has("app copia/.env") && !s.has("app copia/.env.local"));
}

#[cfg(unix)]
#[test]
fn links_are_never_followed_copied_or_moved_through() {
    let s = Sandbox::new("links");
    let outside = s.root.parent().unwrap().join("fuera");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secreto.txt"), "S").unwrap();
    s.write("real/a.txt", "A");
    std::os::unix::fs::symlink(&outside, s.root.join("enlace")).unwrap();
    std::os::unix::fs::symlink(&outside, s.root.join("real/dentro")).unwrap();

    assert!(rename(&s.root, Path::new("enlace"), "x").is_err());
    assert!(trash(&s.root, &s.home, Path::new("enlace")).is_err());
    assert!(copy_into(&s.root, &paths(&["real"]), Path::new("enlace")).is_err());
    assert!(create_file(&s.root, Path::new("enlace"), "nuevo").is_err());
    // A link inside a copied tree is skipped, not followed.
    copy_into(&s.root, &paths(&["real"]), Path::new("")).unwrap();
    assert!(s.has("real copia/a.txt") && !s.has("real copia/dentro"));
    assert!(!outside.join("nuevo").exists());
    assert_eq!(
        fs::read_to_string(outside.join("secreto.txt")).unwrap(),
        "S"
    );
}

#[test]
fn deleting_goes_to_a_trash_that_can_put_it_back() {
    let s = Sandbox::new("trash");
    s.write("dir/a.txt", "A");
    s.write("b.txt", "B");

    let file = trash(&s.root, &s.home, Path::new("b.txt")).unwrap();
    let folder = trash(&s.root, &s.home, Path::new("dir")).unwrap();
    assert!(!s.has("b.txt") && !s.has("dir"));
    assert!(file.location.starts_with(s.home.join("trash")) && file.location.is_file());
    assert_eq!(
        fs::read_to_string(folder.location.join("a.txt")).unwrap(),
        "A"
    );

    assert_eq!(restore(&s.root, &file).unwrap(), Path::new("b.txt"));
    assert_eq!(restore(&s.root, &folder).unwrap(), Path::new("dir"));
    assert_eq!(
        (s.read("b.txt"), s.read("dir/a.txt")),
        ("B".to_owned(), "A".to_owned())
    );
    assert!(
        restore(&s.root, &file).is_err(),
        "it is no longer in the trash"
    );
}

#[test]
fn restoring_never_replaces_what_took_the_place_and_reports_a_missing_folder() {
    let s = Sandbox::new("restore");
    s.write("dir/a.txt", "A");
    let gone = trash(&s.root, &s.home, Path::new("dir/a.txt")).unwrap();

    s.write("dir/a.txt", "nuevo");
    assert!(restore(&s.root, &gone).unwrap_err().contains("Ya existe"));
    assert_eq!(s.read("dir/a.txt"), "nuevo");
    assert!(gone.location.is_file(), "the trashed copy is kept");

    fs::remove_dir_all(s.root.join("dir")).unwrap();
    assert!(
        restore(&s.root, &gone)
            .unwrap_err()
            .contains("ya no existe")
    );
}

#[test]
fn the_trash_must_be_an_absolute_folder_without_links() {
    let s = Sandbox::new("trash-home");
    s.write("a.txt", "A");
    assert!(trash(&s.root, Path::new("relativa"), Path::new("a.txt")).is_err());
    assert!(s.has("a.txt"));
}
