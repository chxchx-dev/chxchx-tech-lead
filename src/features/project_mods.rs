//! Mods: first-party workflow instructions installed as agent skills inside a project.
//! Claude Code and OpenCode read `.claude/skills`; Codex reads `.agents/skills`.
use crate::features::catalog::{self, Mod};
use crate::features::project_changes::{apply_changes, missing_directories, resolve_root};
use crate::features::project_detection::{ProjectDetection, detect_project};
use crate::features::project_mod_agents::{ALL_ROOTS, install_roots};
use crate::features::project_setup::ProjectFileChange;
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct ModStatus {
    pub name: String,
    pub description: String,
    pub recommended: bool,
    pub reasons: Vec<String>,
    pub applied: bool,
    pub conflict: bool,
}

#[derive(Clone, Debug)]
pub enum ModsChange {
    /// Installs the named Mods, or the recommended ones when `None`.
    Apply(Option<Vec<String>>),
    Remove(Vec<String>),
}

#[derive(Clone, Debug)]
struct Removal {
    relative: String,
    label: &'static str,
    name: String,
    before: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ModsPlan {
    pub project_root: String,
    pub actions: Vec<String>,
    pub blocked_reason: Option<String>,
    root: PathBuf,
    directories: Vec<PathBuf>,
    files: Vec<ProjectFileChange>,
    removals: Vec<Removal>,
    replaced: Vec<(String, &'static str, String, Vec<u8>)>,
}

impl ModsPlan {
    pub fn changes_state(&self) -> bool {
        !self.files.is_empty() || !self.removals.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct ModsApplication {
    pub completed: Vec<String>,
    pub backups: Vec<String>,
}

fn marker(name: &str) -> String {
    format!("<!-- chxchx-mod:{name} -->")
}

fn render(module: &Mod) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n\n{}\n",
        module.name,
        module.description,
        marker(&module.name),
        module.instructions
    )
}

fn read_target(path: &Path, relative: &str) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("{relative} es un enlace simbólico; no se modifica"))
        }
        Ok(metadata) if metadata.is_file() => fs::read(path)
            .map(Some)
            .map_err(|error| format!("No se pudo leer {relative}: {error}")),
        Ok(_) => Err(format!("Se esperaba un archivo en {relative}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("No se pudo inspeccionar {relative}: {error}")),
    }
}

pub fn mod_status(root: &Path, detection: &ProjectDetection) -> Vec<ModStatus> {
    catalog::mods()
        .iter()
        .map(|module| {
            let target = root
                .join(".claude/skills")
                .join(&module.name)
                .join("SKILL.md");
            let content = fs::read_to_string(&target).unwrap_or_default();
            let applied = content.contains(&marker(&module.name));
            let reasons = catalog::mod_reasons(detection, module);
            ModStatus {
                name: module.name.clone(),
                description: module.description.clone(),
                recommended: !reasons.is_empty(),
                reasons,
                applied,
                conflict: target.exists() && !applied,
            }
        })
        .collect()
}

pub fn plan_mods(path: &Path, change: &ModsChange) -> Result<ModsPlan, String> {
    let root = resolve_root(path)?;
    let mut plan = ModsPlan {
        project_root: root.display().to_string(),
        actions: Vec::new(),
        blocked_reason: None,
        root: root.clone(),
        directories: Vec::new(),
        files: Vec::new(),
        removals: Vec::new(),
        replaced: Vec::new(),
    };
    match change {
        ModsChange::Apply(names) => plan_apply(&mut plan, names.as_deref())?,
        ModsChange::Remove(names) => plan_remove(&mut plan, names)?,
    }
    if plan.actions.is_empty() && plan.blocked_reason.is_none() {
        plan.actions.push("todo está actualizado".to_owned());
    }
    plan.directories = missing_directories(&root, &plan.files)?;
    Ok(plan)
}

