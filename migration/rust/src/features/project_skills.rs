//! Per-project skill selection (`.ai/chxchx-skills.toml`) and its synced context.
use crate::features::catalog;
use crate::features::project_changes::{apply_changes, missing_directories, resolve_root};
use crate::features::project_setup::{ProjectFileChange, plan_file_change, plan_managed_file};
use std::fs;
use std::path::{Path, PathBuf};

pub const SELECTION_FILE: &str = ".ai/chxchx-skills.toml";
pub const CONTEXT_FILE: &str = ".ai/SKILLS.md";

#[derive(Clone, Debug)]
pub enum SkillsChange {
    Enable(Vec<String>),
    Disable(Vec<String>),
    /// Enables every skill of a Tech Pack.
    Pack(String),
    /// Rewrites the generated context from the current selection.
    Sync,
}

#[derive(Clone, Debug)]
pub struct SkillsPlan {
    pub project_root: String,
    pub enabled_before: Vec<String>,
    pub enabled_after: Vec<String>,
    pub actions: Vec<String>,
    pub blocked_reason: Option<String>,
    root: PathBuf,
    directories: Vec<PathBuf>,
    files: Vec<ProjectFileChange>,
}

impl SkillsPlan {
    pub fn changes_state(&self) -> bool {
        !self.files.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct SkillsApplication {
    pub enabled: Vec<String>,
    pub backups: Vec<String>,
}

/// Enabled skills; a missing file means none, an invalid one is an error.
pub fn enabled_skills(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join(SELECTION_FILE);
    let text = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "{SELECTION_FILE} es un enlace simbólico; no se lee"
            ));
        }
        Ok(_) => fs::read_to_string(&path)
            .map_err(|error| format!("No se pudo leer {SELECTION_FILE}: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("No se pudo inspeccionar {SELECTION_FILE}: {error}")),
    };
    let table: toml::Table = toml::from_str(&text)
        .map_err(|error| format!("No se pudo leer {SELECTION_FILE}: {error}"))?;
    if table.keys().any(|key| key != "version" && key != "enabled")
        || table.get("version").and_then(toml::Value::as_integer) != Some(1)
    {
        return Err(format!(
            "Configuración de skills no compatible: {SELECTION_FILE}"
        ));
    }
    let mut names = match table.get("enabled") {
        None => Vec::new(),
        Some(toml::Value::Array(items)) => items
            .iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                format!("El campo enabled debe ser una lista de nombres en {SELECTION_FILE}")
            })?,
        Some(_) => {
            return Err(format!(
                "El campo enabled debe ser una lista de nombres en {SELECTION_FILE}"
            ));
        }
    };
    names.sort();
    names.dedup();
    Ok(names)
}

pub fn plan_skills(path: &Path, change: &SkillsChange) -> Result<SkillsPlan, String> {
    let root = resolve_root(path)?;
    let before = enabled_skills(&root)?;
    let mut after = before.clone();
    let mut actions = Vec::new();
    match change {
        SkillsChange::Enable(names) => {
            for name in names {
                let skill =
                    catalog::skill(name).ok_or_else(|| format!("Skill desconocida: {name}"))?;
                if !after.contains(&skill.name) {
                    after.push(skill.name.clone());
                    actions.push(format!("habilitar la skill `{}`", skill.name));
                }
            }
        }
        SkillsChange::Disable(names) => {
            for name in names {
                let skill =
                    catalog::skill(name).ok_or_else(|| format!("Skill desconocida: {name}"))?;
                if let Some(index) = after.iter().position(|item| *item == skill.name) {
                    after.remove(index);
                    actions.push(format!("deshabilitar la skill `{}`", skill.name));
                }
            }
        }
        SkillsChange::Pack(name) => {
            let pack =
                catalog::pack(name).ok_or_else(|| format!("Tech Pack desconocido: {name}"))?;
            for skill in &pack.skills {
                if !after.contains(skill) {
                    after.push(skill.clone());
                    actions.push(format!("habilitar `{skill}` del pack `{}`", pack.name));
                }
            }
        }
        SkillsChange::Sync => {}
    }
    after.sort();
    after.dedup();

    let mut files = Vec::new();
    // An absent selection with nothing enabled stays absent instead of creating an empty file.
    if !after.is_empty() || root.join(SELECTION_FILE).exists() {
        let selection = render_selection(&after);
        if let Some(file) = plan_file_change(&root, SELECTION_FILE, selection.into_bytes())? {
            files.push(file);
        }
    }
    let body = context_body(&after)?;
    if let Some(file) = plan_managed_file(&root, CONTEXT_FILE, "active-skills", &body)? {
        actions.push(format!("actualizar {CONTEXT_FILE}"));
        files.push(file);
    }
    let guide = format!(
        "Consulta `{CONTEXT_FILE}` al trabajar en tareas cubiertas por esas skills. Skills habilitadas: {}.",
        if after.is_empty() {
            "ninguna".to_owned()
        } else {
            after.join(", ")
        }
    );
    if let Some(file) = plan_managed_file(&root, "AGENTS.md", "project-skills", &guide)? {
        actions.push("actualizar el bloque administrado de AGENTS.md".to_owned());
        files.push(file);
    }
    if files.is_empty() {
        actions = vec!["todo está actualizado".to_owned()];
    }
    let directories = missing_directories(&root, &files)?;
    Ok(SkillsPlan {
        project_root: root.display().to_string(),
        enabled_before: before,
        enabled_after: after,
        actions,
        blocked_reason: None,
        root,
        directories,
        files,
    })
}

pub fn apply_skills(plan: &SkillsPlan) -> Result<SkillsApplication, String> {
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("No se aplicó: {reason}"));
    }
    let backups = if plan.files.is_empty() {
        Vec::new()
    } else {
        apply_changes(&plan.root, &plan.directories, &plan.files, true)?
    };
    Ok(SkillsApplication {
        enabled: plan.enabled_after.clone(),
        backups,
    })
}

fn render_selection(names: &[String]) -> String {
    let items: String = names
        .iter()
        .map(|name| format!("  \"{name}\",\n"))
        .collect();
    format!("version = 1\nenabled = [\n{items}]\n")
}

fn context_body(names: &[String]) -> Result<String, String> {
    if names.is_empty() {
        return Ok("No hay skills habilitadas para este proyecto.".to_owned());
    }
    let mut sections = Vec::new();
    for name in names {
        let skill = catalog::skill(name)
            .ok_or_else(|| format!("Skill habilitada que ya no existe en el catálogo: {name}"))?;
        sections.push(format!("## {}\n\n{}", skill.name, skill.instructions));
    }
    Ok(sections.join("\n\n"))
}
