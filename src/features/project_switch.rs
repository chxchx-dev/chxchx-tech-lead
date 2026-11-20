//! `projects switch`: suspends the workspaces that are active, activates another registered
//! project and remembers it as the last one. Nothing runs without that project's local trust
//! (`workspace_lifecycle::start` checks it) and `--dry-run` changes nothing.
use crate::features::project_catalog::{ProjectCatalog, inspect_registered_projects};
use crate::features::project_registry::{apply_registration, plan_last_project};
use crate::features::workspace_lifecycle::{self, WorkspaceAction};
use crate::features::workspace_resources::read_workspace_status;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Switch {
    pub alias: String,
    pub name: String,
    pub path: PathBuf,
    /// The workspaces that were active and were suspended, with what happened to each.
    pub suspended: Vec<(PathBuf, WorkspaceAction)>,
    pub started: WorkspaceAction,
    /// Whether `last_project` was (or, in a preview, would be) updated.
    pub remembered: bool,
}

fn canonical(path: &str) -> Option<PathBuf> {
    Path::new(path)
        .is_dir()
        .then(|| std::fs::canonicalize(path).ok())
        .flatten()
}

/// The registered project called `alias` (its alias, or its name when it has no alias).
fn resolve(catalog: &ProjectCatalog, alias: &str) -> Result<(String, String, PathBuf), String> {
    let project = catalog
        .projects
        .iter()
        .find(|project| project.alias == alias)
        .ok_or_else(|| format!("No hay un proyecto registrado con el alias `{alias}`"))?;
    let path = project
        .path
        .as_deref()
        .and_then(canonical)
        .ok_or_else(|| format!("El proyecto `{alias}` ya no está disponible en disco"))?;
    Ok((project.alias.clone(), project.name.clone(), path))
}

pub fn switch(alias: &str, dry_run: bool) -> Result<Switch, String> {
    let catalog = inspect_registered_projects()?;
    let (alias, name, target) = resolve(&catalog, alias)?;
    // Check the target first: a project that cannot start must not leave the others suspended.
    let trusted = crate::features::project_overview::inspect_project(&target)?
        .workspace
        .trusted;
    if !trusted {
        return Err(format!(
            "`{alias}` no es confiable; ejecuta `workspace trust --apply` en {} antes de activarlo",
            target.display()
        ));
    }
    let mut suspended = Vec::new();
    for other in &catalog.projects {
        let Some(other) = other.path.as_deref().and_then(canonical) else {
            continue;
        };
        if other == target || read_workspace_status(&other) != "ACTIVE" {
            continue;
        }
        let action = workspace_lifecycle::suspend(&other, dry_run)
            .map_err(|error| format!("No pude suspender {}: {error}", other.display()))?;
        suspended.push((other, action));
    }
    let started = workspace_lifecycle::start(&target, dry_run)?;
    let update = plan_last_project(&target.display().to_string())?;
    if !dry_run {
        apply_registration(&update)?;
    }
    Ok(Switch {
        alias,
        name,
        path: target,
        suspended,
        started,
        remembered: update.changed,
    })
}

#[cfg(test)]
#[path = "project_switch_tests.rs"]
mod tests;
