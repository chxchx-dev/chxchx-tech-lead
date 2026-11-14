use crate::features::project_profiles::application_home;
use sha1::{Digest, Sha1};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub const MANAGED_ROOTS: &[&str] = &["AGENTS.md", "CLAUDE.md", ".ai"];

pub fn latest_backup(root: &Path) -> Result<Option<PathBuf>, String> {
    let home = application_home()
        .ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME para buscar backups".to_owned())?;
    let backup_root = home.join("backups");
    let project_backups = backup_root.join(project_key(root));
    if fs::symlink_metadata(&backup_root).is_ok_and(|metadata| metadata.file_type().is_symlink())
        || fs::symlink_metadata(&project_backups)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("La carpeta de backups es un enlace simbólico; no se restauró nada".to_owned());
    }
    let entries = match fs::read_dir(&project_backups) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("No se pudo leer los backups del proyecto: {error}")),
    };
    let expected_parent = project_backups
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver los backups del proyecto: {error}"))?;
    let mut candidates = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("No se pudo leer una entrada de backup: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("No se pudo inspeccionar un backup: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("No se pudo resolver un backup: {error}"))?;
        if !canonical.starts_with(&expected_parent) {
            continue;
        }
        let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
        candidates.push((modified, path));
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    Ok(candidates.pop().map(|(_, path)| path))
}

pub fn backup_project(root: &Path) -> Result<Option<PathBuf>, String> {
    let sources = MANAGED_ROOTS
        .iter()
        .map(|relative| (relative, root.join(relative)))
        .filter(|(_, source)| fs::symlink_metadata(source).is_ok())
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return Ok(None);
    }
    let home = application_home()
        .ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME para guardar backup".to_owned())?;
    let key = project_key(root);
    let target = home
        .join("backups")
        .join(key)
        .join(OffsetDateTime::now_utc().unix_timestamp_nanos().to_string());
    let resolved_root = root
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver la raíz para backup: {error}"))?;
    let resolved_target = resolve_future_path(&target)?;
    if resolved_target.starts_with(&resolved_root) {
        return Err("El destino de backup cae dentro del proyecto; no se modificó nada".to_owned());
    }
    let parent = target
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta de backups".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear el directorio de backup: {error}"))?;
    fs::create_dir(&target)
        .map_err(|error| format!("No se pudo reservar un directorio de backup: {error}"))?;
    for (relative, source) in sources {
        if let Err(error) = copy_path(&source, &target.join(relative)) {
            let _ = fs::remove_dir_all(&target);
            return Err(format!(
                "No se pudo completar el backup del proyecto: {error}"
            ));
        }
    }
    Ok(Some(target))
}

fn project_key(root: &Path) -> String {
    let digest = Sha1::digest(root.display().to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "project".to_owned());
    format!("{name}-{}", &digest[..8])
}

fn resolve_future_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("No se pudo resolver el directorio actual: {error}"))?
            .join(path)
    };
    let mut existing = absolute.as_path();
    let mut suffix = Vec::<OsString>::new();
    while !existing.exists() {
        let name = existing
            .file_name()
            .ok_or_else(|| "No se pudo resolver el destino de backup".to_owned())?;
        suffix.push(name.to_owned());
        existing = existing
            .parent()
            .ok_or_else(|| "No se pudo resolver el destino de backup".to_owned())?;
    }
    let mut resolved = existing
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver el destino de backup: {error}"))?;
    for part in suffix.iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

pub(crate) fn copy_path(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("No se pudo inspeccionar {}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(source)
            .map_err(|error| format!("No se pudo leer un enlace del proyecto: {error}"))?;
        let target_is_directory = fs::metadata(source).is_ok_and(|item| item.is_dir());
        create_symlink(&target, destination, target_is_directory)?;
    } else if metadata.is_dir() {
        fs::create_dir(destination)
            .map_err(|error| format!("No se pudo crear {}: {error}", destination.display()))?;
        let mut entries = fs::read_dir(source)
            .map_err(|error| format!("No se pudo leer {}: {error}", source.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            let name = entry
                .file_name()
                .ok_or_else(|| "Entrada de directorio sin nombre".to_owned())?;
            copy_path(&entry, &destination.join(name))?;
        }
    } else if metadata.is_file() {
        fs::copy(source, destination)
            .map_err(|error| format!("No se pudo copiar {}: {error}", source.display()))?;
    } else {
        return Err(format!(
            "Tipo de archivo no soportado: {}",
            source.display()
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &Path, destination: &Path, _directory: bool) -> Result<(), String> {
    std::os::unix::fs::symlink(target, destination)
        .map_err(|error| format!("No se pudo respaldar un enlace simbólico: {error}"))
}

#[cfg(windows)]
fn create_symlink(target: &Path, destination: &Path, directory: bool) -> Result<(), String> {
    let result = if directory {
        std::os::windows::fs::symlink_dir(target, destination)
    } else {
        std::os::windows::fs::symlink_file(target, destination)
    };
    result.map_err(|error| format!("No se pudo respaldar un enlace simbólico: {error}"))
}

#[cfg(not(any(unix, windows)))]
fn create_symlink(_target: &Path, _destination: &Path, _directory: bool) -> Result<(), String> {
    Err("Este sistema no permite preservar enlaces simbólicos en el backup".to_owned())
}
