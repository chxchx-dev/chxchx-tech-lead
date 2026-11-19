//! Review and remove noisy automatic captures without changing handwritten notes.

use crate::features::explicit_memory::{
    MAX_MEMORY_FILE_BYTES, is_explicit_request, user_visible_text,
};
use crate::features::project_profiles::application_home;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryPruneResult {
    pub path: PathBuf,
    pub removed: Vec<String>,
    pub backup: Option<PathBuf>,
}

/// Preview or remove automatic memory entries rejected by the current detector.
pub fn prune(project: &Path, dry_run: bool) -> Result<MemoryPruneResult, String> {
    let root = project
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver el proyecto: {error}"))?;
    let ai = root.join(".ai");
    let memory_dir = ai.join("memory");
    let path = memory_dir.join("PROJECT_MEMORY.md");
    for directory in [&ai, &memory_dir] {
        match fs::symlink_metadata(directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(MemoryPruneResult {
                    path,
                    removed: Vec::new(),
                    backup: None,
                });
            }
            Err(error) => {
                return Err(format!(
                    "No se pudo inspeccionar {}: {error}",
                    directory.display()
                ));
            }
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(format!("{} no es una carpeta segura", directory.display()));
            }
        }
    }

    let metadata = match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MemoryPruneResult {
                path,
                removed: Vec::new(),
                backup: None,
            });
        }
        Err(error) => return Err(format!("No se pudo inspeccionar la memoria: {error}")),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err("PROJECT_MEMORY.md no es un archivo regular".to_owned());
        }
        Ok(metadata) if metadata.len() > MAX_MEMORY_FILE_BYTES => {
            return Err("PROJECT_MEMORY.md supera el límite de lectura".to_owned());
        }
        Ok(metadata) => metadata,
    };
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("No se pudo leer PROJECT_MEMORY.md: {error}"))?;
    let (pruned, removed) = prune_content(&content);
    if removed.is_empty() || dry_run {
        return Ok(MemoryPruneResult {
            path,
            removed,
            backup: None,
        });
    }

    // Recheck the file before backup and replacement. This also narrows the window for a path swap.
    let current = fs::symlink_metadata(&path)
        .map_err(|error| format!("No se pudo volver a inspeccionar la memoria: {error}"))?;
    if current.file_type().is_symlink() || !current.is_file() || current.len() != metadata.len() {
        return Err("PROJECT_MEMORY.md cambió durante la limpieza; no se modificó".to_owned());
    }
    let backup = backup_memory_file(&root, &path)?;
    if let Err(error) = atomic_replace(&path, &pruned) {
        return Err(format!(
            "{error}. El original sigue disponible en {}",
            backup.display()
        ));
    }

    Ok(MemoryPruneResult {
        path,
        removed,
        backup: Some(backup),
    })
}

fn prune_content(content: &str) -> (String, Vec<String>) {
    let entries = automatic_entries(content);
    let mut kept = String::with_capacity(content.len());
    let mut removed = Vec::new();
    let mut cursor = 0;
    let mut seen = HashSet::new();

    for (start, end, text) in entries {
        kept.push_str(&content[cursor..start]);
        let normalized = text.to_lowercase();
        if is_explicit_request(&text) && seen.insert(normalized) {
            kept.push_str(&content[start..end]);
        } else {
            removed.push(text);
        }
        cursor = end;
    }
    kept.push_str(&content[cursor..]);
    (kept, removed)
}

fn automatic_entries(content: &str) -> Vec<(usize, usize, String)> {
    let lines = content.split_inclusive('\n').collect::<Vec<_>>();
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    let mut offset = 0;
    for line in &lines {
        offsets.push(offset);
        offset += line.len();
    }
    offsets.push(offset);

    let mut entries = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let heading = line_text(lines[index]);
        if !heading.starts_with("### Petición explícita de memoria — ") {
            index += 1;
            continue;
        }
        let start = offsets[index];
        let mut end_index = index + 1;
        while end_index < lines.len() && !markdown_heading(line_text(lines[end_index])) {
            end_index += 1;
        }
        let end = offsets[end_index];
        let section = &content[start..end];
        if has_generated_marker(section) {
            let text = section
                .lines()
                .find_map(|line| line.strip_prefix("> "))
                .map(user_visible_text)
                .unwrap_or_default();
            entries.push((start, end, text));
        }
        index = end_index.max(index + 1);
    }
    entries
}

fn line_text(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn markdown_heading(line: &str) -> bool {
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ')
}

fn has_generated_marker(section: &str) -> bool {
    let mut lines = section.lines();
    let Some(heading) = lines.next() else {
        return false;
    };
    heading.starts_with("### Petición explícita de memoria — ")
        && lines.next().is_some_and(str::is_empty)
        && lines
            .next()
            .is_some_and(|line| line.starts_with("<!-- explicit-memory:") && line.ends_with("-->"))
}

fn backup_memory_file(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let home = application_home()
        .ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME para guardar backup".to_owned())?;
    backup_memory_file_at(root, path, &home)
}

fn backup_memory_file_at(root: &Path, path: &Path, home: &Path) -> Result<PathBuf, String> {
    let planned_home = crate::features::project_backup::resolve_future_path(home)?;
    if planned_home.starts_with(root) {
        return Err("El backup quedaría dentro del proyecto; no se modificó la memoria".to_owned());
    }
    fs::create_dir_all(home)
        .map_err(|error| format!("No se pudo preparar CHXCHX_TECH_HOME: {error}"))?;
    let home = home
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver CHXCHX_TECH_HOME: {error}"))?;
    if home.starts_with(root) {
        return Err("El backup quedaría dentro del proyecto; no se modificó la memoria".to_owned());
    }

    let backups = child_directory(&home, "backups")?;
    let project_name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "project".to_owned());
    let project_hash = Sha256::digest(root.to_string_lossy().as_bytes())
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let project_backups = child_directory(&backups, &format!("{project_name}-{project_hash}"))?;
    let memory_backups = child_directory(&project_backups, "memory")?;
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos().to_string();
    let backup_dir = child_directory(&memory_backups, &stamp)?;
    let backup = backup_dir.join("PROJECT_MEMORY.md");
    if let Err(error) = fs::copy(path, &backup) {
        let _ = fs::remove_dir_all(&backup_dir);
        return Err(format!("No se pudo guardar el backup de memoria: {error}"));
    }
    Ok(backup)
}

fn child_directory(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(path),
        Ok(_) => Err(format!("{} no es una carpeta segura", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&path)
                .map_err(|error| format!("No se pudo crear {}: {error}", path.display()))?;
            Ok(path)
        }
        Err(error) => Err(format!(
            "No se pudo inspeccionar {}: {error}",
            path.display()
        )),
    }
}

fn atomic_replace(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta de memoria".to_owned())?;
    let temporary = parent.join(format!(
        ".project-memory-prune-{}-{}.tmp",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("No se pudo preparar la memoria temporal: {error}"))?;
    if let Err(error) = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
    {
        let _ = fs::remove_file(&temporary);
        return Err(format!("No se pudo escribir la memoria temporal: {error}"));
    }
    drop(file);

    let backup = parent.join(format!(
        ".project-memory-prune-backup-{}-{}.tmp",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    fs::rename(path, &backup).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("No se pudo conservar la memoria anterior: {error}")
    })?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        let _ = fs::rename(&backup, path);
        return Err(format!("No se pudo guardar PROJECT_MEMORY.md: {error}"));
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

#[cfg(test)]
#[path = "explicit_memory_prune_tests.rs"]
mod tests;
