//! The per-project database: where it lives, its schema, and how it is removed.
use crate::features::project_profiles::application_home;
use crate::features::studio_preferences::project_key;
use crate::integrations::sqlite::{self, Migration, Opened};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

/// Folder (inside the application data) that holds every project's database.
pub const FOLDER: &str = "hive";

/// The schema. Ids are text so a card made on another PC never collides (ADR 0019): every row that
/// can be created remotely carries where it came from.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: "
CREATE TABLE phase (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    note TEXT NOT NULL DEFAULT ''
);
CREATE TABLE card (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    phase_id TEXT REFERENCES phase(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'backlog'
        CHECK (status IN ('backlog','ready','doing','review','done','blocked')),
    priority INTEGER NOT NULL DEFAULT 0,
    role TEXT NOT NULL DEFAULT '',
    acceptance TEXT NOT NULL DEFAULT '[]',
    evidence TEXT NOT NULL DEFAULT '',
    agent TEXT NOT NULL DEFAULT '',
    reviewer TEXT NOT NULL DEFAULT '',
    blocker TEXT NOT NULL DEFAULT '',
    assigned_node TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE card_dep (
    card_id TEXT NOT NULL REFERENCES card(id) ON DELETE CASCADE,
    depends_on TEXT NOT NULL REFERENCES card(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, depends_on),
    CHECK (card_id <> depends_on)
);
CREATE TABLE decision (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    question TEXT NOT NULL,
    options TEXT NOT NULL DEFAULT '[]',
    outcome TEXT NOT NULL DEFAULT '',
    why TEXT NOT NULL DEFAULT '',
    limits TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'proposed'
        CHECK (status IN ('proposed','decided','discarded')),
    decided_by TEXT NOT NULL DEFAULT '',
    origin TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    decided_at INTEGER
);
CREATE TABLE event (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    origin_node TEXT NOT NULL DEFAULT '',
    actor TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL,
    subject TEXT NOT NULL DEFAULT '',
    data TEXT NOT NULL DEFAULT '{}',
    at INTEGER NOT NULL
);
CREATE INDEX card_status ON card(status);
CREATE INDEX decision_status ON decision(status);
CREATE INDEX event_subject ON event(subject);
",
    },
    Migration {
        version: 2,
        // A card can wait for decisions the person has not made yet; deciding releases it.
        sql: "
CREATE TABLE card_decision (
    card_id TEXT NOT NULL REFERENCES card(id) ON DELETE CASCADE,
    decision_id TEXT NOT NULL REFERENCES decision(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, decision_id)
);
CREATE INDEX card_decision_decision ON card_decision(decision_id);
",
    },
];

/// Where the database of the project at `root` lives, under `home`.
pub fn path_in(home: &Path, root: &Path) -> PathBuf {
    home.join(FOLDER).join(format!("{}.db", project_key(root)))
}

/// Where it lives in this installation, if there is a data folder.
pub fn path(root: &Path) -> Option<PathBuf> {
    application_home().map(|home| path_in(&home, root))
}

/// Whether the project already has a database (answered without creating anything).
pub fn exists(root: &Path) -> bool {
    path(root).is_some_and(|path| path.is_file())
}

/// Size on disk of the project's database and its side files, in bytes.
pub fn size(root: &Path) -> u64 {
    path(root).map_or(0, |path| size_of(&path))
}

fn size_of(path: &Path) -> u64 {
    side_files(path)
        .iter()
        .filter_map(|file| fs::metadata(file).ok())
        .map(|meta| meta.len())
        .sum()
}

fn side_files(path: &Path) -> Vec<PathBuf> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    ["", "-wal", "-shm"]
        .iter()
        .map(|suffix| path.with_file_name(format!("{name}{suffix}")))
        .collect()
}

/// Opens (and creates or migrates) the database of the project at `root`.
pub fn open(root: &Path) -> Result<(Connection, Opened), String> {
    let path = path(root).ok_or("No se encontró la carpeta de datos de Studio.")?;
    open_at(&path)
}

pub fn open_at(path: &Path) -> Result<(Connection, Opened), String> {
    sqlite::open(path, MIGRATIONS)
}

