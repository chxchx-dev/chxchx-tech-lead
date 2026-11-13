//! Studio preferences and per-project session, persisted as JSON in the ChxChx home.
//!
//! Mirrors what the Qt Studio kept in QSettings (recent projects, recent files, clean
//! editor session, layout) without importing its registry. Nothing here grants trust,
//! stores secrets or restores terminals/agents.
use crate::features::project_profiles::application_home;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const FILE_NAME: &str = "studio-preferences.json";
const VERSION: u32 = 1;
pub const MAX_RECENT_PROJECTS: usize = 10;
pub const MAX_RECENT_FILES: usize = 20;
pub const MAX_TABS: usize = 24;
pub const MAX_EXPANDED: usize = 64;
const MAX_PROJECTS: usize = 200;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub explorer: bool,
    /// Sidebar view: `explorer`, `search` or `workspace`.
    pub side: String,
    /// `terminal`, `activity` or `None` when the bottom dock is hidden.
    pub dock: Option<String>,
    pub explorer_width: f32,
    pub dock_height: f32,
    /// Last window size in logical pixels.
    pub window: Option<[f32; 2]>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            explorer: true,
            side: "explorer".to_owned(),
            dock: None,
            explorer_width: 248.0,
            dock_height: 220.0,
            window: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSession {
    pub tabs: Vec<String>,
    pub active: Option<String>,
    pub expanded: Vec<String>,
    pub recent_files: Vec<String>,
    /// Last visited section of the Home tab.
    pub page: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub version: u32,
    pub layout: Layout,
    pub recent_projects: Vec<String>,
    pub projects: BTreeMap<String, ProjectSession>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: VERSION,
            layout: Layout::default(),
            recent_projects: Vec::new(),
            projects: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoadedPreferences {
    pub preferences: Preferences,
    pub warning: Option<String>,
    /// False when the file belongs to a newer product; it must not be overwritten.
    pub writable: bool,
}

/// Stable key of a project: SHA-256 of its canonical root, as in the Qt Studio.
pub fn project_key(root: &Path) -> String {
    Sha256::digest(root.display().to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A relative path that stays inside the project and is safe to store and reopen.
pub fn is_safe_relative(value: &str) -> bool {
    if value.is_empty() || value.len() > 1024 || value.contains('\0') || value.contains(':') {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn clean_paths(values: &mut Vec<String>, limit: usize) {
    let mut seen = std::collections::BTreeSet::new();
    values.retain(|value| is_safe_relative(value) && seen.insert(value.clone()));
    values.truncate(limit);
}

impl Preferences {
    /// Clamps every value so a hand-edited or damaged file cannot break the layout.
    pub fn sanitize(&mut self) {
        self.version = VERSION;
        let layout = &mut self.layout;
        layout.explorer_width = clamp(layout.explorer_width, 180.0, 520.0, 248.0);
        layout.dock_height = clamp(layout.dock_height, 110.0, 560.0, 220.0);
        if !matches!(
            layout.side.as_str(),
            "explorer" | "search" | "workspace" | "git"
        ) {
            layout.side = "explorer".to_owned();
        }
        if !matches!(layout.dock.as_deref(), Some("terminal" | "activity")) {
            layout.dock = None;
        }
        layout.window = layout.window.and_then(|[width, height]| {
            (width.is_finite() && height.is_finite())
                .then(|| [width.clamp(880.0, 3840.0), height.clamp(520.0, 2160.0)])
        });
        self.recent_projects
            .retain(|path| !path.is_empty() && path.len() <= 1024);
        self.recent_projects.dedup();
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
        while self.projects.len() > MAX_PROJECTS {
            self.projects.pop_first();
        }
        for session in self.projects.values_mut() {
            clean_paths(&mut session.tabs, MAX_TABS);
            clean_paths(&mut session.expanded, MAX_EXPANDED);
            clean_paths(&mut session.recent_files, MAX_RECENT_FILES);
            if session
                .active
                .as_deref()
                .is_some_and(|path| !is_safe_relative(path))
            {
                session.active = None;
            }
            if !matches!(
                session.page.as_str(),
                "home" | "setup" | "projects" | "catalog" | "integrations" | "diagnostics"
            ) {
                session.page = "home".to_owned();
            }
        }
    }

    pub fn session(&self, root: &Path) -> ProjectSession {
        self.projects
            .get(&project_key(root))
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_session(&mut self, root: &Path, session: ProjectSession) {
        self.projects.insert(project_key(root), session);
    }

    /// Moves `root` to the front of the recent projects list.
    pub fn touch_recent_project(&mut self, root: &Path) {
        let value = root.display().to_string();
        self.recent_projects.retain(|item| *item != value);
        self.recent_projects.insert(0, value);
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }
}

fn clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

/// Moves `path` to the front of a recent list, keeping it unique and bounded.
pub fn push_recent(list: &mut Vec<String>, path: &str, limit: usize) {
    list.retain(|item| item != path);
    list.insert(0, path.to_owned());
    list.truncate(limit);
}

pub fn load() -> LoadedPreferences {
    match application_home() {
        Some(home) => load_from(&home),
        None => LoadedPreferences {
            preferences: Preferences::default(),
            warning: Some(
                "No se pudo resolver CHXCHX_TECH_HOME; no se guardarán preferencias".into(),
            ),
            writable: false,
        },
    }
}

pub fn save(preferences: &Preferences) -> Result<(), String> {
    let home = application_home().ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    save_to(&home, preferences)
}

pub fn load_from(home: &Path) -> LoadedPreferences {
    let path = home.join(FILE_NAME);
    let fresh = |warning: Option<String>, writable: bool| LoadedPreferences {
        preferences: Preferences::default(),
        warning,
        writable,
    };
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return fresh(
                Some(format!("{FILE_NAME} es un enlace simbólico; se ignoró")),
                false,
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return fresh(None, true),
        Err(error) => return fresh(Some(format!("No se pudo leer {FILE_NAME}: {error}")), false),
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => return fresh(Some(format!("No se pudo leer {FILE_NAME}: {error}")), false),
    };
    match serde_json::from_slice::<Preferences>(&bytes) {
        Ok(mut preferences) if preferences.version <= VERSION => {
            preferences.sanitize();
            LoadedPreferences {
                preferences,
                warning: None,
                writable: true,
            }
        }
        Ok(_) => fresh(
            Some(format!(
                "{FILE_NAME} es de una versión más nueva; se conservó sin cambios"
            )),
            false,
        ),
        Err(_) => {
            // Keep the unreadable file for inspection instead of silently overwriting it.
            let quarantine = path.with_extension("json.corrupt");
            let kept = fs::rename(&path, &quarantine).is_ok();
            fresh(
                Some(if kept {
                    format!(
                        "{FILE_NAME} estaba dañado; se guardó como {}",
                        quarantine.display()
                    )
                } else {
                    format!("{FILE_NAME} está dañado y no se pudo apartar")
                }),
                kept,
            )
        }
    }
}

pub fn save_to(home: &Path, preferences: &Preferences) -> Result<(), String> {
    let mut preferences = preferences.clone();
    preferences.sanitize();
    let json = serde_json::to_vec_pretty(&preferences)
        .map_err(|error| format!("No se pudieron serializar las preferencias: {error}"))?;
    fs::create_dir_all(home)
        .map_err(|error| format!("No se pudo crear {}: {error}", home.display()))?;
    let target = home.join(FILE_NAME);
    if fs::symlink_metadata(&target).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(format!(
            "{FILE_NAME} es un enlace simbólico; no se modificó"
        ));
    }
    let temp: PathBuf = home.join(format!(".{FILE_NAME}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(&json)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, &target)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudieron guardar las preferencias: {error}"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "studio_preferences_tests.rs"]
mod tests;
