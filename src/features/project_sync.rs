use crate::features::project_backup::backup_project;
use crate::features::project_catalog::{
    ProjectRegistrySnapshot, inspect_registered_projects, read_project_registry_snapshot,
};
use crate::features::project_overview::{inspect_project, normalize_canonical_path};
use crate::features::project_setup::{
    ProjectFileChange, plan_managed_file, rollback_project, validate_file_snapshot,
    write_project_file,
};
use crate::features::project_templates::{claude_rules_body, project_rules_body};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct ProjectSyncEntry {
    pub alias: String,
    pub project_root: String,
    pub changes: Vec<String>,
    pub blocked_reason: Option<String>,
    root: PathBuf,
    files: Vec<ProjectFileChange>,
}

#[derive(Clone)]
pub struct ProjectSyncPlan {
    pub entries: Vec<ProjectSyncEntry>,
    pub warnings: Vec<String>,
    registry_path: PathBuf,
    registry_before: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct ProjectSyncOutcome {
    pub project_root: String,
    pub changed_files: Vec<String>,
    pub backup: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectSyncApplication {
    pub outcomes: Vec<ProjectSyncOutcome>,
    pub warnings: Vec<String>,
}

pub fn plan_projects_sync() -> Result<ProjectSyncPlan, String> {
    let registry_snapshot = read_project_registry_snapshot()?;
    let catalog = inspect_registered_projects()?;
    if read_project_registry_snapshot()? != registry_snapshot {
        return Err("projects.json cambió durante la vista previa; vuelve a intentarlo".to_owned());
    }

    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_roots = BTreeSet::new();
    for project in catalog.projects {
        let Some(raw_path) = project.path else {
            warnings.push(format!(
                "{} omitido: el registro no contiene una ruta",
                project.alias
            ));
            continue;
        };
        let path = PathBuf::from(&raw_path);
        if !path.is_dir() {
            warnings.push(format!("{} omitido: no existe {}", project.alias, raw_path));
            continue;
        }
        let canonical = match path.canonicalize() {
            Ok(canonical) => normalize_canonical_path(canonical),
            Err(error) => {
                warnings.push(format!(
                    "{} omitido: no se pudo resolver la ruta: {error}",
                    project.alias
                ));
                continue;
            }
        };
        if !seen_roots.insert(canonical.clone()) {
            warnings.push(format!(
                "{} omitido: la ruta ya aparece en el registro",
                project.alias
            ));
            continue;
        }
        match plan_project(&project.alias, &canonical) {
            Ok(entry) => {
                if let Some(reason) = &entry.blocked_reason {
                    warnings.push(format!("{} omitido: {reason}", entry.project_root));
                }
                entries.push(entry);
            }
            Err(error) => warnings.push(format!("{} omitido: {error}", project.alias)),
        }
    }
    Ok(ProjectSyncPlan {
        entries,
        warnings,
        registry_path: registry_snapshot.path,
        registry_before: registry_snapshot.contents,
    })
}

pub fn apply_projects_sync(plan: &ProjectSyncPlan) -> Result<ProjectSyncApplication, String> {
    if read_project_registry_snapshot()?
        != (ProjectRegistrySnapshot {
            path: plan.registry_path.clone(),
            contents: plan.registry_before.clone(),
        })
    {
        return Err(
            "projects.json cambió después de la vista previa; vuelve a previsualizar".to_owned(),
        );
    }

    let mut outcomes = Vec::new();
    let mut warnings = plan.warnings.clone();
    for entry in &plan.entries {
        if entry.blocked_reason.is_some() {
            continue;
        }
        if entry.files.is_empty() {
            outcomes.push(ProjectSyncOutcome {
                project_root: entry.project_root.clone(),
                changed_files: Vec::new(),
                backup: None,
            });
            continue;
        }
        if let Err(error) = validate_project_root(&entry.root) {
            warnings.push(format!("{}: {error}", entry.project_root));
            continue;
        }
        if let Err(error) = entry
            .files
            .iter()
            .try_for_each(|change| validate_file_snapshot(&entry.root, change))
        {
            warnings.push(format!("{}: {error}", entry.project_root));
            continue;
        }
        let backup = match backup_project(&entry.root) {
            Ok(path) => path.map(|path| path.display().to_string()),
            Err(error) => {
                warnings.push(format!(
                    "{}: no se pudo crear el backup: {error}",
                    entry.project_root
                ));
                continue;
            }
        };

        let mut applied = Vec::new();
        let mut failure = None;
        for change in &entry.files {
            if let Err(error) = validate_project_root(&entry.root)
                .and_then(|()| validate_file_snapshot(&entry.root, change))
                .and_then(|()| write_project_file(&entry.root, change))
            {
                failure = Some(error);
                break;
            }
            applied.push(change.clone());
        }
        if let Some(error) = failure {
            let rollback = if validate_project_root(&entry.root).is_ok() {
                rollback_project(&entry.root, &applied, &[])
            } else {
                vec!["no se pudo verificar la raíz para hacer rollback".to_owned()]
            };
            let detail = if rollback.is_empty() {
                error
            } else {
                format!("{error}. Rollback incompleto: {}", rollback.join("; "))
            };
            warnings.push(format!("{}: {detail}", entry.project_root));
            continue;
        }
        outcomes.push(ProjectSyncOutcome {
            project_root: entry.project_root.clone(),
            changed_files: entry
                .files
                .iter()
                .map(|change| change.relative.clone())
                .collect(),
            backup,
        });
    }
    Ok(ProjectSyncApplication { outcomes, warnings })
}

fn plan_project(alias: &str, path: &Path) -> Result<ProjectSyncEntry, String> {
    let overview = inspect_project(path)?;
    let root = PathBuf::from(&overview.project.root);
    let managed_files = [
        (
            "AGENTS.md",
            "project-rules",
            project_rules_body(&root, &overview.project.name, &overview.detection),
        ),
        ("CLAUDE.md", "claude-rules", claude_rules_body().to_owned()),
    ];
    let mut files = Vec::new();
    let mut changes = Vec::new();
    let mut blocked_reason = None;
    for (relative, marker, body) in managed_files {
        match plan_managed_file(&root, relative, marker, &body) {
            Ok(Some(change)) => {
                changes.push(format!("actualizar bloque administrado en {relative}"));
                files.push(change);
            }
            Ok(None) => {}
            Err(error) => {
                blocked_reason = Some(error);
                files.clear();
                changes.clear();
                break;
            }
        }
    }
    Ok(ProjectSyncEntry {
        alias: alias.to_owned(),
        project_root: root.display().to_string(),
        changes,
        blocked_reason,
        root,
        files,
    })
}

fn validate_project_root(root: &Path) -> Result<(), String> {
    let canonical = root
        .canonicalize()
        .map_err(|error| format!("no se pudo resolver el proyecto: {error}"))?;
    let canonical = normalize_canonical_path(canonical);
    if !canonical.is_dir() || !paths_equal(&canonical, root) {
        return Err("la ruta del proyecto cambió desde la vista previa".to_owned());
    }
    Ok(())
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}