fn plan_apply(plan: &mut ModsPlan, names: Option<&[String]>) -> Result<(), String> {
    let selected: Vec<&'static Mod> = match names {
        Some(names) => {
            let mut selected: Vec<&'static Mod> = Vec::new();
            for name in names {
                let module =
                    catalog::module(name).ok_or_else(|| format!("Mod desconocido: {name}"))?;
                if !selected.iter().any(|item| item.name == module.name) {
                    selected.push(module);
                }
            }
            selected
        }
        None => {
            let detection = detect_project(&plan.root);
            catalog::mods()
                .iter()
                .filter(|module| !catalog::mod_reasons(&detection, module).is_empty())
                .collect()
        }
    };
    let mut conflicts = Vec::new();
    for module in selected {
        let content = render(module).into_bytes();
        for (label, directory) in install_roots(&plan.root) {
            let relative = format!("{directory}/{}/SKILL.md", module.name);
            let before = read_target(&plan.root.join(&relative), &relative)?;
            match &before {
                Some(existing) if *existing == content => continue,
                Some(existing)
                    if !String::from_utf8_lossy(existing).contains(&marker(&module.name)) =>
                {
                    conflicts.push(relative);
                    continue;
                }
                Some(existing) => plan.replaced.push((
                    relative.clone(),
                    label,
                    module.name.clone(),
                    existing.clone(),
                )),
                None => {}
            }
            plan.actions
                .push(format!("instalar el mod `{}` en {directory}", module.name));
            plan.files.push(ProjectFileChange {
                relative,
                before,
                after: content.clone(),
            });
        }
    }
    if !conflicts.is_empty() {
        plan.blocked_reason = Some(format!(
            "existen archivos que no administra ChxChx y no se sobrescribirán: {}",
            conflicts.join(", ")
        ));
    }
    Ok(())
}

fn plan_remove(plan: &mut ModsPlan, names: &[String]) -> Result<(), String> {
    for name in names {
        let module = catalog::module(name).ok_or_else(|| format!("Mod desconocido: {name}"))?;
        for (label, directory) in ALL_ROOTS {
            let relative = format!("{directory}/{}/SKILL.md", module.name);
            let Some(before) = read_target(&plan.root.join(&relative), &relative)? else {
                continue;
            };
            if !String::from_utf8_lossy(&before).contains(&marker(&module.name)) {
                plan.blocked_reason = Some(format!(
                    "{relative} no está administrado por ChxChx; no se eliminará"
                ));
                return Ok(());
            }
            plan.actions
                .push(format!("retirar el mod `{}` de {directory}", module.name));
            plan.removals.push(Removal {
                relative,
                label,
                name: module.name.clone(),
                before,
            });
        }
    }
    Ok(())
}

pub fn apply_mods_plan(plan: &ModsPlan) -> Result<ModsApplication, String> {
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("No se aplicó: {reason}"));
    }
    if !plan.changes_state() {
        return Ok(ModsApplication {
            completed: vec!["sin cambios".to_owned()],
            backups: Vec::new(),
        });
    }
    if let Some(reason) = crate::features::project_setup::invalid_project_paths(&plan.root) {
        return Err(format!("No se aplicó: {reason}"));
    }
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos().to_string();
    let backup_root = plan.root.join(".ai/backups/claude-mods").join(stamp);
    let mut backups = Vec::new();
    for (_, label, name, before) in &plan.replaced {
        backups.push(save_backup(&backup_root, label, name, before)?);
    }
    for removal in &plan.removals {
        let current = read_target(&plan.root.join(&removal.relative), &removal.relative)?;
        if current.as_deref() != Some(removal.before.as_slice()) {
            return Err(format!(
                "{} cambió desde la vista previa; vuelve a previsualizar",
                removal.relative
            ));
        }
        backups.push(save_backup(
            &backup_root,
            removal.label,
            &removal.name,
            &removal.before,
        )?);
    }
    if !plan.files.is_empty() {
        apply_changes(&plan.root, &plan.directories, &plan.files, false)?;
    }
    let mut removed: Vec<&Removal> = Vec::new();
    for removal in &plan.removals {
        let target = plan.root.join(&removal.relative);
        if let Err(error) = fs::remove_file(&target) {
            for done in removed {
                let _ = fs::write(plan.root.join(&done.relative), &done.before);
            }
            return Err(format!("No se pudo retirar {}: {error}", removal.relative));
        }
        if let Some(parent) = target.parent() {
            let _ = fs::remove_dir(parent);
        }
        removed.push(removal);
    }
    Ok(ModsApplication {
        completed: plan.actions.clone(),
        backups,
    })
}

fn save_backup(root: &Path, label: &str, name: &str, content: &[u8]) -> Result<String, String> {
    let target = root.join(label).join(name).join("SKILL.md");
    let parent = target
        .parent()
        .ok_or("No se pudo resolver la carpeta de backup")?;
    fs::create_dir_all(parent).map_err(|error| format!("No se pudo crear el backup: {error}"))?;
    fs::write(&target, content)
        .map_err(|error| format!("No se pudo guardar el backup: {error}"))?;
    Ok(target.display().to_string())
}
