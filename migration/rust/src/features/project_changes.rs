//! Shared transaction for features that rewrite a few project files:
//! snapshot check, backup, atomic writes and rollback on failure.
use crate::features::project_backup::backup_project;
use crate::features::project_setup::{
    ProjectFileChange, append_rollback, directory_resolves_inside, invalid_project_paths,
    missing_directory_chain, rollback_project, validate_file_snapshot, write_project_file,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Directories that must be created before `files` can be written.
pub(crate) fn missing_directories(
    root: &Path,
    files: &[ProjectFileChange],
) -> Result<Vec<PathBuf>, String> {
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::new();
    for change in files {
        let parent = Path::new(&change.relative)
            .parent()
            .and_then(Path::to_str)
            .unwrap_or("");
        if parent.is_empty() {
            continue;
        }
        for directory in missing_directory_chain(root, &parent.replace('\\', "/"))? {
            if seen.insert(directory.clone()) {
                ordered.push(directory);
            }
        }
    }
    Ok(ordered)
}

/// Applies a previewed change set. With `project_backup` the managed project files
/// (`AGENTS.md`, `CLAUDE.md`, `.ai`) are saved first; returns the backups created.
pub(crate) fn apply_changes(
    root: &Path,
    directories: &[PathBuf],
    files: &[ProjectFileChange],
    project_backup: bool,
) -> Result<Vec<String>, String> {
    if let Some(reason) = invalid_project_paths(root) {
        return Err(format!("No se aplicó: {reason}"));
    }
    for change in files {
        validate_file_snapshot(root, change)?;
    }
    let mut backups = Vec::new();
    if project_backup && let Some(path) = backup_project(root)? {
        backups.push(path.display().to_string());
    }
    let mut created = Vec::new();
    for directory in directories {
        match fs::create_dir(directory) {
            Ok(()) => created.push(directory.clone()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if !directory.is_dir() || !directory_resolves_inside(root, directory) {
                    let rollback = rollback_project(root, &[], &created);
                    return Err(append_rollback(
                        format!(
                            "El destino dejó de ser un directorio válido: {}",
                            directory.display()
                        ),
                        rollback,
                    ));
                }
            }
            Err(error) => {
                let rollback = rollback_project(root, &[], &created);
                return Err(append_rollback(
                    format!("No se pudo crear un directorio: {error}"),
                    rollback,
                ));
            }
        }
    }
    let mut applied = Vec::new();
    for change in files {
        let result =
            validate_file_snapshot(root, change).and_then(|()| write_project_file(root, change));
        if let Err(error) = result {
            let rollback = rollback_project(root, &applied, &created);
            return Err(append_rollback(error, rollback));
        }
        applied.push(change.clone());
    }
    Ok(backups)
}

/// Canonical project root without the Windows `\?\` prefix.
pub(crate) fn resolve_root(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        format!(
            "No se pudo resolver la ruta de proyecto {}: {error}",
            path.display()
        )
    })?;
    let root = crate::features::project_overview::normalize_canonical_path(canonical);
    if root.is_dir() {
        Ok(root)
    } else {
        Err(format!(
            "La ruta de proyecto no es un directorio: {}",
            root.display()
        ))
    }
}
