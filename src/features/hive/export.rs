//! Optional `.ai/board.json` snapshot so agents without the CLI can read the board.
//! Read-only view of the database: it holds no secrets and the database stays the source of truth.
use super::board;
use rusqlite::Connection;
use serde_json::json;
use std::path::{Path, PathBuf};

pub fn board_json(db: &Connection) -> Result<String, String> {
    serde_json::to_string_pretty(&json!({
        "version": 1,
        "cards": board::cards(db)?,
        "decisions": board::decisions(db)?,
    }))
    .map(|text| text + "\n")
    .map_err(|error| error.to_string())
}

/// Writes `<root>/.ai/board.json` through a temporary file; skips the write when nothing changed.
pub fn write(db: &Connection, root: &Path) -> Result<PathBuf, String> {
    let target = root.join(".ai").join("board.json");
    let text = board_json(db)?;
    if std::fs::read_to_string(&target).is_ok_and(|current| current == text) {
        return Ok(target);
    }
    let folder = root.join(".ai");
    std::fs::create_dir_all(&folder)
        .map_err(|error| format!("No se pudo crear {}: {error}", folder.display()))?;
    let temporary = folder.join("board.json.tmp");
    std::fs::write(&temporary, text)
        .and_then(|()| std::fs::rename(&temporary, &target))
        .map_err(|error| format!("No se pudo escribir {}: {error}", target.display()))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hive::store;

    #[test]
    fn it_writes_the_board_once_and_is_stable() {
        let dir = std::env::temp_dir().join(format!("hive-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (db, _) = store::open_at(&dir.join("x.db")).unwrap();
        board::add_card(&db, "Una", "", "t").unwrap();
        board::block(&db, "C-001", "falta acceso", "t").unwrap();
        let path = write(&db, &dir).unwrap();
        let first = std::fs::read_to_string(&path).unwrap();
        assert!(
            first.contains("\"C-001\"") && first.contains("falta acceso"),
            "{first}"
        );
        write(&db, &dir).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), first);
        assert!(!dir.join(".ai/board.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_block_needs_a_reason_of_its_own() {
        let dir = std::env::temp_dir().join(format!("hive-block-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (db, _) = store::open_at(&dir.join("x.db")).unwrap();
        board::add_card(&db, "Una", "", "t").unwrap();
        assert!(board::block(&db, "C-001", "  ", "t").is_err());
        assert!(board::block(&db, "C-001", "Espera la decisión D-9", "t").is_err());
        board::block(&db, "C-001", "motivo", "t").unwrap();
        assert_eq!(board::card(&db, "C-001").unwrap().blocker, "motivo");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
