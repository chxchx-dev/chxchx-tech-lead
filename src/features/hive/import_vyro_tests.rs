use super::*;
use crate::features::hive::{board, store};

fn project(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-vyro-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".vyro")).unwrap();
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(
        dir.join(".vyro/tasks.json"),
        r#"{"tasks":[
          {"id":"VYR-000","phase":"P0","title":"Base","priority":0,"depends_on":[],"owner_role":"qa","status":"done","acceptance":["a"],"evidence":"e","agent":"x","reviewer":"r","blocker":null,"reopen_reason":"why"},
          {"id":"VYR-001","phase":"P0","title":"Sigue","priority":1,"depends_on":["VYR-000"],"owner_role":"dev","status":"todo","acceptance":[],"evidence":"","agent":"","reviewer":"","blocker":null}
        ]}"#,
    ).unwrap();
    fs::write(
        dir.join("docs/DECISIONS.json"),
        r#"[{"id":"D-100","status":"abierta","question":"¿Motor?","outcome":"","why":"","decided_by":"","limits":"","links":[],"tasks":["VYR-001"]},
            {"id":"D-101","status":"decidida","question":"¿Idioma?","outcome":"es","why":"x","decided_by":"yo","limits":"","links":[],"tasks":[]}]"#,
    ).unwrap();
    dir
}

#[test]
fn imports_once_reads_only_and_a_later_decision_releases_the_card() {
    let root = project("one");
    let before = fs::read(root.join(".vyro/tasks.json")).unwrap();
    let home = root.join("home");
    let (db, _) = store::open_at(&store::path_in(&home, &root)).unwrap();

    let first = import(&db, &root).unwrap();
    assert_eq!(
        first,
        Summary {
            cards: 2,
            dependencies: 1,
            decisions: 2,
            waiting: 1
        }
    );
    let again = import(&db, &root).unwrap();
    assert_eq!(
        (again.cards, again.dependencies, again.waiting),
        (2, 0, 0),
        "idempotent"
    );
    assert_eq!(board::cards(&db).unwrap().len(), 2);
    assert_eq!(board::decisions(&db).unwrap().len(), 2);

    let card = board::card(&db, "VYR-001").unwrap();
    assert_eq!(
        (
            card.status.as_str(),
            card.depends_on.clone(),
            card.waits_for.clone()
        ),
        (
            "backlog",
            vec!["VYR-000".to_owned()],
            vec!["D-100".to_owned()]
        )
    );
    assert!(board::set_status(&db, "VYR-001", "doing", "me").is_err());
    let released = board::resolve(&db, "D-100", Some(("sqlite", "")), "me").unwrap();
    assert_eq!(
        released.cards,
        ["VYR-001"],
        "its dependency is already done"
    );

    assert_eq!(
        fs::read(root.join(".vyro/tasks.json")).unwrap(),
        before,
        "Vyro is untouched"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_folder_without_a_board_or_with_broken_json_is_a_clear_error() {
    let root = project("bad");
    let (db, _) = store::open_at(&store::path_in(&root.join("home"), &root)).unwrap();
    fs::write(root.join(".vyro/tasks.json"), "{no").unwrap();
    assert!(import(&db, &root).unwrap_err().contains("JSON"));
    fs::remove_file(root.join(".vyro/tasks.json")).unwrap();
    assert!(import(&db, &root).unwrap_err().contains("tasks.json"));
    let _ = fs::remove_dir_all(root);
}
