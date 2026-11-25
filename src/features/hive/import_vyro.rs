//! Reads the board a Vyro project keeps (`.vyro/tasks.json`, `docs/DECISIONS.json`) into Colmena.
//!
//! Strictly one way: Vyro's files are only read, never written. Importing twice updates the same
//! cards (the Vyro id is the card key) instead of duplicating them.
use super::board::{fail, new_id, now, record};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::fs;
use std::path::Path;

const LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub cards: usize,
    pub dependencies: usize,
    pub decisions: usize,
    pub waiting: usize,
}

fn read(path: &Path) -> Result<Option<Value>, String> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("No se pudo leer {}: {error}", path.display())),
    };
    if !meta.is_file() || meta.len() > LIMIT {
        return Err(format!(
            "{} no es un archivo normal o es demasiado grande.",
            path.display()
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("No se pudo leer {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| format!("{} no es JSON válido: {e}", path.display()))
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_owned()
}

fn list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn card_status(source: &str) -> &'static str {
    match source {
        "doing" => "doing",
        "done" => "done",
        "blocked" => "blocked",
        _ => "backlog",
    }
}

fn row_id(db: &Connection, table: &str, key: &str) -> Option<String> {
    db.query_row(
        &format!("SELECT id FROM {table} WHERE key = ?1"),
        [key],
        |r| r.get(0),
    )
    .ok()
}

pub fn import(db: &Connection, root: &Path) -> Result<Summary, String> {
    let tasks = read(&root.join(".vyro").join("tasks.json"))?
        .ok_or("No hay .vyro/tasks.json en ese proyecto.")?;
    let decisions = read(&root.join("docs").join("DECISIONS.json"))?;
    let tasks = tasks
        .get("tasks")
        .and_then(Value::as_array)
        .ok_or("tasks.json no trae «tasks».")?;
    let mut summary = Summary::default();
    let tx = db.unchecked_transaction().map_err(fail)?;
    let at = now();

    for task in tasks {
        let key = text(task, "id");
        if key.is_empty() {
            continue;
        }
        let phase = text(task, "phase");
        if !phase.is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO phase (id, title) VALUES (?1, ?1)",
                [&phase],
            )
            .map_err(fail)?;
        }
        let acceptance =
            serde_json::to_string(&list(task, "acceptance")).map_err(|e| e.to_string())?;
        let values = params![
            text(task, "title"),
            card_status(&text(task, "status")),
            task.get("priority").and_then(Value::as_i64).unwrap_or(0),
            text(task, "owner_role"),
            acceptance,
            text(task, "evidence"),
            text(task, "agent"),
            text(task, "reviewer"),
            text(task, "blocker"),
            text(task, "reopen_reason"),
            (!phase.is_empty()).then_some(phase.as_str()),
            at,
            key
        ];
        if row_id(&tx, "card", &key).is_some() {
            tx.execute(
                "UPDATE card SET title=?1, status=?2, priority=?3, role=?4, acceptance=?5, evidence=?6, agent=?7, reviewer=?8, blocker=?9, body=?10, phase_id=?11, updated_at=?12 WHERE key=?13",
                values,
            ).map_err(fail)?;
        } else {
            tx.execute(
                "INSERT INTO card (title, status, priority, role, acceptance, evidence, agent, reviewer, blocker, body, phase_id, created_at, updated_at, key, id) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12,?13,?14)",
                params![
                    text(task, "title"), card_status(&text(task, "status")), task.get("priority").and_then(Value::as_i64).unwrap_or(0),
                    text(task, "owner_role"), acceptance, text(task, "evidence"), text(task, "agent"),
                    text(task, "reviewer"), text(task, "blocker"), text(task, "reopen_reason"),
                    (!phase.is_empty()).then_some(phase.as_str()), at, key, new_id()
                ],
            ).map_err(fail)?;
        }
        summary.cards += 1;
    }
    for task in tasks {
        let Some(id) = row_id(&tx, "card", &text(task, "id")) else {
            continue;
        };
        for dep in list(task, "depends_on") {
            if let Some(dep_id) = row_id(&tx, "card", &dep).filter(|d| *d != id) {
                summary.dependencies += tx
                    .execute(
                        "INSERT OR IGNORE INTO card_dep (card_id, depends_on) VALUES (?1, ?2)",
                        [&id, &dep_id],
                    )
                    .map_err(fail)?;
            }
        }
    }
    for decision in decisions
        .as_ref()
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let key = text(decision, "id");
        if key.is_empty() {
            continue;
        }
        let decided = text(decision, "status") == "decidida";
        let status = if decided { "decided" } else { "proposed" };
        if row_id(&tx, "decision", &key).is_some() {
            tx.execute(
                "UPDATE decision SET question=?1, outcome=?2, why=?3, decided_by=?4, limits=?5, status=?6 WHERE key=?7",
                params![text(decision, "question"), text(decision, "outcome"), text(decision, "why"), text(decision, "decided_by"), text(decision, "limits"), status, key],
            ).map_err(fail)?;
        } else {
            tx.execute(
                "INSERT INTO decision (question, outcome, why, decided_by, limits, status, key, id, origin, created_at, decided_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'vyro',?9, ?10)",
                params![text(decision, "question"), text(decision, "outcome"), text(decision, "why"), text(decision, "decided_by"), text(decision, "limits"), status, key, new_id(), at, decided.then_some(at)],
            ).map_err(fail)?;
        }
        summary.decisions += 1;
        if !decided {
            // An open decision holds back the cards still in the backlog that it mentions.
            let did = row_id(&tx, "decision", &key).ok_or("decisión perdida")?;
            for task in list(decision, "tasks") {
                if let Some(cid) = row_id(&tx, "card", &task) {
                    let status: String = tx
                        .query_row("SELECT status FROM card WHERE id=?1", [&cid], |r| r.get(0))
                        .map_err(fail)?;
                    if status == "backlog" {
                        summary.waiting += tx
                            .execute("INSERT OR IGNORE INTO card_decision (card_id, decision_id) VALUES (?1, ?2)", [&cid, &did])
                            .map_err(fail)?;
                    }
                }
            }
        }
    }
    record(&tx, "import", "import.vyro", "")?;
    tx.commit().map_err(fail)?;
    Ok(summary)
}

#[cfg(test)]
#[path = "import_vyro_tests.rs"]
mod tests;
