//! `chxchx-tech hive`: the board and decisions of Colmena from the terminal. Only calls the
//! shared board rules; it holds no logic of its own.
use crate::features::extensions;
use crate::features::hive::{board, import_vyro, store};
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
        Action::Card { action } => match action {
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
