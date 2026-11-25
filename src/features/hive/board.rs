//! The board of one project: cards, their dependencies and the decisions they wait for.
//!
//! Rules that live here and nowhere else (the CLI and Studio only call these functions):
//! - a card cannot move forward while a dependency is not done or a decision is still open;
//! - finishing a card promotes the cards that were only waiting for it;
//! - deciding releases, in chain, every card that was waiting for that decision;
//! - a decision is a record. It never grants a permission or runs anything.
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

pub const STATUSES: [&str; 6] = ["backlog", "ready", "doing", "review", "done", "blocked"];
const WAITING: &str = "Espera la decisión";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Card {
    pub key: String,
    pub title: String,
    pub status: String,
    pub priority: i64,
    pub role: String,
    pub agent: String,
    pub blocker: String,
    pub depends_on: Vec<String>,
    pub waits_for: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub key: String,
    pub question: String,
    pub options: Vec<String>,
    pub outcome: String,
    pub why: String,
    pub status: String,
    pub decided_by: String,
}

/// What a change released, so the caller can tell the agent that was waiting.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Released {
    pub cards: Vec<String>,
}

pub(super) fn fail(error: rusqlite::Error) -> String {
    format!("Error de la base de Colmena: {error}")
}

pub(super) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// A unique id that never collides with one made on another PC (ADR 0019).
pub(super) fn new_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{:x}-{count:x}", std::process::id())
}

fn next_key(db: &Connection, table: &str, prefix: &str) -> Result<String, String> {
    let sql = format!("SELECT key FROM {table}");
    let mut statement = db.prepare(&sql).map_err(fail)?;
    let highest = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(fail)?
        .filter_map(Result::ok)
        .filter_map(|key| key.strip_prefix(prefix)?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    Ok(format!("{prefix}{:03}", highest + 1))
}

pub(super) fn record(
    db: &Connection,
    actor: &str,
    kind: &str,
    subject: &str,
) -> Result<(), String> {
    db.execute(
        "INSERT INTO event (id, actor, kind, subject, at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new_id(), actor, kind, subject, now()],
    )
    .map(drop)
    .map_err(fail)
}

fn card_id(db: &Connection, key: &str) -> Result<String, String> {
    db.query_row("SELECT id FROM card WHERE key = ?1", [key], |r| r.get(0))
        .optional()
        .map_err(fail)?
        .ok_or_else(|| format!("No existe la tarjeta {key}."))
}

fn decision_id(db: &Connection, key: &str) -> Result<String, String> {
    db.query_row("SELECT id FROM decision WHERE key = ?1", [key], |r| {
        r.get(0)
    })
    .optional()
    .map_err(fail)?
    .ok_or_else(|| format!("No existe la decisión {key}."))
}

fn keys(db: &Connection, sql: &str, id: &str) -> Result<Vec<String>, String> {
    let mut statement = db.prepare(sql).map_err(fail)?;
    let rows = statement
        .query_map([id], |row| row.get::<_, String>(0))
        .map_err(fail)?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn add_card(db: &Connection, title: &str, role: &str, actor: &str) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("La tarjeta necesita un título.".into());
    }
    let tx = db.unchecked_transaction().map_err(fail)?;
    let key = next_key(&tx, "card", "C-")?;
    let at = now();
    tx.execute(
        "INSERT INTO card (id, key, title, role, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![new_id(), key, title, role, at],
    )
    .map_err(fail)?;
    record(&tx, actor, "card.added", &key)?;
    tx.commit().map_err(fail)?;
    Ok(key)
}

pub fn card(db: &Connection, key: &str) -> Result<Card, String> {
    let id = card_id(db, key)?;
    let mut card = db
        .query_row(
            "SELECT key, title, status, priority, role, agent, blocker FROM card WHERE id = ?1",
            [&id],
            |r| {
                Ok(Card {
                    key: r.get(0)?,
                    title: r.get(1)?,
                    status: r.get(2)?,
                    priority: r.get(3)?,
                    role: r.get(4)?,
                    agent: r.get(5)?,
                    blocker: r.get(6)?,
                    depends_on: vec![],
                    waits_for: vec![],
                })
            },
        )
        .map_err(fail)?;
    card.depends_on = keys(
        db,
        "SELECT c.key FROM card_dep d JOIN card c ON c.id = d.depends_on WHERE d.card_id = ?1 ORDER BY c.key",
        &id,
    )?;
    card.waits_for = keys(
        db,
        "SELECT x.key FROM card_decision d JOIN decision x ON x.id = d.decision_id WHERE d.card_id = ?1 ORDER BY x.key",
        &id,
    )?;
    Ok(card)
}

pub fn cards(db: &Connection) -> Result<Vec<Card>, String> {
    let mut statement = db
        .prepare("SELECT key FROM card ORDER BY priority DESC, key")
        .map_err(fail)?;
    let all: Vec<String> = statement
        .query_map([], |r| r.get(0))
        .map_err(fail)?
        .filter_map(Result::ok)
        .collect();
    all.iter().map(|key| card(db, key)).collect()
}

