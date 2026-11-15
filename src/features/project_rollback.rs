use crate::features::project_backup::copy_path;
use crate::features::project_backup::{MANAGED_ROOTS, backup_project, latest_backup};
use crate::features::project_overview::inspect_project;
use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Eq, PartialEq)]
struct SnapshotEntry {
    path: String,
    signature: String,
}

#[derive(Clone)]
pub struct ProjectRollbackPlan {
    pub project_root: String,
    pub backup_path: String,
    pub actions: Vec<String>,
    pub blocked_reason: Option<String>,
    pub changes_state: bool,
    root: PathBuf,
    backup: PathBuf,
    current_snapshot: Vec<SnapshotEntry>,
    backup_snapshot: Vec<SnapshotEntry>,
}

pub fn plan_project_rollback(path: &Path) -> Result<ProjectRollbackPlan, String> {
    let overview = inspect_project(path)?;
    let root = PathBuf::from(overview.project.root);
    if !root.is_dir() {
        return Err("La ruta del proyecto no es un directorio".to_owned());
    }
    let backup =
        latest_backup(&root)?.ok_or_else(|| "No hay backups para este proyecto".to_owned())?;
    validate_managed_roots(&root)?;
    validate_managed_roots(&backup)?;
    let current_snapshot = snapshot_roots(&root)?;
    let backup_snapshot = snapshot_roots(&backup)?;
    if backup_snapshot.is_empty() {
        return Err("El último backup no contiene archivos administrados".to_owned());
    }

    let mut actions = Vec::new();
    let mut changes_state = false;
    for managed in MANAGED_ROOTS {
        let current = snapshot_for_root(&current_snapshot, managed);
        let saved = snapshot_for_root(&backup_snapshot, managed);
        if current == saved {
            continue;
        }
        changes_state = true;
        if saved.is_empty() {
            actions.push(format!("eliminar {managed} (no existe en el backup)"));
        } else {
            actions.push(format!("restaurar {managed} desde el backup"));
        }
    }
    if changes_state {
        actions.push("crear backup de seguridad del estado actual".to_owned());
    } else {
        actions.push("el estado actual ya coincide con el último backup".to_owned());
    }

    Ok(ProjectRollbackPlan {
        project_root: root.display().to_string(),
        backup_path: backup.display().to_string(),
        actions,
        blocked_reason: None,
        changes_state,
        root,
        backup,
        current_snapshot,
        backup_snapshot,
    })
}

pub fn apply_project_rollback(
    plan: &ProjectRollbackPlan,
) -> Result<(Vec<String>, Option<String>), String> {
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("Rollback bloqueado: {reason}"));
    }
    if !plan.changes_state {
        return Ok((plan.actions.clone(), None));
    }
    let newest = latest_backup(&plan.root)?
        .ok_or_else(|| "El backup dejó de estar disponible; vuelve a previsualizar".to_owned())?;
    if newest != plan.backup {
        return Err("Hay un backup más reciente; vuelve a previsualizar rollback".to_owned());
    }
    validate_managed_roots(&plan.root)?;
    validate_managed_roots(&plan.backup)?;
    if snapshot_roots(&plan.root)? != plan.current_snapshot {
        return Err("Los archivos administrados cambiaron desde la vista previa".to_owned());
    }
    if snapshot_roots(&plan.backup)? != plan.backup_snapshot {
        return Err("El backup cambió desde la vista previa".to_owned());
    }

    let safety_backup = backup_project(&plan.root)?;
    if let Err(error) = replace_managed_roots(&plan.root, Some(&plan.backup)) {
        let recovery = match safety_backup.as_deref() {
            Some(path) => replace_managed_roots(&plan.root, Some(path)),
            None => replace_managed_roots(&plan.root, None),
        };
        return match recovery {
            Ok(()) => Err(format!(
                "Rollback falló y se restauró el estado anterior: {error}"
            )),
            Err(recovery_error) => Err(format!(
                "Rollback falló: {error}. También falló la recuperación automática: {recovery_error}"
            )),
        };
    }
    Ok((
        plan.actions.clone(),
        safety_backup.map(|path| path.display().to_string()),
    ))
}

fn replace_managed_roots(root: &Path, source: Option<&Path>) -> Result<(), String> {
    for relative in MANAGED_ROOTS {
        let destination = root.join(relative);
        remove_path(&destination)?;
        if let Some(source) = source {
            let saved = source.join(relative);
            if fs::symlink_metadata(&saved).is_ok() {
                copy_path(&saved, &destination)?;
            }
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "No se pudo inspeccionar {}: {error}",
                path.display()
            ));
        }
    };
    if metadata.file_type().is_symlink() {
        #[cfg(windows)]
        let result = if fs::metadata(path).is_ok_and(|target| target.is_dir()) {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        };
        #[cfg(not(windows))]
        let result = fs::remove_file(path);
        result.map_err(|error| format!("No se pudo quitar el enlace {}: {error}", path.display()))
    } else if metadata.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("No se pudo quitar {}: {error}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("No se pudo quitar {}: {error}", path.display()))
    }
}

fn validate_managed_roots(root: &Path) -> Result<(), String> {
    for relative in MANAGED_ROOTS {
        let path = root.join(relative);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("No se pudo inspeccionar {relative}: {error}")),
        };
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "{relative} es un enlace simbólico; rollback no lo atraviesa"
            ));
        }
        let expected_directory = *relative == ".ai";
        if metadata.is_dir() != expected_directory || (!expected_directory && !metadata.is_file()) {
            return Err(format!(
                "Tipo inesperado para {relative}; no se restauró nada"
            ));
        }
    }
    Ok(())
}

fn snapshot_roots(root: &Path) -> Result<Vec<SnapshotEntry>, String> {
    let mut result = Vec::new();
    for relative in MANAGED_ROOTS {
        let path = root.join(relative);
        if fs::symlink_metadata(&path).is_ok() {
            snapshot_path(root, &path, &mut result)?;
        }
    }
    result.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(result)
}

fn snapshot_path(root: &Path, path: &Path, result: &mut Vec<SnapshotEntry>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
    let signature = if metadata.file_type().is_symlink() {
        format!(
            "link:{}",
            fs::read_link(path)
                .map_err(|error| {
                    format!("No se pudo leer el enlace {}: {error}", path.display())
                })?
                .display()
        )
    } else if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("No se pudo enumerar {}: {error}", path.display()))?;
        entries.sort();
        result.push(SnapshotEntry {
            path: relative_label(root, path),
            signature: "directory".to_owned(),
        });
        for entry in entries {
            snapshot_path(root, &entry, result)?;
        }
        return Ok(());
    } else if metadata.is_file() {
        let bytes = fs::read(path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
        let digest = Sha1::digest(bytes);
        format!(
            "file:{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    } else {
        "special".to_owned()
    };
    result.push(SnapshotEntry {
        path: relative_label(root, path),
        signature,
    });
    Ok(())
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn snapshot_for_root<'a>(snapshot: &'a [SnapshotEntry], root: &str) -> Vec<(&'a str, &'a str)> {
    snapshot
        .iter()
        .filter(|entry| entry.path == root || entry.path.starts_with(&format!("{root}/")))
        .map(|entry| (entry.path.as_str(), entry.signature.as_str()))
        .collect()
}
