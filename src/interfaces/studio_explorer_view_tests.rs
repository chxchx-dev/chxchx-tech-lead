use super::*;
use crate::interfaces::studio::explorer::Clipboard;

fn labels(rows: &[Row]) -> Vec<&str> {
    rows.iter()
        .filter_map(|row| match row {
            Row::Item { label, .. } => Some(label.as_str()),
            Row::Separator => None,
        })
        .collect()
}

fn enabled(rows: &[Row], wanted: &str) -> Option<bool> {
    rows.iter().find_map(|row| match row {
        Row::Item { label, enabled, .. } if label == wanted => Some(*enabled),
        _ => None,
    })
}

fn file(path: &str) -> Target {
    Target::Item {
        path: path.into(),
        directory: false,
    }
}

fn folder(path: &str) -> Target {
    Target::Item {
        path: path.into(),
        directory: true,
    }
}

#[test]
fn a_file_offers_what_editors_offer_for_a_file() {
    let state = editor::fixture();
    let rows = rows(&state, &file("src/lib.rs"));

    assert_eq!(
        labels(&rows),
        [
            "Abrir",
            "Renombrar",
            "Duplicar",
            "Copiar",
            "Cortar",
            "Pegar",
            "Copiar nombre",
            "Copiar ruta",
            "Copiar ruta relativa",
            "Mostrar en el explorador de archivos",
            "Eliminar",
        ]
    );
    assert!(
        !labels(&rows).contains(&"Nuevo archivo"),
        "a file cannot hold new files"
    );
}

#[test]
fn a_folder_also_creates_things_and_toggles_between_expand_and_collapse() {
    let mut state = editor::fixture();
    let closed = rows(&state, &folder("src"));
    assert_eq!(
        &labels(&closed)[..3],
        ["Nuevo archivo", "Nueva carpeta", "Expandir"]
    );

    state.editor.expanded.insert("src".into());
    assert!(labels(&rows(&state, &folder("src"))).contains(&"Contraer"));
}

#[test]
fn the_empty_area_is_the_project_folder_and_offers_undo_only_when_there_is_something() {
    let mut state = editor::fixture();
    let rows_now = rows(&state, &Target::Root);
    assert_eq!(
        labels(&rows_now),
        [
            "Nuevo archivo",
            "Nueva carpeta",
            "Pegar",
            "Copiar la ruta del proyecto",
            "Mostrar en el explorador de archivos",
            "Actualizar",
            "Contraer todo",
        ]
    );

    state
        .explorer
        .trashed
        .push(vec![crate::features::project_fs::Trashed {
            original: "a.txt".into(),
            location: "/tmp/x".into(),
        }]);
    assert!(labels(&rows(&state, &Target::Root)).contains(&"Deshacer la última eliminación"));
}

#[test]
fn paste_needs_something_copied_and_a_multiple_selection_cannot_be_renamed() {
    let mut state = editor::fixture();
    assert_eq!(enabled(&rows(&state, &file("a.rs")), "Pegar"), Some(false));
    state.explorer.clipboard = Some(Clipboard {
        paths: vec!["b.rs".into()],
        cut: false,
    });
    assert_eq!(enabled(&rows(&state, &file("a.rs")), "Pegar"), Some(true));

    state.explorer.selection = ["a.rs".into(), "b.rs".into()].into_iter().collect();
    let many = rows(&state, &file("a.rs"));
    assert_eq!(enabled(&many, "Renombrar"), Some(false));
    assert_eq!(
        enabled(&many, "Mostrar en el explorador de archivos"),
        Some(false)
    );
    assert!(labels(&many).contains(&"Eliminar 2 elementos"));
    assert!(!labels(&many).contains(&"Abrir"), "open is for one item");
}

#[test]
fn the_shortcuts_are_shown_next_to_the_actions_that_have_one() {
    let state = editor::fixture();
    keymap::apply(&Default::default());
    let rows = rows(&state, &file("a.rs"));
    let hint = |wanted: &str| {
        rows.iter().find_map(|row| match row {
            Row::Item { label, hint, .. } if label == wanted => Some(hint.clone()),
            _ => None,
        })
    };

    assert_eq!(hint("Renombrar").as_deref(), Some("F2"));
    assert_eq!(hint("Eliminar").as_deref(), Some("Supr"));
    assert_eq!(hint("Duplicar").as_deref(), Some(""));
    assert!(hint("Copiar").is_some_and(|text| text.contains('C')));
}

#[test]
fn the_menu_opens_at_the_pointer_and_never_leaves_the_panel() {
    let panel = Size::new(300.0, 500.0);
    assert_eq!(
        place(Point::new(40.0, 60.0), panel, 200.0),
        Point::new(40.0, 60.0)
    );
    // Near the right edge it shifts left; near the bottom it opens upward.
    assert_eq!(
        place(Point::new(290.0, 60.0), panel, 200.0).x,
        300.0 - MENU_WIDTH - 4.0
    );
    assert_eq!(place(Point::new(40.0, 450.0), panel, 200.0).y, 250.0);
    // A panel too small for it still starts inside.
    let tiny = place(Point::new(250.0, 250.0), Size::new(120.0, 100.0), 300.0);
    assert!(tiny.x >= 4.0 && tiny.y >= 4.0);
}