/// Whether `from` can already reach `target` following "depends on" edges.
fn reaches(db: &Connection, from: &str, target: &str) -> Result<bool, String> {
    let mut seen = vec![from.to_owned()];
    let mut next = vec![from.to_owned()];
    while let Some(current) = next.pop() {
        if current == target {
            return Ok(true);
        }
        for dep in keys(
            db,
            "SELECT depends_on FROM card_dep WHERE card_id = ?1",
            &current,
        )? {
            if !seen.contains(&dep) {
                seen.push(dep.clone());
                next.push(dep);
            }
        }
    }
    Ok(false)
}

pub fn add_dependency(db: &Connection, key: &str, depends_on: &str) -> Result<(), String> {
    let (id, dep) = (card_id(db, key)?, card_id(db, depends_on)?);
    if id == dep {
        return Err("Una tarjeta no puede depender de sí misma.".into());
    }
    if reaches(db, &dep, &id)? {
        return Err(format!(
            "{key} → {depends_on} cerraría un ciclo de dependencias."
        ));
    }
    db.execute(
        "INSERT OR IGNORE INTO card_dep (card_id, depends_on) VALUES (?1, ?2)",
        [&id, &dep],
    )
    .map_err(fail)?;
    Ok(())
}

fn open_decisions(db: &Connection, id: &str) -> Result<Vec<String>, String> {
    keys(
        db,
        "SELECT x.key FROM card_decision d JOIN decision x ON x.id = d.decision_id \
         WHERE d.card_id = ?1 AND x.status = 'proposed' ORDER BY x.key",
        id,
    )
}

fn pending_deps(db: &Connection, id: &str) -> Result<Vec<String>, String> {
    keys(
        db,
        "SELECT c.key FROM card_dep d JOIN card c ON c.id = d.depends_on \
         WHERE d.card_id = ?1 AND c.status <> 'done' ORDER BY c.key",
        id,
    )
}

fn write_status(db: &Connection, id: &str, status: &str, blocker: &str) -> Result<(), String> {
    db.execute(
        "UPDATE card SET status = ?2, blocker = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, status, blocker, now()],
    )
    .map(drop)
    .map_err(fail)
}

/// Moves a card. Returns the cards this released (only when the card became `done`).
pub fn set_status(
    db: &Connection,
    key: &str,
    status: &str,
    actor: &str,
) -> Result<Released, String> {
    if !STATUSES.contains(&status) {
        return Err(format!(
            "Estado desconocido «{status}». Usa: {}.",
            STATUSES.join(", ")
        ));
    }
    let id = card_id(db, key)?;
    let open = open_decisions(db, &id)?;
    if let (true, Some(first)) = (
        matches!(status, "ready" | "doing" | "review" | "done"),
        open.first(),
    ) {
        return Err(format!("{key} {} {first}.", WAITING.to_lowercase()));
    }
    if matches!(status, "doing" | "review" | "done") {
        let pending = pending_deps(db, &id)?;
        if !pending.is_empty() {
            return Err(format!("{key} depende de {}.", pending.join(", ")));
        }
    }
    let tx = db.unchecked_transaction().map_err(fail)?;
    write_status(&tx, &id, status, "")?;
    record(&tx, actor, &format!("card.{status}"), key)?;
    let released = if status == "done" {
        promote_dependents(&tx, &id, actor)?
    } else {
        Released::default()
    };
    tx.commit().map_err(fail)?;
    Ok(released)
}

/// Cards still in the backlog whose last obstacle was `id` become ready.
fn promote_dependents(db: &Connection, id: &str, actor: &str) -> Result<Released, String> {
    let dependents = keys(db, "SELECT card_id FROM card_dep WHERE depends_on = ?1", id)?;
    let mut released = Released::default();
    for dependent in dependents {
        let status: String = db
            .query_row("SELECT status FROM card WHERE id = ?1", [&dependent], |r| {
                r.get(0)
            })
            .map_err(fail)?;
        if status == "backlog"
            && pending_deps(db, &dependent)?.is_empty()
            && open_decisions(db, &dependent)?.is_empty()
        {
            write_status(db, &dependent, "ready", "")?;
            let key: String = db
                .query_row("SELECT key FROM card WHERE id = ?1", [&dependent], |r| {
                    r.get(0)
                })
                .map_err(fail)?;
            record(db, actor, "card.released", &key)?;
            released.cards.push(key);
        }
    }
    released.cards.sort();
    Ok(released)
}

pub fn add_decision(
    db: &Connection,
    question: &str,
    options: &[String],
    actor: &str,
) -> Result<String, String> {
    let question = question.trim();
    if question.is_empty() {
        return Err("La decisión necesita una pregunta.".into());
    }
    let tx = db.unchecked_transaction().map_err(fail)?;
    let key = next_key(&tx, "decision", "D-")?;
    let options = serde_json::to_string(options).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO decision (id, key, question, options, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new_id(), key, question, options, now()],
    )
    .map_err(fail)?;
    record(&tx, actor, "decision.proposed", &key)?;
    tx.commit().map_err(fail)?;
    Ok(key)
}