/// Deletes the project's database after leaving a copy under `backups/` in the same folder. Returns
/// the copy. Nothing is removed when the copy cannot be made.
pub fn remove(root: &Path) -> Result<Option<PathBuf>, String> {
    let Some(path) = path(root) else {
        return Ok(None);
    };
    remove_at(&path)
}

pub fn remove_at(path: &Path) -> Result<Option<PathBuf>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let folder = path
        .parent()
        .ok_or("La base no tiene carpeta.")?
        .join("backups");
    fs::create_dir_all(&folder)
        .map_err(|error| format!("No se pudo crear {}: {error}", folder.display()))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let name = path
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let copy = folder.join(format!("{name}-{stamp}.db"));
    {
        let (connection, _) = open_at(path)?;
        connection
            .execute("VACUUM INTO ?1", [copy.to_string_lossy().as_ref()])
            .map_err(|error| format!("No se pudo guardar la copia: {error}"))?;
    }
    for file in side_files(path) {
        if file.exists() {
            fs::remove_file(&file)
                .map_err(|error| format!("No se pudo borrar {}: {error}", file.display()))?;
        }
    }
    Ok(Some(copy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-hive-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn each_project_has_its_own_file_inside_the_data_folder() {
        let home = home("paths");
        let a = path_in(&home, Path::new("/work/a"));
        let b = path_in(&home, Path::new("/work/b"));
        assert_ne!(a, b);
        assert!(a.starts_with(home.join(FOLDER)));
        assert_eq!(a, path_in(&home, Path::new("/work/a")), "stable");
    }

    #[test]
    fn asking_does_not_create_anything() {
        let home = home("quiet");
        let path = path_in(&home, Path::new("/work/a"));
        assert!(!path.exists() && !home.exists());
        assert_eq!(size_of(&path), 0);
    }

    #[test]
    fn the_schema_enforces_states_and_dependencies() {
        let home = home("schema");
        let (db, opened) = open_at(&path_in(&home, Path::new("/work/a"))).unwrap();
        assert_eq!(opened.to_version, 2);
        let card = |id: &str, status: &str| {
            db.execute(
                "INSERT INTO card (id, key, title, status, created_at, updated_at) VALUES (?1, ?1, 't', ?2, 0, 0)",
                [id, status],
            )
        };
        assert!(card("A", "done").is_ok());
        assert!(card("B", "nope").is_err(), "unknown status is refused");
        assert!(card("C", "ready").is_ok());
        assert!(
            db.execute("INSERT INTO card_dep VALUES ('C', 'A')", [])
                .is_ok()
        );
        assert!(
            db.execute("INSERT INTO card_dep VALUES ('C', 'C')", [])
                .is_err(),
            "no self dependency"
        );
        assert!(
            db.execute("INSERT INTO card_dep VALUES ('C', 'ghost')", [])
                .is_err(),
            "foreign keys are on"
        );
        db.execute("DELETE FROM card WHERE id = 'A'", []).unwrap();
        let left: i64 = db
            .query_row("SELECT count(*) FROM card_dep", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0, "a removed card takes its links with it");
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn full_text_search_is_compiled_in() {
        let (db, _) = open_at(&home("fts").join("x.db")).unwrap();
        db.execute_batch("CREATE VIRTUAL TABLE t USING fts5(body); INSERT INTO t VALUES ('el tablero de decisiones');")
            .expect("FTS5 must be part of the bundled SQLite");
        let hits: i64 = db
            .query_row("SELECT count(*) FROM t WHERE t MATCH 'tablero'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(hits, 1);
    }

    #[test]
    fn removing_keeps_a_copy_first_and_removing_nothing_is_fine() {
        let home = home("remove");
        let path = path_in(&home, Path::new("/work/a"));
        assert_eq!(remove_at(&path).unwrap(), None, "nothing to remove");
        let (db, _) = open_at(&path).unwrap();
        db.execute("INSERT INTO phase (id, title) VALUES ('P0', 'Base')", [])
            .unwrap();
        drop(db);
        let copy = remove_at(&path).unwrap().expect("a copy is left");
        assert!(!path.exists());
        let (kept, _) = open_at(&copy).unwrap();
        let title: String = kept
            .query_row("SELECT title FROM phase", [], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "Base");
        let _ = fs::remove_dir_all(home);
    }
}
