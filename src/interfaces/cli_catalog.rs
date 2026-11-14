//! `skills`, `packs` and `mods` commands. Previews by default; `--apply` writes.
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

pub(super) fn skills(command: SkillsCommand) -> Result<(), String> {
    match command {
        SkillsCommand::List { path } => {
            let view = inspect_catalog(&path)?;
            print_header(&view);
            for skill in &view.skills {
                println!(
                    "{}{} {} — {}",
                    if skill.enabled { "✓" } else { " " },
                    if skill.recommended { "★" } else { " " },
                    skill.name,
                    skill.description
                );
            }
            Ok(())
        }
        SkillsCommand::Enable { names, path, apply } => {
            run_skills(&path, &SkillsChange::Enable(names), apply)
        }
        SkillsCommand::Disable { names, path, apply } => {
            run_skills(&path, &SkillsChange::Disable(names), apply)
        }
        SkillsCommand::Sync { path, apply } => run_skills(&path, &SkillsChange::Sync, apply),
    }
}

pub(super) fn packs(command: PacksCommand) -> Result<(), String> {
    match command {
        PacksCommand::List { path } => {
            let view = inspect_catalog(&path)?;
            print_header(&view);
            for pack in &view.packs {
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
            Ok(())
        }
        PacksCommand::Apply { name, path, apply } => {
            run_skills(&path, &SkillsChange::Pack(name), apply)
        }
    }
}

pub(super) fn mods(command: ModsCommand) -> Result<(), String> {
    match command {
        ModsCommand::List { path } => {
            let view = inspect_catalog(&path)?;
            print_header(&view);
            for module in &view.mods {
                println!(
                    "{}{}{} {} — {}",
                    if module.applied { "✓" } else { " " },
                    if module.recommended { "★" } else { " " },
                    if module.conflict { "!" } else { " " },
                    module.name,
                    module.description
                );
            }
            println!("\n✓ instalado · ★ recomendado · ! existe un archivo ajeno con ese nombre");
            Ok(())
        }
        ModsCommand::Apply { names, path, apply } => {
            let names = (!names.is_empty()).then_some(names);
            run_mods(&path, &ModsChange::Apply(names), apply)
        }
        ModsCommand::Remove { names, path, apply } => {
            run_mods(&path, &ModsChange::Remove(names), apply)
        }
    }
}

fn print_header(view: &CatalogView) {
    println!("Proyecto: {}", view.project_root);
    if let Some(error) = &view.selection_error {
        println!("Aviso: {error}");
    }
}

fn run_skills(path: &Path, change: &SkillsChange, apply: bool) -> Result<(), String> {
    let plan: SkillsPlan = plan_skills(path, change)?;
    println!("Vista previa para {}", plan.project_root);
    for action in &plan.actions {
        println!("• {action}");
    }
    if !apply || !plan.changes_state() {
        if plan.changes_state() {
            println!("Añade --apply para aplicar estos cambios.");
        }
        return Ok(());
    }
    let result = apply_skills(&plan)?;
    println!("Skills habilitadas: {}", result.enabled.join(", "));
    for backup in &result.backups {
        println!("Backup: {backup}");
    }
    Ok(())
}

fn run_mods(path: &Path, change: &ModsChange, apply: bool) -> Result<(), String> {
    let plan: ModsPlan = plan_mods(path, change)?;
    println!("Vista previa para {}", plan.project_root);
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("No se puede aplicar: {reason}"));
    }
    for action in &plan.actions {
        println!("• {action}");
    }
    if !apply || !plan.changes_state() {
        if plan.changes_state() {
            println!("Añade --apply para aplicar estos cambios.");
        }
        return Ok(());
    }
    let result = apply_mods_plan(&plan)?;
    println!("Completado: {}", result.completed.join(" · "));
    for backup in &result.backups {
        println!("Backup recuperable: {backup}");
    }
    println!("Los agentes reconocen los Mods al iniciar una sesión nueva.");
    Ok(())
}
