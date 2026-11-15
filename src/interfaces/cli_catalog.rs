//! `skills`, `packs` and `mods` commands. Previews by default; `--apply` writes.
use super::style::{Tone, Ui, pad};
use crate::features::catalog_view::{CatalogView, inspect_catalog};
use crate::features::project_mods::{ModsChange, ModsPlan, apply_mods_plan, plan_mods};
use crate::features::project_skills::{SkillsChange, SkillsPlan, apply_skills, plan_skills};
use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub(super) enum SkillsCommand {
    /// Lista las skills y marca las habilitadas y recomendadas.
    List {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Habilita skills y sincroniza `.ai/SKILLS.md`.
    Enable {
        #[arg(required = true)]
        names: Vec<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        apply: bool,
    },
    /// Deshabilita skills y sincroniza `.ai/SKILLS.md`.
    Disable {
        #[arg(required = true)]
        names: Vec<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        apply: bool,
    },
    /// Regenera el contexto de las skills habilitadas.
    Sync {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum PacksCommand {
    /// Lista los Tech Packs y marca los detectados para el proyecto.
    List {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Habilita todas las skills de un Tech Pack.
    Apply {
        name: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum ModsCommand {
    /// Lista los Mods locales y marca los recomendados.
    List {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Instala Mods como skills del proyecto; sin nombres usa los recomendados.
    Apply {
        names: Vec<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        apply: bool,
    },
    /// Retira Mods administrados por ChxChx guardando antes un backup.
    Remove {
        #[arg(required = true)]
        names: Vec<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        apply: bool,
    },
}

/// `✓`/`★`/`!` markers of a row, colored; two spaces apart as in the plain listing.
fn marks(ui: &Ui, on: bool, recommended: bool, conflict: Option<bool>) -> String {
    let mut out = format!(
        "{}{}",
        if on {
            ui.strong(Tone::Green, "✓")
        } else {
            " ".to_owned()
        },
        if recommended {
            ui.strong(Tone::Yellow, "★")
        } else {
            " ".to_owned()
        }
    );
    if let Some(conflict) = conflict {
        out.push_str(&if conflict {
            ui.strong(Tone::Red, "!")
        } else {
            " ".to_owned()
        });
    }
    out
}

pub(super) fn skills(ui: &Ui, command: SkillsCommand) -> Result<(), String> {
    match command {
        SkillsCommand::List { path } => {
            let view = inspect_catalog(&path)?;
            print_header(ui, "Skills", &view);
            let width = view
                .skills
                .iter()
                .map(|s| s.name.chars().count())
                .max()
                .unwrap_or(0);
            for skill in &view.skills {
                if ui.is_rich() {
                    println!(
                        "  {} {}  {}",
                        marks(ui, skill.enabled, skill.recommended, None),
                        ui.bold(&pad(&skill.name, width)),
                        ui.dim(&skill.description)
                    );
                } else {
                    println!(
                        "{}{} {} — {}",
                        if skill.enabled { "✓" } else { " " },
                        if skill.recommended { "★" } else { " " },
                        skill.name,
                        skill.description
                    );
                }
            }
            legend(ui, false);
            Ok(())
        }
        SkillsCommand::Enable { names, path, apply } => {
            run_skills(ui, &path, &SkillsChange::Enable(names), apply)
        }
        SkillsCommand::Disable { names, path, apply } => {
            run_skills(ui, &path, &SkillsChange::Disable(names), apply)
        }
        SkillsCommand::Sync { path, apply } => run_skills(ui, &path, &SkillsChange::Sync, apply),
    }
}

pub(super) fn packs(ui: &Ui, command: PacksCommand) -> Result<(), String> {
    match command {
        PacksCommand::List { path } => {
            let view = inspect_catalog(&path)?;
            print_header(ui, "Tech Packs", &view);
            let width = view
                .packs
                .iter()
                .map(|p| p.name.chars().count())
                .max()
                .unwrap_or(0);
            for pack in &view.packs {
                if ui.is_rich() {
                    let progress = format!("{}/{}", pack.enabled_skills, pack.skills.len());
                    let tone =
                        if pack.enabled_skills == pack.skills.len() && !pack.skills.is_empty() {
                            Tone::Green
                        } else {
                            Tone::Muted
                        };
                    println!(
                        "  {} {}  {}  {}",
                        if pack.recommended {
                            ui.strong(Tone::Yellow, "★")
                        } else {
                            " ".to_owned()
                        },
                        ui.bold(&pad(&pack.name, width)),
                        ui.paint(tone, &pad(&progress, 5)),
                        ui.dim(&pack.description)
                    );
                    if pack.recommended {
                        println!("      {}", ui.paint(Tone::Yellow, &pack.reasons.join("; ")));
                    }
                } else {
                    println!(
                        "{} {} ({}/{} skills) — {}",
                        if pack.recommended { "★" } else { " " },
                        pack.name,
                        pack.enabled_skills,
                        pack.skills.len(),
                        pack.description
                    );
                    if pack.recommended {
                        println!("    {}", pack.reasons.join("; "));
                    }
                }
            }
            legend(ui, false);
            Ok(())
        }
        PacksCommand::Apply { name, path, apply } => {
            run_skills(ui, &path, &SkillsChange::Pack(name), apply)
        }
    }
}

pub(super) fn mods(ui: &Ui, command: ModsCommand) -> Result<(), String> {
    match command {
        ModsCommand::List { path } => {
            let view = inspect_catalog(&path)?;
            print_header(ui, "Mods", &view);
            let width = view
                .mods
                .iter()
                .map(|m| m.name.chars().count())
                .max()
                .unwrap_or(0);
            for module in &view.mods {
                if ui.is_rich() {
                    println!(
                        "  {} {}  {}",
                        marks(
                            ui,
                            module.applied,
                            module.recommended,
                            Some(module.conflict)
                        ),
                        ui.bold(&pad(&module.name, width)),
                        ui.dim(&module.description)
                    );
                } else {
                    println!(
                        "{}{}{} {} — {}",
                        if module.applied { "✓" } else { " " },
                        if module.recommended { "★" } else { " " },
                        if module.conflict { "!" } else { " " },
                        module.name,
                        module.description
                    );
                }
            }
            if ui.is_rich() {
                legend(ui, true);
            } else {
                println!(
                    "\n✓ instalado · ★ recomendado · ! existe un archivo ajeno con ese nombre"
                );
            }
            Ok(())
        }
        ModsCommand::Apply { names, path, apply } => {
            let names = (!names.is_empty()).then_some(names);
            run_mods(ui, &path, &ModsChange::Apply(names), apply)
        }
        ModsCommand::Remove { names, path, apply } => {
            run_mods(ui, &path, &ModsChange::Remove(names), apply)
        }
    }
}

/// What the markers mean, only when the listing is drawn with them in color.
fn legend(ui: &Ui, conflicts: bool) {
    if !ui.is_rich() {
        return;
    }
    let mut text = format!(
        "{} habilitado   {} recomendado",
        ui.strong(Tone::Green, "✓"),
        ui.strong(Tone::Yellow, "★")
    );
    if conflicts {
        text.push_str(&format!(
            "   {} existe un archivo ajeno con ese nombre",
            ui.strong(Tone::Red, "!")
        ));
    }
    println!("\n  {}", text);
}

fn print_header(ui: &Ui, title: &str, view: &CatalogView) {
    if ui.is_rich() {
        println!("{}", ui.heading("", title, &view.project_root));
    } else {
        println!("Proyecto: {}", view.project_root);
    }
    if let Some(error) = &view.selection_error {
        println!("{}", ui.warning(error));
    }
}

fn run_skills(ui: &Ui, path: &Path, change: &SkillsChange, apply: bool) -> Result<(), String> {
    let plan: SkillsPlan = plan_skills(path, change)?;
    println!(
        "{}",
        ui.heading(
            &format!("Vista previa para {}", plan.project_root),
            "Vista previa",
            &plan.project_root
        )
    );
    for action in &plan.actions {
        println!("{}", ui.bullet(action));
    }
    if !apply || !plan.changes_state() {
        if plan.changes_state() {
            println!(
                "{}",
                ui.hint("Añade --apply para aplicar estos cambios.")
                    .trim_start_matches('\n')
            );
        }
        return Ok(());
    }
    let result = apply_skills(&plan)?;
    println!(
        "{}",
        ui.done(&format!(
            "Skills habilitadas: {}",
            result.enabled.join(", ")
        ))
    );
    for backup in &result.backups {
        println!("{}", ui.field("Backup", backup));
    }
    Ok(())
}

fn run_mods(ui: &Ui, path: &Path, change: &ModsChange, apply: bool) -> Result<(), String> {
    let plan: ModsPlan = plan_mods(path, change)?;
    println!(
        "{}",
        ui.heading(
            &format!("Vista previa para {}", plan.project_root),
            "Vista previa",
            &plan.project_root
        )
    );
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("No se puede aplicar: {reason}"));
    }
    for action in &plan.actions {
        println!("{}", ui.bullet(action));
    }
    if !apply || !plan.changes_state() {
        if plan.changes_state() {
            println!(
                "{}",
                ui.hint("Añade --apply para aplicar estos cambios.")
                    .trim_start_matches('\n')
            );
        }
        return Ok(());
    }
    let result = apply_mods_plan(&plan)?;
    println!(
        "{}",
        ui.done(&format!("Completado: {}", result.completed.join(" · ")))
    );
    for backup in &result.backups {
        println!("{}", ui.field("Backup recuperable", backup));
    }
    println!(
        "{}",
        ui.dim("Los agentes reconocen los Mods al iniciar una sesión nueva.")
    );
    Ok(())
}
