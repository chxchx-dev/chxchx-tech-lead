//! Bounded project file access shared by presentations. Never follows project links.
use crate::features::project_profiles::application_home;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use time::OffsetDateTime;

const MAX_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Entry {
    pub relative: PathBuf,
    pub directory: bool,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub relative: PathBuf,
    pub text: String,
    pub fingerprint: Vec<u8>,
}

fn resolve(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut path = root.clone();
    for part in relative.components() {
        let Component::Normal(name) = part else {
            return Err("La ruta debe permanecer dentro del proyecto".into());
        };
        if name.to_string_lossy().starts_with(".env") {
            return Err(
                "Los archivos de entorno se mantienen fuera del editor y sus respaldos".into(),
            );
        }
        path.push(name);
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("No se siguen enlaces simbólicos del proyecto".into());
        }
    }
    let resolved = path.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(root) {
        return Err("La ruta sale del proyecto".into());
    }
    Ok(resolved)
}

pub fn list(root: &Path, relative: &Path) -> Result<Vec<Entry>, String> {
    let path = resolve(root, relative)?;
    let mut entries = Vec::new();
    for (index, entry) in fs::read_dir(path).map_err(|e| e.to_string())?.enumerate() {
        if index >= 2000 {
            return Err("La carpeta supera el límite de 2000 entradas".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let label = name.to_string_lossy();
        if matches!(
            label.as_ref(),
            ".git" | "node_modules" | "target" | ".venv" | "__pycache__"
        ) || label.starts_with(".env")
        {
            continue;
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if !kind.is_symlink() && (kind.is_dir() || kind.is_file()) {
            entries.push(Entry {
                relative: relative.join(name),
                directory: kind.is_dir(),
            });
        }
    }
    if entries.len() > 2000 {
        return Err("La carpeta supera el límite de 2000 entradas".into());
    }
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then(a.relative.cmp(&b.relative))
    });
    Ok(entries)
}

pub fn read(root: &Path, relative: &Path) -> Result<Document, String> {
    let path = resolve(root, relative)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES || bytes.contains(&0) {
        return Err("Solo se abren archivos de texto UTF-8 de hasta 2 MiB".into());
    }
    let fingerprint = Sha256::digest(&bytes).to_vec();
    let text = String::from_utf8(bytes).map_err(|_| "El archivo no es texto UTF-8")?;
    Ok(Document {
        relative: relative.into(),
        text,
        fingerprint,
    })
}

pub fn save(root: &Path, document: &Document, text: &str) -> Result<Document, String> {
    let home = application_home().ok_or("No se pudo localizar la carpeta de respaldos")?;
    save_with_home(root, document, text, &home)
}

fn save_with_home(
    root: &Path,
    document: &Document,
    text: &str,
    home: &Path,
) -> Result<Document, String> {
    if text.len() as u64 > MAX_BYTES {
        return Err("El documento supera 2 MiB".into());
    }
    let current = read(root, &document.relative)?;
    if current.fingerprint != document.fingerprint {
        return Err(
            "El archivo cambió en disco. Conserva tus cambios y vuelve a abrirlo antes de guardar"
                .into(),
        );
    }
    if current.text == text {
        return Ok(current);
    }
    let path = resolve(root, &document.relative)?;
    if !home.is_absolute() {
        return Err("La carpeta de respaldos debe ser absoluta".into());
    }
    if fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .permissions()
        .readonly()
    {
        return Err("El archivo es de solo lectura".into());
    }
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let backup = home.join("editor-backups").join(format!(
        "{:x}",
        Sha256::digest(path.to_string_lossy().as_bytes())
    ));
    let mut parent = PathBuf::new();
    for part in backup.components() {
        parent.push(part);
        if fs::symlink_metadata(&parent).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("La ruta de respaldos contiene un enlace".into());
        }
    }
    fs::create_dir_all(&backup).map_err(|e| e.to_string())?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(backup.join(format!("{stamp}.bak")))
        .and_then(|mut file| file.write_all(current.text.as_bytes()))
        .map_err(|e| e.to_string())?;
    let temporary = path.with_file_name(format!(".chxchx-save-{}-{stamp}", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        fs::set_permissions(
            &temporary,
            fs::metadata(&path)
                .map_err(|e| e.to_string())?
                .permissions(),
        )
        .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        if read(root, &document.relative)?.fingerprint != document.fingerprint {
            return Err("El archivo cambió durante el guardado; no se reemplazó".into());
        }
        fs::rename(&temporary, &path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    read(root, &document.relative)
}

#[cfg(test)]
#[path = "project_files_tests.rs"]
mod tests;
