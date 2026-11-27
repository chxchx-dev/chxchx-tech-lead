use super::*;
use crate::features::hive::store;

fn project(name: &str) -> (PathBuf, Connection) {
    let dir = std::env::temp_dir().join(format!("chxchx-rag-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".ai")).unwrap();
    let (db, _) = store::open_at(&dir.join("home/x.db")).unwrap();
    (dir, db)
}

#[test]
fn finds_the_section_that_answers_with_its_heading_and_path() {
    let (dir, db) = project("find");
    fs::write(
        dir.join("README.md"),
        "# Producto\nUn instalador.\n\n## Tablero\nLas tarjetas dependen unas de otras.\n",
    )
    .unwrap();
    let stats = index(&db, &dir).unwrap();
    assert_eq!((stats.files, stats.chunks), (1, 2));
    let hits = search(&db, "tarjetas dependen", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(
        (hits[0].path.as_str(), hits[0].heading.as_str()),
        ("README.md", "Tablero")
    );
    assert!(search(&db, "\"; DROP TABLE -- *", 5).unwrap().is_empty());
    assert!(search(&db, "   ", 5).unwrap().is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn only_changed_files_are_read_again_and_removed_ones_leave_the_index() {
    let (dir, db) = project("incremental");
    fs::write(dir.join("a.md"), "# A\nprimero\n").unwrap();
    fs::write(dir.join("b.md"), "# B\nsegundo\n").unwrap();
    assert_eq!(index(&db, &dir).unwrap().files, 2);
    let again = index(&db, &dir).unwrap();
    assert_eq!((again.files, again.unchanged), (0, 2));

    fs::write(dir.join("a.md"), "# A\ntercero\n").unwrap();
    fs::remove_file(dir.join("b.md")).unwrap();
    let after = index(&db, &dir).unwrap();
    assert_eq!((after.files, after.unchanged, after.removed), (1, 0, 1));
    assert!(search(&db, "primero", 5).unwrap().is_empty());
    assert!(search(&db, "segundo", 5).unwrap().is_empty());
    assert_eq!(search(&db, "tercero", 5).unwrap().len(), 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn credentials_never_reach_the_index() {
    let (dir, db) = project("secrets");
    fs::write(dir.join(".env.md"), "# env\nzzzprivado\n").unwrap();
    fs::write(dir.join("credentials.txt"), "zzzprivado\n").unwrap();
    fs::write(
        dir.join("nota.md"),
        "# Nota\napi_key=sk-abcdefghijklmnopqrstuvwxyz123456 y zzzpublico\n",
    )
    .unwrap();
    index(&db, &dir).unwrap();
    assert!(search(&db, "zzzprivado", 5).unwrap().is_empty());
    assert!(
        search(&db, "abcdefghijklmnopqrstuvwxyz123456", 5)
            .unwrap()
            .is_empty()
    );
    assert_eq!(search(&db, "zzzpublico", 5).unwrap().len(), 1);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn headings_inside_code_fences_do_not_split_and_long_sections_do() {
    let text = format!(
        "# T\n```\n# no es título\n```\n\n{}",
        "palabra "
            .repeat(300)
            .repeat(2)
            .replace("palabra palabra", "palabra\n\npalabra")
    );
    let parts = chunks(&text);
    assert!(parts.iter().all(|(h, _)| h == "T"));
    assert!(parts.len() > 1, "a long section is cut at blank lines");
}
