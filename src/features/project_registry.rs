use crate::features::project_detection::ProjectDetection;
use crate::features::project_profiles::application_home;
use serde_json::{Map, Value, json};
use sha1::{Digest, Sha1};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Clone)]
pub struct RegistryUpdate {
    pub path: PathBuf,
    pub changed: bool,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
}

pub struct RegistryWriteResult {
    pub changed: bool,
    pub backup: Option<PathBuf>,
}

pub fn plan_registration(
    root: &str,
    name: &str,
    detection: &ProjectDetection,
) -> Result<RegistryUpdate, String> {
    let home =
        application_home().ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME".to_owned())?;
    let path = home.join("projects.json");
    if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("projects.json es un enlace simbólico; no se modificó".to_owned());
    }
    let before = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("No se pudo leer el registro de proyectos: {error}")),
    };
    let mut document = match &before {
        Some(bytes) => serde_json::from_slice::<Value>(bytes).map_err(|_| {
            "projects.json contiene JSON inválido; se preservó sin cambios".to_owned()
        })?,
        None => json!({ "projects": [] }),
    };
    let object = document
        .as_object_mut()
        .ok_or_else(|| "projects.json debe contener un objeto".to_owned())?;
    let projects = object
        .entry("projects")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| "projects.json: projects debe ser una lista".to_owned())?;
    let current_index = projects
        .iter()
        .position(|project| project.get("path").and_then(Value::as_str) == Some(root));
    let alias = project_alias(root, name, projects, current_index);
    let mut payload = Map::new();
    payload.insert("alias".to_owned(), Value::String(alias));
    payload.insert("name".to_owned(), Value::String(name.to_owned()));
    payload.insert("path".to_owned(), Value::String(root.to_owned()));
    payload.insert(
        "profile".to_owned(),
        Value::String(detection.profile.clone()),
    );
    payload.insert(
        "stacks".to_owned(),
        serde_json::to_value(&detection.stacks)
            .map_err(|error| format!("No se pudo serializar los stacks: {error}"))?,
    );
    let changed = match current_index {
        Some(index) => {
            let current = projects[index]
                .as_object_mut()
                .ok_or_else(|| "Entrada de proyecto inválida en projects.json".to_owned())?;
            let changed = payload
                .iter()
                .any(|(key, value)| current.get(key) != Some(value));
            if changed {
                current.extend(payload);
                current.insert("updated_at".to_owned(), Value::String(timestamp()?));
            }
            changed
        }
        None => {
            payload.insert("updated_at".to_owned(), Value::String(timestamp()?));
            projects.push(Value::Object(payload));
            true
        }
    };
    let after = if changed {
        let mut bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("No se pudo renderizar projects.json: {error}"))?;
        bytes.push(b'\n');
        Some(bytes)
    } else {
        None
    };
    Ok(RegistryUpdate {
        path,
        changed,
        before,
        after,
    })
}

pub fn apply_registration(update: &RegistryUpdate) -> Result<RegistryWriteResult, String> {
    if !update.changed {
        return Ok(RegistryWriteResult {
            changed: false,
            backup: None,
        });
    }
    check_registration_unchanged(update)?;
    let after = update
        .after
        .as_ref()
        .ok_or_else(|| "Falta el contenido actualizado del registro".to_owned())?;
    let backup = match &update.before {
        Some(bytes) => Some(backup_registry(&update.path, bytes)?),
        None => None,
    };
    if let Err(error) = replace_file(&update.path, after) {
        if let Some(bytes) = &update.before {
            let _ = replace_file(&update.path, bytes);
        } else {
            let _ = fs::remove_file(&update.path);
        }
        return Err(error);
    }
    Ok(RegistryWriteResult {
        changed: true,
        backup,
    })
}

pub fn check_registration_unchanged(update: &RegistryUpdate) -> Result<(), String> {
    if !update.changed {
        return Ok(());
    }
    if fs::symlink_metadata(&update.path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(
            "projects.json se convirtió en enlace simbólico; vuelve a previsualizar".to_owned(),
        );
    }
    let current = match fs::read(&update.path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("No se pudo releer el registro: {error}")),
    };
    if current != update.before {
        return Err(
            "El registro de proyectos cambió después de la vista previa; vuelve a previsualizar"
                .to_owned(),
        );
    }
    Ok(())
}

pub fn restore_registry(update: &RegistryUpdate) -> Result<(), String> {
    if !update.changed {
        return Ok(());
    }
    let current = fs::read(&update.path).ok();
    if current == update.before {
        return Ok(());
    }
    if current != update.after {
        return Err("No se restauró projects.json porque cambió después de aplicarse".to_owned());
    }
    match &update.before {
        Some(bytes) => replace_file(&update.path, bytes),
        None => match fs::remove_file(&update.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("No se pudo revertir projects.json: {error}")),
        },
    }
}

fn project_alias(root: &str, name: &str, projects: &[Value], current: Option<usize>) -> String {
    if let Some(alias) = current
        .and_then(|index| projects.get(index))
        .and_then(|project| project.get("alias"))
        .and_then(Value::as_str)
        .filter(|alias| !alias.trim().is_empty())
    {
        return alias.trim().to_owned();
    }
    let candidate = slug(name);
    let occupied = projects.iter().enumerate().any(|(index, project)| {
        Some(index) != current
            && project.get("alias").and_then(Value::as_str) == Some(candidate.as_str())
    });
    if !occupied {
        return candidate;
    }
    format!("{}-{}", candidate, sha1_prefix(root, 6))
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in value.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            if separator && !result.is_empty() {
                result.push('-');
            }
            separator = false;
            result.push(character);
        } else {
            separator = true;
        }
    }
    let value = result.trim_matches('-');
    if value.is_empty() {
        "project".to_owned()
    } else {
        value.to_owned()
    }
}

fn sha1_prefix(value: &str, digits: usize) -> String {
    let hex = Sha1::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    hex[..digits.min(hex.len())].to_owned()
}

fn timestamp() -> Result<String, String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("No se pudo generar marca de tiempo UTC: {error}"))
}

fn backup_registry(path: &std::path::Path, contents: &[u8]) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta del registro".to_owned())?;
    let target = parent.join("backups").join("global").join(format!(
        "{}-projects",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let parent = target
        .parent()
        .ok_or_else(|| "No se pudo resolver el destino del backup".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear el backup del registro: {error}"))?;
    fs::create_dir(&target)
        .map_err(|error| format!("No se pudo crear el backup del registro: {error}"))?;
    let backup = target.join("projects.json");
    fs::write(&backup, contents)
        .map_err(|error| format!("No se pudo guardar el backup de projects.json: {error}"))?;
    Ok(backup)
}

fn replace_file(path: &std::path::Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta del registro".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear la carpeta del registro: {error}"))?;
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("No se reemplazó projects.json porque es un enlace simbólico".to_owned());
    }
    let temp = parent.join(format!(
        ".projects-{}-{}.tmp",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("No se pudo preparar el registro temporal: {error}"))?;
    if let Err(error) = file.write_all(contents).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudo escribir el registro temporal: {error}"));
    }
    drop(file);
    if path.exists()
        && let Err(error) = fs::remove_file(path)
    {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudo reemplazar projects.json: {error}"));
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudo guardar projects.json: {error}"));
    }
    Ok(())
}
