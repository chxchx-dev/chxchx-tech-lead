//! Start and stop a project's configured auto-start processes and record its state.
use crate::features::project_overview::inspect_project;
use crate::features::workspace_processes::{self, Action};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct WorkspaceAction {
    pub status: String,
    pub processes: Vec<Action>,
    pub message: String,
}

pub fn start(root: &Path, dry_run: bool) -> Result<WorkspaceAction, String> {
    let overview = inspect_project(root)?;
    if !overview.workspace.trusted {
        return Err(
            "El proyecto no es confiable; ejecuta `workspace trust --apply` antes de iniciar comandos"
                .to_owned(),
        );
    }
    if overview.workspace.config_exists && overview.workspace.config_supported == Some(false) {
        return Err(overview
            .workspace
            .config_validation_error
            .unwrap_or_else(|| "La configuración del workspace no es válida".to_owned()));
    }
    let specs = workspace_processes::configured(root)?;
    let config = read_workspace_config(root)?;
    let session = session_name(root, &config);
    let workspace_auto_start = config
        .get("auto_start")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut processes = Vec::new();
    if workspace_auto_start {
        for spec in specs.iter().filter(|spec| spec.auto_start) {
            match workspace_processes::start(root, &spec.id, dry_run) {
                Ok(action) => processes.push(action),
                Err(error) => {
                    workspace_processes::set_workspace_status(
                        root,
                        "ERROR",
                        Some(&session),
                        dry_run,
                    )?;
                    return Err(error);
                }
            }
        }
    }
    workspace_processes::set_workspace_status(root, "ACTIVE", Some(&session), dry_run)?;
    Ok(WorkspaceAction {
        status: if dry_run {
            "ACTIVE (preview)"
        } else {
            "ACTIVE"
        }
        .to_owned(),
        processes,
        message: if dry_run {
            format!("DRY RUN: iniciar workspace `{session}`")
        } else {
            format!("Workspace activo: `{session}`")
        },
    })
}

pub fn stop(root: &Path, dry_run: bool) -> Result<WorkspaceAction, String> {
    let overview = inspect_project(root)?;
    if overview.workspace.config_exists && overview.workspace.config_supported == Some(false) {
        return Err(overview
            .workspace
            .config_validation_error
            .unwrap_or_else(|| "La configuración del workspace no es válida".to_owned()));
    }
    let mut processes = Vec::new();
    for process in workspace_processes::list_readonly(root)?
        .into_iter()
        .filter(|process| process.status == workspace_processes::Status::Running)
    {
        processes.push(workspace_processes::stop(root, &process.id, dry_run)?);
    }
    workspace_processes::set_workspace_status(root, "STOPPED", None, dry_run)?;
    Ok(WorkspaceAction {
        status: if dry_run {
            "STOPPED (preview)"
        } else {
            "STOPPED"
        }
        .to_owned(),
        processes,
        message: if dry_run {
            "DRY RUN: detener workspace".to_owned()
        } else {
            "Workspace detenido".to_owned()
        },
    })
}

pub fn suspend(root: &Path, dry_run: bool) -> Result<WorkspaceAction, String> {
    let mut action = stop(root, dry_run)?;
    workspace_processes::set_workspace_status(root, "SUSPENDED", None, dry_run)?;
    action.status = if dry_run {
        "SUSPENDED (preview)".to_owned()
    } else {
        "SUSPENDED".to_owned()
    };
    action.message = if dry_run {
        "DRY RUN: suspender workspace".to_owned()
    } else {
        "Workspace suspendido".to_owned()
    };
    Ok(action)
}

pub fn resume(root: &Path, dry_run: bool) -> Result<WorkspaceAction, String> {
    let mut action = start(root, dry_run)?;
    action.message = if dry_run {
        "DRY RUN: reanudar workspace".to_owned()
    } else {
        "Workspace reanudado".to_owned()
    };
    Ok(action)
}

fn read_workspace_config(root: &Path) -> Result<Value, String> {
    let path = root.join(".ai/chxchx-tech.toml");
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Value::Null),
        Err(error) => return Err(format!("No pude leer la configuración: {error}")),
    };
    let document: toml::Value =
        toml::from_str(&contents).map_err(|error| format!("TOML inválido: {error}"))?;
    match document.get("workspace") {
        Some(workspace) => serde_json::to_value(workspace).map_err(|error| error.to_string()),
        None => Ok(serde_json::json!({})),
    }
}

fn session_name(root: &Path, config: &Value) -> String {
    let raw = config
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| root.file_name().and_then(|name| name.to_str()))
        .unwrap_or("workspace");
    let mut result = String::new();
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-') {
            result.push(character);
        } else if !result.ends_with('-') {
            result.push('-');
        }
    }
    let result = result
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>();
    if result.is_empty() {
        "workspace".to_owned()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_names_are_stable_and_safe_for_workspace_state() {
        let root = Path::new("/tmp/demo-project");
        let config = serde_json::json!({"name": "My project / API"});
        assert_eq!(session_name(root, &config), "My-project-API");
        assert_eq!(session_name(root, &Value::Null), "demo-project");
    }
}
