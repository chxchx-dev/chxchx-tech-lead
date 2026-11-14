use crate::features::project_profiles::application_home;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct RegisteredProject {
    pub alias: String,
    pub name: String,
    pub profile: String,
    pub path: Option<String>,
    pub stacks: Vec<String>,
    pub available: bool,
    pub is_current: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CurrentProject {
    pub alias: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectCatalog {
    pub projects: Vec<RegisteredProject>,
    pub current: Option<CurrentProject>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectRegistrySnapshot {
    pub path: PathBuf,
    pub contents: Option<Vec<u8>>,
}

pub(crate) fn read_project_registry_snapshot() -> Result<ProjectRegistrySnapshot, String> {
    let home =
        application_home().ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME".to_owned())?;
    let path = home.join("projects.json");
    if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err("projects.json es un enlace simbólico; no se leyó".to_owned());
    }
    let contents = match fs::read(&path) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("No se pudo leer el registro de proyectos: {error}")),
    };
    Ok(ProjectRegistrySnapshot { path, contents })
}

pub fn inspect_registered_projects() -> Result<ProjectCatalog, String> {
    let snapshot = read_project_registry_snapshot()?;
    let contents = match snapshot.contents.as_deref() {
        Some(contents) => contents,
        None => {
            return Ok(ProjectCatalog {
                projects: Vec::new(),
                current: None,
            });
        }
    };
    let document = serde_json::from_slice::<Value>(contents)
        .map_err(|_| "projects.json contiene JSON inválido; se preservó sin cambios".to_owned())?;
    let object = document
        .as_object()
        .ok_or_else(|| "projects.json debe contener un objeto".to_owned())?;
    let entries = object
        .get("projects")
        .map(|projects| {
            projects
                .as_array()
                .ok_or_else(|| "projects.json: projects debe ser una lista".to_owned())
        })
        .transpose()?
        .cloned()
        .unwrap_or_default();
    let current_path = object
        .get("last_project")
        .and_then(Value::as_str)
        .and_then(existing_directory)
        .map(|(_, canonical)| canonical);

    let projects = entries
        .iter()
        .filter_map(Value::as_object)
        .map(|entry| {
            let name = string_field(entry.get("name")).unwrap_or_default();
            let alias = string_field(entry.get("alias"))
                .filter(|alias| !alias.trim().is_empty())
                .unwrap_or_else(|| name.clone());
            let path = string_field(entry.get("path"));
            let canonical = path
                .as_deref()
                .and_then(existing_directory)
                .map(|(_, canonical)| canonical);
            RegisteredProject {
                alias,
                name,
                profile: string_field(entry.get("profile")).unwrap_or_else(|| "generic".to_owned()),
                path,
                stacks: entry
                    .get("stacks")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                available: canonical.is_some(),
                is_current: canonical
                    .as_ref()
                    .zip(current_path.as_ref())
                    .is_some_and(|(project, current)| paths_equal(project, current)),
            }
        })
        .collect::<Vec<_>>();

    let current = object
        .get("last_project")
        .and_then(Value::as_str)
        .and_then(existing_directory)
        .map(|(raw, canonical)| {
            let alias = projects
                .iter()
                .find(|project| {
                    project.path.as_deref().is_some_and(|path| {
                        Path::new(path)
                            .canonicalize()
                            .is_ok_and(|project_path| paths_equal(&project_path, &canonical))
                    })
                })
                .map(|project| project.alias.clone())
                .unwrap_or_else(|| {
                    Path::new(&raw)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| raw.clone())
                });
            CurrentProject {
                alias,
                path: canonical.display().to_string(),
            }
        });

    Ok(ProjectCatalog { projects, current })
}

fn string_field(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_owned)
}

fn existing_directory(raw: &str) -> Option<(String, PathBuf)> {
    let path = Path::new(raw);
    if !path.is_dir() {
        return None;
    }
    path.canonicalize()
        .ok()
        .map(|canonical| (raw.to_owned(), canonical))
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
