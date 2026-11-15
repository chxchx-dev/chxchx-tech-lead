use crate::features::project_config::validate_project_config;
use crate::features::project_detection::{ProjectDetection, detect_project};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_RELATIVE_PATH: &str = ".ai/chxchx-tech.toml";
const TRUST_FILE: &str = "trusted_projects.json";

#[derive(Clone, Debug, Serialize)]
pub struct ProjectOverview {
    pub schema: &'static str,
    pub schema_version: u32,
    pub project: ProjectSummary,
    pub detection: ProjectDetection,
    pub workspace: WorkspaceSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectSummary {
    pub name: String,
    pub root: String,
    pub git_repository: bool,
    pub has_ai_directory: bool,
    pub has_agents_guide: bool,
    pub has_claude_guide: bool,
    pub detected_markers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceSummary {
    pub config_path: String,
    pub config_exists: bool,
    pub config_parseable: Option<bool>,
    pub config_version: Option<i64>,
    pub config_supported: Option<bool>,
    pub config_validation_error: Option<String>,
    pub trusted: bool,
}

pub fn inspect_project(path: &Path) -> Result<ProjectOverview, String> {
    let canonical_root = fs::canonicalize(path).map_err(|error| {
        format!(
            "No se pudo resolver la ruta de proyecto {}: {error}",
            path.display()
        )
    })?;
    let root = normalize_canonical_path(canonical_root);
    if !root.is_dir() {
        return Err(format!(
            "La ruta de proyecto no es un directorio: {}",
            root.display()
        ));
    }

    let config_path = root.join(CONFIG_RELATIVE_PATH);
    let (config_parseable, config_version, config_supported, config_validation_error) =
        inspect_config(&config_path, &root);
    let project = ProjectSummary {
        name: project_name(&root),
        root: root.display().to_string(),
        git_repository: root.join(".git").exists(),
        has_ai_directory: root.join(".ai").is_dir(),
        has_agents_guide: root.join("AGENTS.md").is_file(),
        has_claude_guide: root.join("CLAUDE.md").is_file(),
        detected_markers: detect_markers(&root),
    };
    let detection = detect_project(&root);
    let trusted = is_trusted(&root, &config_path);

    Ok(ProjectOverview {
        schema: "chxchx.project-overview",
        schema_version: 1,
        project,
        detection,
        workspace: WorkspaceSummary {
            config_path: config_path.display().to_string(),
            config_exists: config_path.is_file(),
            config_parseable,
            config_version,
            config_supported,
            config_validation_error,
            trusted,
        },
    })
}

fn project_name(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| root.display().to_string())
}

pub(crate) fn normalize_canonical_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        if let Some(unc_path) = value.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc_path}"));
        }
        if let Some(drive_path) = value.strip_prefix(r"\\?\") {
            return PathBuf::from(drive_path);
        }
    }
    path
}

fn detect_markers(root: &Path) -> Vec<String> {
    let markers: [(&str, &str); 7] = [
        ("pyproject.toml", "Python"),
        ("requirements.txt", "Python"),
        ("package.json", "Node.js"),
        ("Cargo.toml", "Rust"),
        ("go.mod", "Go"),
        ("CMakeLists.txt", "CMake"),
        (".github/workflows", "GitHub Actions"),
    ];
    let mut detected = BTreeMap::new();
    for (path, marker) in markers {
        if root.join(path).exists() {
            detected.insert(marker, ());
        }
    }
    detected.into_keys().map(str::to_owned).collect()
}

fn inspect_config(
    path: &Path,
    root: &Path,
) -> (Option<bool>, Option<i64>, Option<bool>, Option<String>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return (None, None, None, None);
    };
    let Ok(document) = toml::from_str::<toml::Value>(&contents) else {
        return (
            Some(false),
            None,
            Some(false),
            Some("TOML mal formado".to_owned()),
        );
    };
    let validation = validate_project_config(&document, root);
    (
        Some(true),
        document.get("version").and_then(toml::Value::as_integer),
        Some(validation.supported),
        validation.error,
    )
}

fn is_trusted(root: &Path, config_path: &Path) -> bool {
    let Ok(home) = application_home() else {
        return false;
    };
    let Ok(contents) = fs::read(home.join(TRUST_FILE)) else {
        return false;
    };
    let Ok(trusted_projects) = serde_json::from_slice::<serde_json::Value>(&contents) else {
        return false;
    };
    let Some(expected_digest) = trusted_projects
        .as_object()
        .and_then(|entries| entries.get(&root.display().to_string()))
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    digest_config(config_path).as_deref() == Some(expected_digest)
}

fn digest_config(path: &Path) -> Option<String> {
    let config = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => b"missing-config".to_vec(),
        Err(_) => return None,
    };
    Some(format!("{:x}", Sha256::digest(config)))
}

fn application_home() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("CHXCHX_TECH_HOME") {
        return Ok(expand_home(PathBuf::from(path)));
    }
    std::env::home_dir()
        .map(|path| path.join(".chxchx-tech-lead"))
        .ok_or_else(|| "No se pudo resolver el directorio de usuario".to_owned())
}

fn expand_home(path: PathBuf) -> PathBuf {
    if path == Path::new("~") {
        return std::env::home_dir().unwrap_or(path);
    }
    let value = path.to_string_lossy();
    if let Some(rest) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
        && let Some(home) = std::env::home_dir()
    {
        return home.join(rest);
    }
    path
}
