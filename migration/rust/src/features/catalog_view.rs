//! Read-only snapshot of the catalog for one project, shared by CLI and Studio.
use crate::features::catalog;
use crate::features::project_changes::resolve_root;
use crate::features::project_detection::detect_project;
use crate::features::project_mods::{ModStatus, mod_status};
use crate::features::project_skills::enabled_skills;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct SkillStatus {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub recommended: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PackStatus {
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
    pub enabled_skills: usize,
    pub recommended: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CatalogView {
    pub project_root: String,
    pub skills: Vec<SkillStatus>,
    pub packs: Vec<PackStatus>,
    pub mods: Vec<ModStatus>,
    /// Set when `.ai/chxchx-skills.toml` is unreadable; the rest is still listed.
    pub selection_error: Option<String>,
}

pub fn inspect_catalog(path: &Path) -> Result<CatalogView, String> {
    let root = resolve_root(path)?;
    let detection = detect_project(&root);
    let (enabled, selection_error) = match enabled_skills(&root) {
        Ok(enabled) => (enabled, None),
        Err(error) => (Vec::new(), Some(error)),
    };
    let skills = catalog::skills()
        .iter()
        .map(|skill| {
            let reasons = catalog::skill_reasons(&detection, skill);
            SkillStatus {
                name: skill.name.clone(),
                description: skill.description.clone(),
                tags: skill.tags.clone(),
                enabled: enabled.contains(&skill.name),
                recommended: !reasons.is_empty(),
                reasons,
            }
        })
        .collect();
    let packs = catalog::packs()
        .iter()
        .map(|pack| {
            let reasons = catalog::pack_reasons(&detection, pack);
            PackStatus {
                name: pack.name.clone(),
                description: pack.description.clone(),
                skills: pack.skills.clone(),
                enabled_skills: pack
                    .skills
                    .iter()
                    .filter(|name| enabled.contains(name))
                    .count(),
                recommended: !reasons.is_empty(),
                reasons,
            }
        })
        .collect();
    Ok(CatalogView {
        project_root: root.display().to_string(),
        skills,
        packs,
        mods: mod_status(&root, &detection),
        selection_error,
    })
}
