//! How this installation reaches the world, chosen by the person (ROADMAP: "que las personas metan
//! sus propias cosas"): the node's port, the HTTPS port Tailscale uses by default, where updates come
//! from and which signing key they must carry, and where each tool lives when it is not on the PATH.
//! Everything has the product's own value as the starting point and nothing is required: an empty or
//! missing file means the defaults. It is kept in the application's folder, written through a
//! temporary file with a `.bak` of the previous one.
//!
//! Nothing here is a secret: the key is the *public* half of the signing pair.

use crate::features::project_profiles::application_home;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const FOLDER: &str = "connection";
const FILE: &str = "connection.json";

pub const DEFAULT_NODE_PORT: u16 = 47_821;
pub const DEFAULT_HTTPS_PORT: u16 = 443;
pub const DEFAULT_OWNER: &str = "chxchx-dev";
pub const DEFAULT_REPOSITORY: &str = "chxchx-tech-lead";
/// The tools whose location can be chosen.
pub const TOOLS: &[&str] = &["claude", "codex", "opencode", "python", "tailscale"];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    /// Port of the node on this PC; `None` is [`DEFAULT_NODE_PORT`].
    pub node_port: Option<u16>,
    /// HTTPS port for `tailscale serve` when none is given; `None` is 443.
    pub https_port: Option<u16>,
    /// Who publishes the releases (GitHub owner and repository); `None` is the product's.
    pub update_owner: Option<String>,
    pub update_repository: Option<String>,
    /// The public signing key updates must carry, 64 hexadecimal digits; `None` is the built-in one.
    pub update_key: Option<String>,
    /// Absolute path of a tool, by name (see [`TOOLS`]).
    pub tools: BTreeMap<String, String>,
}

fn github_name(value: &str) -> Option<String> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 100
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && !value.starts_with('.');
    valid.then(|| value.to_owned())
}

fn port(value: u16) -> Option<u16> {
    (value >= 1024).then_some(value)
}

impl Connection {
    /// The same settings with anything invalid dropped (so a hand-edited file cannot do harm).
    pub fn sanitized(&self) -> Self {
        Self {
            node_port: self.node_port.and_then(port),
            https_port: self.https_port.filter(|value| *value != 0),
            update_owner: self.update_owner.as_deref().and_then(github_name),
            update_repository: self.update_repository.as_deref().and_then(github_name),
            update_key: self
                .update_key
                .as_deref()
                .map(str::trim)
                .map(str::to_ascii_lowercase)
                .filter(|key| super::release_manifest::from_hex::<32>(key).is_some()),
            tools: self
                .tools
                .iter()
                .filter(|(name, path)| {
                    TOOLS.contains(&name.as_str()) && Path::new(path.as_str()).is_absolute()
                })
                .map(|(name, path)| (name.clone(), path.clone()))
                .collect(),
        }
    }

    pub fn node_port(&self) -> u16 {
        self.node_port.unwrap_or(DEFAULT_NODE_PORT)
    }

    pub fn https_port(&self) -> u16 {
        self.https_port.unwrap_or(DEFAULT_HTTPS_PORT)
    }

    pub fn owner(&self) -> &str {
        self.update_owner.as_deref().unwrap_or(DEFAULT_OWNER)
    }

    pub fn repository(&self) -> &str {
        self.update_repository
            .as_deref()
            .unwrap_or(DEFAULT_REPOSITORY)
    }

    /// The key updates are checked against: the person's, or the one built into this build.
    pub fn trusted_key(&self) -> Option<[u8; 32]> {
        self.update_key
            .as_deref()
            .and_then(super::release_manifest::from_hex::<32>)
            .or_else(super::release_manifest::built_in_key)
    }

    /// Where the person said `name` is (`python3`, `codex.exe`… count as their tool).
    pub fn tool(&self, name: &str) -> Option<&Path> {
        let stem = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let stem = stem.split('.').next().unwrap_or(stem);
        let key = if stem.starts_with("python") || stem == "py" {
            "python"
        } else {
            stem
        };
        self.tools.get(key).map(Path::new)
    }
}

fn file(home: &Path) -> PathBuf {
    home.join(FOLDER).join(FILE)
}

fn plain_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file() && !meta.file_type().is_symlink())
}

pub fn load_in(home: &Path) -> Connection {
    let path = file(home);
    if !plain_file(&path) {
        return Connection::default();
    }
    fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Connection>(&bytes).ok())
        .map(|connection| connection.sanitized())
        .unwrap_or_default()
}

/// Why `connection` cannot be saved as it is, in words for the person; `Ok` when it is fine.
/// (A tool's path must exist; the rest are the rules of [`Connection::sanitized`], said out loud.)
pub fn check(connection: &Connection) -> Result<(), String> {
    if connection.node_port.is_some_and(|value| value < 1024) {
        return Err("El puerto del nodo debe estar entre 1024 y 65535.".into());
    }
    if connection.https_port == Some(0) {
        return Err("El puerto HTTPS de Tailscale no puede ser 0.".into());
    }
    for (label, value) in [
        ("El dueño de las versiones", &connection.update_owner),
        (
            "El repositorio de las versiones",
            &connection.update_repository,
        ),
    ] {
        if value
            .as_deref()
            .is_some_and(|text| github_name(text).is_none())
        {
            return Err(format!(
                "{label} solo admite letras, números, `-`, `_` y `.` (hasta 100)."
            ));
        }
    }
    if let Some(key) = &connection.update_key
        && super::release_manifest::from_hex::<32>(key.trim()).is_none()
    {
        return Err("La clave pública de firma son 64 dígitos hexadecimales.".into());
    }
    for (name, path) in &connection.tools {
        if !TOOLS.contains(&name.as_str()) {
            return Err(format!(
                "«{name}» no es una herramienta conocida ({}).",
                TOOLS.join(", ")
            ));
        }
        let path = Path::new(path);
        if !path.is_absolute() {
            return Err(format!("La ruta de {name} debe ser absoluta."));
        }
        if !path.is_file() {
            return Err(format!(
                "{name}: no existe un archivo en {}.",
                path.display()
            ));
        }
    }
    Ok(())
}