pub fn decision(db: &Connection, key: &str) -> Result<Decision, String> {
    db.query_row(
        "SELECT key, question, options, outcome, why, status, decided_by FROM decision WHERE key = ?1",
        [key],
        |r| {
            let options: String = r.get(2)?;
            Ok(Decision {
                key: r.get(0)?,
                question: r.get(1)?,
                options: serde_json::from_str(&options).unwrap_or_default(),
                outcome: r.get(3)?,
                why: r.get(4)?,
                status: r.get(5)?,
                decided_by: r.get(6)?,
            })
        },
    )
    .optional()
    .map_err(fail)?
    .ok_or_else(|| format!("No existe la decisión {key}."))
}

pub fn decisions(db: &Connection) -> Result<Vec<Decision>, String> {
    let mut statement = db
        .prepare("SELECT key FROM decision ORDER BY key")
        .map_err(fail)?;
    let all: Vec<String> = statement
        .query_map([], |r| r.get(0))
        .map_err(fail)?
        .filter_map(Result::ok)
        .collect();
    all.iter().map(|key| decision(db, key)).collect()
}

/// Makes a card wait for a decision. A card already in progress goes back to `blocked`.
pub fn wait_for(db: &Connection, key: &str, decision_key: &str, actor: &str) -> Result<(), String> {
    let (id, did) = (card_id(db, key)?, decision_id(db, decision_key)?);
    if decision(db, decision_key)?.status != "proposed" {
        return Err(format!("{decision_key} ya no está abierta."));
    }
    let tx = db.unchecked_transaction().map_err(fail)?;
    tx.execute(
        "INSERT OR IGNORE INTO card_decision (card_id, decision_id) VALUES (?1, ?2)",
        [&id, &did],
    )
    .map_err(fail)?;
    let status: String = tx
        .query_row("SELECT status FROM card WHERE id = ?1", [&id], |r| r.get(0))
        .map_err(fail)?;
    if status != "done" {
        write_status(&tx, &id, "blocked", &format!("{WAITING} {decision_key}"))?;
    }
    record(&tx, actor, "card.waiting", key)?;
    tx.commit().map_err(fail)?;
    Ok(())
}

/// Closes a decision with `outcome` (`None` discards it) and releases, in chain, what waited for it.
pub fn resolve(
    db: &Connection,
    key: &str,
    outcome: Option<(&str, &str)>,
    decided_by: &str,
) -> Result<Released, String> {
    let current = decision(db, key)?;
    if current.status != "proposed" {
        return Err(format!("{key} ya estaba cerrada ({}).", current.status));
    }
    if outcome.is_some_and(|(text, _)| text.trim().is_empty()) {
        return Err("Decidir necesita un resultado.".into());
    }
    let did = decision_id(db, key)?;
    let tx = db.unchecked_transaction().map_err(fail)?;
    let (status, text, why) = match outcome {
        Some((text, why)) => ("decided", text.trim(), why.trim()),
        None => ("discarded", "", ""),
    };
    tx.execute(
        "UPDATE decision SET status = ?2, outcome = ?3, why = ?4, decided_by = ?5, decided_at = ?6 WHERE id = ?1",
        params![did, status, text, why, decided_by, now()],
    )
    .map_err(fail)?;
    record(&tx, decided_by, &format!("decision.{status}"), key)?;
    let waiting = keys(
        &tx,
        "SELECT card_id FROM card_decision WHERE decision_id = ?1",
        &did,
    )?;
    let mut released = Released::default();
    for id in waiting {
        tx.execute(
            "DELETE FROM card_decision WHERE card_id = ?1 AND decision_id = ?2",
            [&id, &did],
        )
        .map_err(fail)?;
        if !open_decisions(&tx, &id)?.is_empty() {
            continue;
        }
        let (status, blocker): (String, String) = tx
            .query_row(
                "SELECT status, blocker FROM card WHERE id = ?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(fail)?;
        // Only what this mechanism blocked is released; a human-written blocker stays.
        let held_by_this = status == "blocked" && blocker.starts_with(WAITING);
        if held_by_this || status == "backlog" {
            let next = if pending_deps(&tx, &id)?.is_empty() {
                "ready"
            } else {
                "backlog"
            };
            if next == status {
                continue;
            }
            write_status(&tx, &id, next, "")?;
            let card_key: String = tx
                .query_row("SELECT key FROM card WHERE id = ?1", [&id], |r| r.get(0))
                .map_err(fail)?;
            record(&tx, decided_by, "card.released", &card_key)?;
            released.cards.push(card_key);
        }
    }
    released.cards.sort();
    tx.commit().map_err(fail)?;
    Ok(released)
}

#[cfg(test)]
#[path = "board_tests.rs"]
mod tests;
