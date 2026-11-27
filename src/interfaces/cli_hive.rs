//! `chxchx-tech hive`: the board and decisions of Colmena from the terminal. Only calls the
//! shared board rules; it holds no logic of its own.
use crate::features::extensions;
use crate::features::hive::{board, export, import_vyro, rag, store};
use crate::features::studio_preferences;
use clap::Subcommand;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug, clap::Args)]
pub(super) struct HiveCommand {
    /// Proyecto cuyo tablero se usa (por defecto, la carpeta actual).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, Subcommand)]
pub(super) enum Action {
    /// Tarjetas del tablero.
    Card {
        #[command(subcommand)]
        action: CardAction,
    },
    /// Lee el tablero de un proyecto Vyro (solo lectura; repetirlo lo actualiza).
    ImportVyro {
        /// Carpeta del proyecto Vyro (por defecto, la de `--root`).
        #[arg(long)]
        from: Option<PathBuf>,
    },
    /// Escribe `.ai/board.json` con el tablero actual.
    Export,
    /// Actualiza el índice de búsqueda con los documentos del proyecto (solo lo que cambió).
    Index,
    /// Busca en los documentos indexados (BM25); indexa primero lo que haya cambiado.
    Search {
        query: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Decisiones que esperan a la persona.
    Decision {
        #[command(subcommand)]
        action: DecisionAction,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum CardAction {
    /// Crea una tarjeta y muestra su clave (C-001, C-002…).
    Add {
        title: String,
        #[arg(long, default_value = "")]
        role: String,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    /// Cambia el estado: backlog, ready, doing, review, done o blocked.
    Move { key: String, status: String },
    /// `key` pasa a depender de `depends_on`.
    Dep { key: String, depends_on: String },
    /// Bloquea la tarjeta con un motivo escrito por una persona.
    Block { key: String, reason: String },
    /// `key` espera a que se decida `decision`.
    Wait { key: String, decision: String },
}

#[derive(Debug, Subcommand)]
pub(super) enum DecisionAction {
    /// Propone una decisión; repite `--option` para las alternativas.
    Add {
        question: String,
        #[arg(long = "option")]
        options: Vec<String>,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    /// Decide y libera lo que esperaba. Decidir no concede permisos.
    Decide {
        key: String,
        outcome: String,
        #[arg(long, default_value = "")]
        why: String,
    },
    /// Descarta la decisión y libera lo que esperaba.
    Discard { key: String },
    /// Espera a que la persona cierre la decisión y muestra el resultado (para agentes).
    Await {
        key: String,
        /// Segundos máximos de espera; 0 solo consulta.
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
}

/// Polls the board until `key` is closed. The result is information for the agent; it grants
/// no permission.
fn await_decision(
    db: &Connection,
    key: &str,
    timeout: std::time::Duration,
    poll: std::time::Duration,
) -> Result<String, String> {
    let start = std::time::Instant::now();
    loop {
        let d = board::decision(db, key)?;
        match d.status.as_str() {
            "proposed" => {}
            "decided" => {
                let why = if d.why.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", d.why)
                };
                return Ok(format!("{key} decidida: {}{why}\n", d.outcome));
            }
            other => return Ok(format!("{key} {other}: sin resultado.\n")),
        }
        if start.elapsed() >= timeout {
            return Err(format!("{key} sigue abierta tras la espera."));
        }
        std::thread::sleep(poll);
    }
}

pub(super) fn run(command: HiveCommand) -> Result<(), String> {
    if !studio_preferences::load()
        .preferences
        .extension_enabled(extensions::HIVE)
    {
        return Err("Colmena está desactivada. Actívala en Studio → Ajustes → Extensiones.".into());
    }
    let root = std::fs::canonicalize(&command.root)
        .map_err(|error| format!("No se pudo abrir {}: {error}", command.root.display()))?;
    let (db, _) = store::open(&root)?;
    print!("{}", execute(&db, &root, command.action)?);
    Ok(())
}

fn json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|text| text + "\n")
        .map_err(|error| error.to_string())
}

fn released(list: &board::Released) -> String {
    if list.cards.is_empty() {
        String::new()
    } else {
        format!("Liberó: {}\n", list.cards.join(", "))
    }
}

pub(super) fn execute(db: &Connection, root: &Path, action: Action) -> Result<String, String> {
    const WHO: &str = "cli";
    match action {
        Action::ImportVyro { from } => {
            let source = from.unwrap_or_else(|| root.to_path_buf());
            let s = import_vyro::import(db, &source)?;
            Ok(format!(
                "Importado: {} tarjetas, {} dependencias nuevas, {} decisiones ({} reteniendo tarjetas).\n",
                s.cards, s.dependencies, s.decisions, s.waiting
            ))
        }
        Action::Index => {
            let s = rag::index(db, root)?;
            Ok(format!(
                "Indexado: {} archivos ({} fragmentos), {} sin cambios, {} retirados.\n",
                s.files, s.chunks, s.unchanged, s.removed
            ))
        }
        Action::Search {
            query,
            limit,
            json: as_json,
        } => {
            rag::index(db, root)?;
            let hits = rag::search(db, &query, limit)?;
            if as_json {
                return json(&hits);
            }
            if hits.is_empty() {
                return Ok("Sin resultados.\n".into());
            }
            Ok(hits
                .iter()
                .map(|h| format!("{} · {}\n  {}\n", h.path, h.heading, h.snippet))
                .collect())
        }
        Action::Export => Ok(format!("{}\n", export::write(db, root)?.display())),
        Action::Card { action } => match action {
            CardAction::Block { key, reason } => {
                board::block(db, &key, &reason, WHO)?;
                Ok(format!("{key} bloqueada: {reason}\n"))
            }
            CardAction::Add { title, role } => {
                Ok(format!("{}\n", board::add_card(db, &title, &role, WHO)?))
            }
            CardAction::List { json: true } => json(&board::cards(db)?),
            CardAction::List { json: false } => Ok(board::cards(db)?
                .iter()
                .map(|card| {
                    let note = match (card.waits_for.is_empty(), card.depends_on.is_empty()) {
                        (true, true) => String::new(),
                        _ => format!(
                            "  [depende: {} · espera: {}]",
                            card.depends_on.join(","),
                            card.waits_for.join(",")
                        ),
                    };
                    format!("{}  {:<8} {}{note}\n", card.key, card.status, card.title)
                })
                .collect()),
            CardAction::Move { key, status } => {
                let list = board::set_status(db, &key, &status, WHO)?;
                Ok(format!("{key} → {status}\n{}", released(&list)))
            }
            CardAction::Dep { key, depends_on } => {
                board::add_dependency(db, &key, &depends_on)?;
                Ok(format!("{key} depende de {depends_on}\n"))
            }
            CardAction::Wait { key, decision } => {
                board::wait_for(db, &key, &decision, WHO)?;
                Ok(format!("{key} espera {decision}\n"))
            }
        },
        Action::Decision { action } => match action {
            DecisionAction::Await { key, timeout } => await_decision(
                db,
                &key,
                std::time::Duration::from_secs(timeout),
                std::time::Duration::from_millis(500),
            ),
            DecisionAction::Add { question, options } => Ok(format!(
                "{}\n",
                board::add_decision(db, &question, &options, WHO)?
            )),
            DecisionAction::List { json: true } => json(&board::decisions(db)?),
            DecisionAction::List { json: false } => Ok(board::decisions(db)?
                .iter()
                .map(|d| format!("{}  {:<9} {}\n", d.key, d.status, d.question))
                .collect()),
            DecisionAction::Decide { key, outcome, why } => {
                let list = board::resolve(db, &key, Some((&outcome, &why)), WHO)?;
                Ok(format!("{key} decidida: {outcome}\n{}", released(&list)))
            }
            DecisionAction::Discard { key } => {
                let list = board::resolve(db, &key, None, WHO)?;
                Ok(format!("{key} descartada\n{}", released(&list)))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn await_returns_the_outcome_once_the_person_decides() {
        let home = std::env::temp_dir().join(format!("chxchx-cli-await-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let path = store::path_in(&home, Path::new("/work/w"));
        let (db, _) = store::open_at(&path).unwrap();
        board::add_decision(&db, "¿Motor?", &["sqlite".into()], "t").unwrap();
        let none = std::time::Duration::ZERO;
        assert!(await_decision(&db, "D-001", none, none).is_err());
        let other = path.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let (db, _) = store::open_at(&other).unwrap();
            board::resolve(&db, "D-001", Some(("sqlite", "es liviano")), "persona").unwrap();
        });
        let ms = std::time::Duration::from_millis(20);
        let out = await_decision(&db, "D-001", std::time::Duration::from_secs(5), ms).unwrap();
        t.join().unwrap();
        assert_eq!(out, "D-001 decidida: sqlite (es liviano)\n");
        let _ = std::fs::remove_dir_all(&home);
    }

    static ROOT_PATH: &str = "/work/a";

    #[test]
    fn the_terminal_drives_the_same_rules() {
        let home = std::env::temp_dir().join(format!("chxchx-cli-hive-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let (db, _) = store::open_at(&store::path_in(&home, Path::new("/work/a"))).unwrap();
        let card = |action| Action::Card { action };
        let decision = |action| Action::Decision { action };
        let add = |title: &str| CardAction::Add {
            title: title.into(),
            role: String::new(),
        };

        assert_eq!(
            execute(&db, Path::new(ROOT_PATH), card(add("A"))).unwrap(),
            "C-001\n"
        );
        execute(&db, Path::new(ROOT_PATH), card(add("B"))).unwrap();
        execute(
            &db,
            Path::new(ROOT_PATH),
            decision(DecisionAction::Add {
                question: "¿Motor?".into(),
                options: vec!["sqlite".into()],
            }),
        )
        .unwrap();
        execute(
            &db,
            Path::new(ROOT_PATH),
            card(CardAction::Wait {
                key: "C-002".into(),
                decision: "D-001".into(),
            }),
        )
        .unwrap();
        let out = execute(
            &db,
            Path::new(ROOT_PATH),
            decision(DecisionAction::Decide {
                key: "D-001".into(),
                outcome: "sqlite".into(),
                why: String::new(),
            }),
        )
        .unwrap();
        assert!(out.contains("Liberó: C-002"), "{out}");
        let listed = execute(
            &db,
            Path::new(ROOT_PATH),
            card(CardAction::List { json: false }),
        )
        .unwrap();
        assert!(listed.contains("C-002  ready"), "{listed}");
        let as_json = execute(
            &db,
            Path::new(ROOT_PATH),
            card(CardAction::List { json: true }),
        )
        .unwrap();
        assert!(as_json.contains("\"key\": \"C-001\""));
        assert!(
            execute(
                &db,
                Path::new(ROOT_PATH),
                card(CardAction::Move {
                    key: "C-009".into(),
                    status: "done".into()
                })
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(home);
    }
}
