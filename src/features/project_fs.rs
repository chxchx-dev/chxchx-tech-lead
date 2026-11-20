//! File management inside a project, for presentations that organize it (the Studio explorer):
//! create, rename, copy, move, duplicate and delete.
//!
//! Everything stays inside the project root, never follows links, never touches the protected
//! entries the explorer hides (`.git`, environment files…) and is bounded. Deleting moves the
//! item to a recoverable trash under the application home, never straight out of existence.
use crate::features::project_files::resolve;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// What the explorer hides: creating these would make the new entry invisible.
const HIDDEN: [&str; 5] = [".git", "node_modules", "target", ".venv", "__pycache__"];
const MAX_NAME_BYTES: usize = 255;
const MAX_ENTRIES: usize = 20_000;
const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A deleted item waiting in the trash, enough to put it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trashed {
    pub original: PathBuf,
    pub location: PathBuf,
}

/// A name that is valid on every system a project may be shared with.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." {
        return Err("Escribe un nombre".into());
    }
    if name.len() > MAX_NAME_BYTES {
        return Err("El nombre es demasiado largo (máximo 255 bytes)".into());
    }
    if name.starts_with(' ') || name.ends_with(' ') || name.ends_with('.') {
        return Err(
            "El nombre no puede empezar con espacio ni terminar con espacio o punto".into(),
        );
    }
    if let Some(bad) = name
        .chars()
        .find(|c| c.is_control() || "<>:\"/\\|?*".contains(*c))
    {
        return Err(format!(
            "El nombre no puede contener `{}`",
            bad.escape_default()
        ));
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        return Err(format!("`{name}` es un nombre reservado en Windows"));
    }
    if name.starts_with(".env") {
        return Err("Los archivos de entorno se mantienen fuera del explorador".into());
    }
    if HIDDEN.contains(&name) {
        return Err(format!("`{name}` es una carpeta que el explorador oculta"));
    }
    Ok(())
}

fn protected(relative: &Path) -> Result<(), String> {
    let hit = relative.components().any(|part| {
        let name = part.as_os_str().to_string_lossy();
        HIDDEN.contains(&name.as_ref()) || name.starts_with(".env")
    });
    if hit {
        Err("Esa ruta está protegida y no se gestiona desde el explorador".into())
    } else {
        Ok(())
    }
}

/// An existing item of the project that may be operated on (never the root itself).
fn existing(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.as_os_str().is_empty() {
        return Err("No se puede operar sobre la carpeta raíz del proyecto".into());
    }
    protected(relative)?;
    resolve(root, relative)
}

fn directory(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if !relative.as_os_str().is_empty() {
        protected(relative)?;
    }
    let path = resolve(root, relative)?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err("El destino no es una carpeta".into())
    }
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// The relative path of `absolute` inside the project, or an error if it left it.
fn relative_to(root: &Path, absolute: &Path) -> Result<PathBuf, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    absolute
        .strip_prefix(&root)
        .map(Path::to_path_buf)
        .map_err(|_| "La ruta sale del proyecto".into())
}

pub fn create_file(root: &Path, parent: &Path, name: &str) -> Result<PathBuf, String> {
    validate_name(name)?;
    let target = directory(root, parent)?.join(name);
    if exists(&target) {
        return Err(format!("Ya existe `{name}` en esta carpeta"));
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| e.to_string())?;
    relative_to(root, &target)
}

pub fn create_folder(root: &Path, parent: &Path, name: &str) -> Result<PathBuf, String> {
    validate_name(name)?;
    let target = directory(root, parent)?.join(name);
    if exists(&target) {
        return Err(format!("Ya existe `{name}` en esta carpeta"));
    }
    fs::create_dir(&target).map_err(|e| e.to_string())?;
    relative_to(root, &target)
}

