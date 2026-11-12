use crate::features::project_overview::inspect_project;
use crate::features::project_profiles::application_home;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

const TRUST_FILE: &str = "trusted_projects.json";
const CONFIG_FILE: &str = ".ai/chxchx-tech.toml";

#[derive(Clone)]
pub struct ProjectTrustPlan {
    pub project_root: String,
    pub desired_trust: bool,
    pub currently_trusted: bool,
    pub changes_state: bool,
    pub actions: Vec<String>,
    pub blocked_reason: Option<String>,
    path: PathBuf,
    root: PathBuf,
    config_digest: String,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
}

pub fn plan_project_trust(path: &Path, desired_trust: bool) -> Result<ProjectTrustPlan, String> {
    let overview = inspect_project(path)?;
    let root = PathBuf::from(overview.project.root);
    let config_digest = config_digest(&root)?;
    let home =
        application_home().ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME".to_owned())?;
    let trust_path = home.join(TRUST_FILE);
    if fs::symlink_metadata(&trust_path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("trusted_projects.json es un enlace simbólico; no se modificó".to_owned());
    }
    let before = match fs::read(&trust_path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("No se pudo leer el registro de confianza: {error}")),
    };
    let mut document = match &before {
        Some(bytes) => serde_json::from_slice::<Value>(bytes)
            .map_err(|_| "trusted_projects.json contiene JSON inválido; se preservó".to_owned())?,
        None => Value::Object(Map::new()),
    };
    let entries = document
        .as_object_mut()
        .ok_or_else(|| "trusted_projects.json debe contener un objeto JSON".to_owned())?;
    let project_key = root.display().to_string();
    let saved_digest = entries.get(&project_key).and_then(Value::as_str);
    let currently_trusted = saved_digest == Some(config_digest.as_str());
    let changes_state = if desired_trust {
        !currently_trusted
    } else {
        saved_digest.is_some()
    };
    let after = if changes_state {
        if desired_trust {
            entries.insert(project_key, Value::String(config_digest.clone()));
        } else {
            entries.remove(&project_key);
        }
        let mut bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("No se pudo serializar el registro de confianza: {error}"))?;
        bytes.push(b'\n');
        Some(bytes)
    } else {
        None
    };
    let action = match (desired_trust, changes_state) {
        (true, true) => "marcar el proyecto como confiable para esta versión de configuración",
        (true, false) => "el proyecto ya está confiable para la configuración actual",
        (false, true) => "quitar la confianza local del proyecto",
        (false, false) => "el proyecto ya no tiene confianza local",
    };
    let mut actions = vec![action.to_owned()];
    if changes_state && before.is_some() {
        actions.push("guardar backup de trusted_projects.json".to_owned());
    }
    Ok(ProjectTrustPlan {
        project_root: root.display().to_string(),
        desired_trust,
        currently_trusted,
        changes_state,
        actions,
        blocked_reason: None,
        path: trust_path,
        root,
        config_digest,
        before,
        after,
    })
}

pub fn apply_project_trust(plan: &ProjectTrustPlan) -> Result<Option<String>, String> {
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("Cambio de confianza bloqueado: {reason}"));
    }
    if !plan.changes_state {
        return Ok(None);
    }
    if config_digest(&plan.root)? != plan.config_digest {
        return Err(
            "La configuración cambió desde la vista previa; vuelve a previsualizar".to_owned(),
        );
    }
    if fs::symlink_metadata(&plan.path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("trusted_projects.json se convirtió en enlace simbólico".to_owned());
    }
    let current = match fs::read(&plan.path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "No se pudo releer el registro de confianza: {error}"
            ));
        }
    };
    if current != plan.before {
        return Err("El registro de confianza cambió desde la vista previa".to_owned());
    }
    let after = plan
        .after
        .as_deref()
        .ok_or_else(|| "Falta el nuevo contenido del registro de confianza".to_owned())?;
    let backup = match &plan.before {
        Some(bytes) => Some(backup_trust(&plan.path, bytes)?),
        None => None,
    };
    if let Err(error) = replace_file(&plan.path, after) {
        let recovery = match &plan.before {
            Some(bytes) => replace_file(&plan.path, bytes),
            None => remove_file_if_present(&plan.path),
        };
        return match recovery {
            Ok(()) => Err(error),
            Err(recovery_error) => Err(format!(
                "No se pudo guardar la confianza: {error}; tampoco restaurar el registro anterior: {recovery_error}"
            )),
        };
    }
    Ok(backup.map(|path| path.display().to_string()))
}

fn config_digest(root: &Path) -> Result<String, String> {
    let ai_directory = root.join(".ai");
    if fs::symlink_metadata(&ai_directory).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(".ai es un enlace simbólico; no se registra confianza".to_owned());
    }
    if ai_directory.exists() && !ai_directory.is_dir() {
        return Err(".ai existe y no es un directorio".to_owned());
    }
    let path = root.join(CONFIG_FILE);
    let bytes = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(
                "La configuración es un enlace simbólico; no se registra confianza".to_owned(),
            );
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err("La ruta de configuración no es un archivo regular".to_owned());
        }
        Ok(_) => fs::read(&path)
            .map_err(|error| format!("No se pudo leer la configuración para trust: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => b"missing-config".to_vec(),
        Err(error) => return Err(format!("No se pudo inspeccionar la configuración: {error}")),
    };
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn backup_trust(path: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver el directorio de confianza".to_owned())?;
    let target = parent.join("backups").join("global").join(format!(
        "{}-trust",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let target_parent = target
        .parent()
        .ok_or_else(|| "No se pudo resolver el destino del backup".to_owned())?;
    fs::create_dir_all(target_parent)
        .map_err(|error| format!("No se pudo crear el directorio de backup: {error}"))?;
    fs::create_dir(&target)
        .map_err(|error| format!("No se pudo reservar el backup de confianza: {error}"))?;
    let backup = target.join(TRUST_FILE);
    if let Err(error) = fs::write(&backup, bytes) {
        let _ = fs::remove_dir_all(&target);
        return Err(format!(
            "No se pudo respaldar el registro de confianza: {error}"
        ));
    }
    Ok(backup)
}

fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver el directorio de confianza".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear el directorio de confianza: {error}"))?;
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("No se reemplaza un registro de confianza enlazado".to_owned());
    }
    let temp = parent.join(format!(
        ".trusted-projects-{}-{}.tmp",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("No se pudo preparar el registro de confianza: {error}"))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudo escribir el registro temporal: {error}"));
    }
    drop(file);
    if path.exists()
        && let Err(error) = fs::remove_file(path)
    {
        let _ = fs::remove_file(&temp);
        return Err(format!(
            "No se pudo reemplazar el registro de confianza: {error}"
        ));
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(format!(
            "No se pudo guardar el registro de confianza: {error}"
        ));
    }
    Ok(())
}

fn remove_file_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "No se pudo limpiar el registro de confianza: {error}"
        )),
    }
}
