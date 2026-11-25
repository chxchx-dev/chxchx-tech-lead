use super::*;
use crate::features::hive::store;
use std::path::{Path, PathBuf};

fn db(name: &str) -> (Connection, PathBuf) {
    let home = std::env::temp_dir().join(format!("chxchx-board-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let (db, _) = store::open_at(&store::path_in(&home, Path::new("/work/a"))).unwrap();
    (db, home)
}

fn status(db: &Connection, key: &str) -> String {
    card(db, key).unwrap().status
}

#[test]
fn keys_are_sequential_and_titles_required() {
    let (db, home) = db("keys");
    assert_eq!(add_card(&db, "Uno", "", "me").unwrap(), "C-001");
    assert_eq!(add_card(&db, " Dos ", "backend", "me").unwrap(), "C-002");
    assert!(add_card(&db, "  ", "", "me").is_err());
    assert_eq!(add_decision(&db, "¿A o B?", &[], "me").unwrap(), "D-001");
    assert_eq!(card(&db, "C-002").unwrap().title, "Dos");
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn dependencies_block_progress_refuse_cycles_and_promote_on_done() {
    let (db, home) = db("deps");
    for title in ["A", "B", "C"] {
        add_card(&db, title, "", "me").unwrap();
    }
    add_dependency(&db, "C-002", "C-001").unwrap();
    add_dependency(&db, "C-003", "C-002").unwrap();
    assert!(add_dependency(&db, "C-001", "C-003").is_err(), "cycle");
    assert!(add_dependency(&db, "C-001", "C-001").is_err(), "self");
    assert!(set_status(&db, "C-002", "doing", "me").is_err());
    assert!(set_status(&db, "C-009", "doing", "me").is_err());
    assert!(set_status(&db, "C-001", "weird", "me").is_err());

    set_status(&db, "C-001", "doing", "me").unwrap();
    let released = set_status(&db, "C-001", "done", "me").unwrap();
    assert_eq!(released.cards, ["C-002"]);
    assert_eq!(status(&db, "C-002"), "ready");
    assert_eq!(status(&db, "C-003"), "backlog", "still waits for C-002");
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn deciding_releases_waiting_cards_in_chain() {
    let (db, home) = db("chain");
    for title in ["A", "B", "C"] {
        add_card(&db, title, "", "me").unwrap();
    }
    add_dependency(&db, "C-003", "C-002").unwrap();
    add_decision(
        &db,
        "¿Qué motor?",
        &["sqlite".into(), "otro".into()],
        "agent",
    )
    .unwrap();
    wait_for(&db, "C-002", "D-001", "agent").unwrap();
    wait_for(&db, "C-003", "D-001", "agent").unwrap();
    assert_eq!(status(&db, "C-002"), "blocked");
    assert!(
        set_status(&db, "C-002", "ready", "me")
            .unwrap_err()
            .contains("D-001")
    );

    assert!(resolve(&db, "D-001", Some((" ", "")), "me").is_err());
    let released = resolve(&db, "D-001", Some(("sqlite", "es liviano")), "me").unwrap();
    // C-002 is free to go; C-003 still depends on C-002, so it only goes back to the backlog.
    assert_eq!(released.cards, ["C-002", "C-003"]);
    assert_eq!(status(&db, "C-002"), "ready");
    assert_eq!(status(&db, "C-003"), "backlog");
    assert_eq!(card(&db, "C-002").unwrap().waits_for.len(), 0);
    let decided = decision(&db, "D-001").unwrap();
    assert_eq!(
        (decided.status.as_str(), decided.outcome.as_str()),
        ("decided", "sqlite")
    );
    assert!(
        resolve(&db, "D-001", Some(("otro", "")), "me").is_err(),
        "closed once"
    );
    assert!(
        wait_for(&db, "C-001", "D-001", "me").is_err(),
        "no waiting on a closed one"
    );
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn a_card_waiting_for_two_decisions_needs_both_and_discarding_releases() {
    let (db, home) = db("two");
    add_card(&db, "A", "", "me").unwrap();
    add_decision(&db, "Uno", &[], "me").unwrap();
    add_decision(&db, "Dos", &[], "me").unwrap();
    wait_for(&db, "C-001", "D-001", "me").unwrap();
    wait_for(&db, "C-001", "D-002", "me").unwrap();
    assert!(
        resolve(&db, "D-001", Some(("sí", "")), "me")
            .unwrap()
            .cards
            .is_empty()
    );
    assert_eq!(status(&db, "C-001"), "blocked");
    assert_eq!(resolve(&db, "D-002", None, "me").unwrap().cards, ["C-001"]);
    assert_eq!(decision(&db, "D-002").unwrap().status, "discarded");
    assert_eq!(status(&db, "C-001"), "ready");
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn a_blocker_written_by_a_person_is_not_lifted_by_a_decision() {
    let (db, home) = db("human");
    add_card(&db, "A", "", "me").unwrap();
    add_decision(&db, "Uno", &[], "me").unwrap();
    wait_for(&db, "C-001", "D-001", "me").unwrap();
    db.execute("UPDATE card SET blocker = 'Falta acceso al servidor'", [])
        .unwrap();
    assert!(
        resolve(&db, "D-001", Some(("sí", "")), "me")
            .unwrap()
            .cards
            .is_empty()
    );
    assert_eq!(status(&db, "C-001"), "blocked");
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn every_change_leaves_an_event() {
    let (db, home) = db("events");
    add_card(&db, "A", "", "agent").unwrap();
    set_status(&db, "C-001", "ready", "agent").unwrap();
    let kinds: Vec<String> = db
        .prepare("SELECT kind FROM event ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert_eq!(kinds, ["card.added", "card.ready"]);
    let _ = std::fs::remove_dir_all(home);
}