pub fn rename(root: &Path, relative: &Path, new_name: &str) -> Result<PathBuf, String> {
    validate_name(new_name)?;
    let source = existing(root, relative)?;
    let target = source.with_file_name(new_name);
    if source == target {
        return Ok(relative.to_path_buf());
    }
    // A case-only rename is the same file on case-insensitive systems: allowed.
    let case_only = source
        .file_name()
        .is_some_and(|old| old.to_string_lossy().eq_ignore_ascii_case(new_name));
    if exists(&target) && !case_only {
        return Err(format!("Ya existe `{new_name}` en esta carpeta"));
    }
    fs::rename(&source, &target).map_err(|e| e.to_string())?;
    relative_to(root, &target)
}

/// "a.txt" → "a copia.txt" → "a copia 2.txt": a free name next to the original.
pub fn unique_name(folder: &Path, name: &str) -> String {
    if !exists(&folder.join(name)) {
        return name.to_owned();
    }
    let (stem, extension) = match name.rfind('.').filter(|dot| *dot > 0) {
        Some(dot) => (&name[..dot], &name[dot..]),
        None => (name, ""),
    };
    for number in 1..10_000u32 {
        let suffix = if number == 1 {
            " copia".to_owned()
        } else {
            format!(" copia {number}")
        };
        let candidate = format!("{stem}{suffix}{extension}");
        if !exists(&folder.join(&candidate)) {
            return candidate;
        }
    }
    format!(
        "{stem} copia {}{extension}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    )
}

#[derive(Default)]
struct Budget {
    entries: usize,
    bytes: u64,
}

impl Budget {
    fn spend(&mut self, bytes: u64) -> Result<(), String> {
        self.entries += 1;
        self.bytes = self.bytes.saturating_add(bytes);
        if self.entries > MAX_ENTRIES || self.bytes > MAX_BYTES {
            Err("La copia supera el límite (20 000 elementos o 2 GiB)".into())
        } else {
            Ok(())
        }
    }
}

/// Copies a file or folder tree. Links and environment files inside it are left out.
fn copy_tree(source: &Path, target: &Path, budget: &mut Budget) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        budget.spend(0)?;
        fs::create_dir(target).map_err(|e| e.to_string())?;
        for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            if name.to_string_lossy().starts_with(".env") {
                continue;
            }
            copy_tree(&entry.path(), &target.join(&name), budget)?;
        }
        Ok(())
    } else {
        budget.spend(metadata.len())?;
        fs::copy(source, target)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn remove_tree(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

/// Where each source ends up: refuses a folder into itself.
fn checked_sources(
    root: &Path,
    sources: &[PathBuf],
    destination: &Path,
) -> Result<Vec<PathBuf>, String> {
    if sources.is_empty() {
        return Err("No hay nada que pegar".into());
    }
    let mut resolved = Vec::new();
    for source in sources {
        let path = existing(root, source)?;
        if path.is_dir() && (destination == path || destination.starts_with(&path)) {
            return Err("No se puede copiar o mover una carpeta dentro de sí misma".into());
        }
        resolved.push(path);
    }
    Ok(resolved)
}

/// Copies into `destination`; a name already there gets a free variant ("x copia").
pub fn copy_into(
    root: &Path,
    sources: &[PathBuf],
    destination: &Path,
) -> Result<Vec<PathBuf>, String> {
    let folder = directory(root, destination)?;
    let mut budget = Budget::default();
    let mut created = Vec::new();
    for source in checked_sources(root, sources, &folder)? {
        let name = source
            .file_name()
            .ok_or("El elemento no tiene nombre")?
            .to_string_lossy()
            .into_owned();
        let target = folder.join(unique_name(&folder, &name));
        copy_tree(&source, &target, &mut budget)?;
        created.push(relative_to(root, &target)?);
    }
    Ok(created)
}

/// Moves into `destination`. A name already there is an error: nothing is replaced.
pub fn move_into(
    root: &Path,
    sources: &[PathBuf],
    destination: &Path,
) -> Result<Vec<PathBuf>, String> {
    let folder = directory(root, destination)?;
    let resolved = checked_sources(root, sources, &folder)?;
    for source in &resolved {
        let name = source.file_name().ok_or("El elemento no tiene nombre")?;
        if source.parent() != Some(folder.as_path()) && exists(&folder.join(name)) {
            return Err(format!(
                "Ya existe `{}` en la carpeta de destino",
                name.to_string_lossy()
            ));
        }
    }
    let mut moved = Vec::new();
    for source in resolved {
        let name = source
            .file_name()
            .ok_or("El elemento no tiene nombre")?
            .to_owned();
        let target = folder.join(&name);
        if source != target {
            move_path(&source, &target)?;
        }
        moved.push(relative_to(root, &target)?);
    }
    Ok(moved)
}

/// A rename, or a copy and delete when the two places are on different disks.
fn move_path(source: &Path, target: &Path) -> Result<(), String> {
    if fs::rename(source, target).is_ok() {
        return Ok(());
    }
    copy_tree(source, target, &mut Budget::default())?;
    remove_tree(source).inspect_err(|_| {
        let _ = remove_tree(target);
    })
}

pub fn duplicate(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let parent = relative.parent().unwrap_or(Path::new("")).to_path_buf();
    copy_into(root, &[relative.to_path_buf()], &parent)?
        .into_iter()
        .next()
        .ok_or_else(|| "No se pudo duplicar".into())
}

fn trash_folder(root: &Path, home: &Path) -> Result<PathBuf, String> {
    if !home.is_absolute() {
        return Err("La carpeta de la papelera debe ser absoluta".into());
    }
    let canonical = root.canonicalize().map_err(|e| e.to_string())?;
    let folder = home.join("trash").join(format!(
        "{:x}",
        Sha256::digest(canonical.to_string_lossy().as_bytes())
    ));
    let mut walked = PathBuf::new();
    for part in folder.components() {
        walked.push(part);
        if fs::symlink_metadata(&walked).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("La ruta de la papelera contiene un enlace".into());
        }
    }
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder)
}

