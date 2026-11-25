//! SQLite for Colmena (ADR 0018): the only place that knows about `rusqlite`.
//!
//! A database is opened with the settings every Colmena file uses (WAL, foreign keys, a busy
//! timeout) and brought up to date with numbered migrations. A migration of a file that already
//! has data copies it aside first, so there is always a way back.
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

/// One step of the schema. `version` starts at 1 and grows by one.
pub struct Migration {
    pub version: u32,
    pub sql: &'static str,
}

/// What opening a database did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    pub from_version: u32,
    pub to_version: u32,
    /// The copy made before migrating existing data, when one was needed.
    pub backup: Option<PathBuf>,
}

fn message(error: rusqlite::Error) -> String {
    format!("SQLite: {error}")
}

/// Opens (creating it and its folder) and migrates the database at `path`.
pub fn open(path: &Path, migrations: &[Migration]) -> Result<(Connection, Opened), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("No se pudo crear {}: {error}", parent.display()))?;
    }
    let mut connection = Connection::open(path).map_err(message)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(message)?;
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;",
        )
        .map_err(message)?;
    let opened = migrate(&mut connection, path, migrations)?;
    Ok((connection, opened))
}

/// The schema version of an open database (0 for a new one).
pub fn schema_version(connection: &Connection) -> Result<u32, String> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
        .map_err(message)
}

fn migrate(
    connection: &mut Connection,
    path: &Path,
    migrations: &[Migration],
) -> Result<Opened, String> {
    let current = schema_version(connection)?;
    let latest = migrations.last().map_or(0, |last| last.version);
    if current > latest {
        return Err(format!(
            "Esta base es de una versión más nueva del producto (esquema {current}, esta versión \
             entiende hasta {latest}). No se modificó."
        ));
    }
    let mut opened = Opened {
        from_version: current,
        to_version: current,
        backup: None,
    };
    if current == latest {
        return Ok(opened);
    }
    if current > 0 {
        opened.backup = Some(backup(connection, path, current)?);
    }
    for migration in migrations.iter().filter(|m| m.version > current) {
        let transaction = connection.transaction().map_err(message)?;
        transaction.execute_batch(migration.sql).map_err(message)?;
        transaction
            .pragma_update(None, "user_version", migration.version)
            .map_err(message)?;
        transaction.commit().map_err(message)?;
        opened.to_version = migration.version;
    }
    Ok(opened)
}

/// A consistent copy of the database next to it (`VACUUM INTO`, so the WAL is included).
fn backup(connection: &Connection, path: &Path, version: u32) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "hive.db".to_owned());
    let target = path.with_file_name(format!("{name}.v{version}.bak"));
    if target.exists() {
        fs::remove_file(&target)
            .map_err(|error| format!("No se pudo reemplazar {}: {error}", target.display()))?;
    }
    connection
        .execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])
        .map_err(message)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: Migration = Migration {
        version: 1,
        sql: "CREATE TABLE a (id INTEGER PRIMARY KEY);",
    };
    const V2: Migration = Migration {
        version: 2,
        sql: "ALTER TABLE a ADD COLUMN name TEXT;",
    };

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-sqlite-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_new_database_is_created_and_migrated_without_a_backup() {
        let dir = dir("new");
        let (connection, opened) = open(&dir.join("x.db"), &[V1, V2]).unwrap();
        assert_eq!((opened.from_version, opened.to_version), (0, 2));
        assert!(opened.backup.is_none());
        assert_eq!(schema_version(&connection).unwrap(), 2);
        let mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn migrating_existing_data_keeps_a_copy_and_reopening_changes_nothing() {
        let dir = dir("old");
        let path = dir.join("x.db");
        let (connection, _) = open(&path, &[V1]).unwrap();
        connection
            .execute("INSERT INTO a (id) VALUES (7)", [])
            .unwrap();
        drop(connection);

        let (connection, opened) = open(&path, &[V1, V2]).unwrap();
        let backup = opened.backup.expect("existing data is copied first");
        assert!(backup.exists());
        let kept: i64 = connection
            .query_row("SELECT id FROM a", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kept, 7);
        drop(connection);

        let (_, again) = open(&path, &[V1, V2]).unwrap();
        assert!(again.backup.is_none(), "up to date: nothing to do");
        assert_eq!(again.from_version, again.to_version);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_database_from_a_newer_product_is_refused_untouched() {
        let dir = dir("newer");
        let path = dir.join("x.db");
        drop(open(&path, &[V1, V2]).unwrap());
        let error = open(&path, &[V1]).expect_err("must refuse");
        assert!(error.contains("más nueva"), "{error}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failing_migration_leaves_the_version_where_it_was() {
        let dir = dir("fail");
        let path = dir.join("x.db");
        drop(open(&path, &[V1]).unwrap());
        let broken = Migration {
            version: 2,
            sql: "CREATE TABLE b (id INTEGER); THIS IS NOT SQL;",
        };
        assert!(open(&path, &[V1, broken]).is_err());
        let (connection, _) = open(&path, &[V1]).unwrap();
        assert_eq!(schema_version(&connection).unwrap(), 1);
        let has_b: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'b'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_b, 0, "the half-applied step was rolled back");
        let _ = fs::remove_dir_all(dir);
    }
}