pub fn save_in(home: &Path, connection: &Connection) -> Result<(), String> {
    check(connection)?;
    let bytes =
        serde_json::to_vec_pretty(&connection.sanitized()).map_err(|error| error.to_string())?;
    let path = file(home);
    let parent = path.parent().ok_or("Ruta sin carpeta")?;
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    if plain_file(&path) {
        let _ = fs::copy(&path, path.with_extension("json.bak"));
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, bytes).map_err(|error| format!("{}: {error}", temporary.display()))?;
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("{}: {error}", path.display())
    })
}

/// Back to the product's values (the previous file stays as `.bak`).
pub fn reset_in(home: &Path) -> Result<(), String> {
    if !plain_file(&file(home)) {
        return Ok(());
    }
    save_in(home, &Connection::default())
}

pub fn load() -> Connection {
    application_home().map_or_else(Connection::default, |home| load_in(&home))
}

pub fn save(connection: &Connection) -> Result<(), String> {
    let home = application_home().ok_or("No se encontró la carpeta de la aplicación")?;
    save_in(&home, connection)
}

pub fn reset() -> Result<(), String> {
    let home = application_home().ok_or("No se encontró la carpeta de la aplicación")?;
    reset_in(&home)
}

/// The node's port on this PC.
pub fn node_port() -> u16 {
    load().node_port()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-conn-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn nothing_saved_means_the_products_values() {
        let connection = load_in(&home("none"));
        assert_eq!(connection.node_port(), 47_821);
        assert_eq!(connection.https_port(), 443);
        assert_eq!(connection.owner(), "chxchx-dev");
        assert_eq!(connection.repository(), "chxchx-tech-lead");
        assert!(connection.tool("codex").is_none());
    }

    #[test]
    fn what_is_saved_comes_back_and_the_previous_file_is_kept() {
        let dir = home("roundtrip");
        let mut mine = Connection {
            node_port: Some(50_000),
            https_port: Some(8443),
            update_owner: Some("mi-org".into()),
            update_repository: Some("mi.fork".into()),
            update_key: Some("AB".repeat(32)),
            ..Default::default()
        };
        save_in(&dir, &mine).unwrap();
        let back = load_in(&dir);
        assert_eq!(back.node_port(), 50_000);
        assert_eq!(back.https_port(), 8443);
        assert_eq!(back.owner(), "mi-org");
        assert_eq!(back.update_key.as_deref(), Some("ab".repeat(32).as_str()));
        assert_eq!(
            back.trusted_key(),
            super::super::release_manifest::from_hex::<32>(&"ab".repeat(32))
        );
        mine.node_port = Some(50_001);
        save_in(&dir, &mine).unwrap();
        assert!(dir.join("connection/connection.json.bak").is_file());
        reset_in(&dir).unwrap();
        assert_eq!(load_in(&dir), Connection::default());
    }

    #[test]
    fn invalid_values_are_refused_with_a_reason_and_dropped_when_read() {
        let dir = home("invalid");
        for (bad, word) in [
            (
                Connection {
                    node_port: Some(80),
                    ..Default::default()
                },
                "1024",
            ),
            (
                Connection {
                    update_owner: Some("a/b".into()),
                    ..Default::default()
                },
                "dueño",
            ),
            (
                Connection {
                    update_repository: Some(String::new()),
                    ..Default::default()
                },
                "repositorio",
            ),
            (
                Connection {
                    update_key: Some("1234".into()),
                    ..Default::default()
                },
                "64",
            ),
        ] {
            let reason = save_in(&dir, &bad).unwrap_err();
            assert!(reason.contains(word), "{reason}");
        }
        // A hand-edited file never takes effect with bad values in it.
        fs::create_dir_all(dir.join("connection")).unwrap();
        fs::write(
            dir.join("connection/connection.json"),
            r#"{"node_port":10,"update_owner":"../x","update_key":"zz","tools":{"rm":"/bin/rm","codex":"relative"}}"#,
        )
        .unwrap();
        assert_eq!(load_in(&dir), Connection::default());
    }

    #[test]
    fn a_tool_must_be_an_absolute_path_to_a_real_file_and_one_that_is_known() {
        let dir = home("tools");
        let real = dir.join("codex-here");
        fs::write(&real, "#!/bin/sh\n").unwrap();
        let mut connection = Connection::default();
        connection
            .tools
            .insert("codex".into(), real.display().to_string());
        save_in(&dir, &connection).unwrap();
        let back = load_in(&dir);
        assert_eq!(back.tool("codex"), Some(real.as_path()));
        assert_eq!(back.tool("codex.exe"), Some(real.as_path()));
        connection
            .tools
            .insert("python".into(), real.display().to_string());
        assert_eq!(
            connection.tool("python3"),
            Some(real.as_path()),
            "python3 es «python»"
        );
        connection
            .tools
            .insert("codex".into(), dir.join("missing").display().to_string());
        assert!(
            save_in(&dir, &connection)
                .unwrap_err()
                .contains("no existe")
        );
        connection.tools.clear();
        connection
            .tools
            .insert("rm".into(), real.display().to_string());
        assert!(
            save_in(&dir, &connection)
                .unwrap_err()
                .contains("herramienta")
        );
    }
}