/// Deletes by moving the item to the application's trash, so it can be put back.
pub fn trash(root: &Path, home: &Path, relative: &Path) -> Result<Trashed, String> {
    let source = existing(root, relative)?;
    let name = source
        .file_name()
        .ok_or("El elemento no tiene nombre")?
        .to_string_lossy()
        .into_owned();
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let slot = trash_folder(root, home)?.join(format!("{stamp}"));
    fs::create_dir(&slot).map_err(|e| e.to_string())?;
    let location = slot.join(name);
    move_path(&source, &location).inspect_err(|_| {
        let _ = fs::remove_dir(&slot);
    })?;
    Ok(Trashed {
        original: relative.to_path_buf(),
        location,
    })
}

/// Puts a deleted item back where it was, unless something took its place.
pub fn restore(root: &Path, trashed: &Trashed) -> Result<PathBuf, String> {
    protected(&trashed.original)?;
    let parent = trashed.original.parent().unwrap_or(Path::new(""));
    let folder = directory(root, parent)
        .map_err(|_| "La carpeta original ya no existe; no se pudo restaurar".to_owned())?;
    let name = trashed
        .original
        .file_name()
        .ok_or("Ruta original inválida")?;
    let target = folder.join(name);
    if exists(&target) {
        return Err(format!(
            "Ya existe `{}`; no se restauró",
            name.to_string_lossy()
        ));
    }
    if !exists(&trashed.location) {
        return Err("El elemento ya no está en la papelera".into());
    }
    move_path(&trashed.location, &target)?;
    if let Some(slot) = trashed.location.parent() {
        let _ = fs::remove_dir(slot);
    }
    relative_to(root, &target)
}

#[cfg(test)]
#[path = "project_fs_tests.rs"]
mod tests;
