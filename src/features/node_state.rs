//! Where a running node says where it is (`node.json`) and the token its local clients use.
//!
//! Both live in `<CHXCHX_TECH_HOME>/node/`, readable only by this user. The token is what proves
//! a request comes from someone who can read this user's files; the node also refuses requests
//! that are not addressed to loopback (see the node API).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeInfo {
    pub pid: u32,
    pub port: u16,
    pub version: String,
    pub protocol: u32,
    pub started_ms: u64,
}

pub fn dir() -> Option<PathBuf> {
    crate::features::project_profiles::application_home().map(|home| home.join("node"))
}

pub fn read_info(dir: &Path) -> Option<NodeInfo> {
    serde_json::from_slice(&fs::read(dir.join("node.json")).ok()?).ok()
}

pub fn write_info(dir: &Path, info: &NodeInfo) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(info).map_err(|error| error.to_string())?;
    write_private(&dir.join("node.json"), &bytes)
}

pub fn remove_info(dir: &Path) {
    let _ = fs::remove_file(dir.join("node.json"));
}

pub fn read_token(dir: &Path) -> Option<String> {
    let token = fs::read_to_string(dir.join("token")).ok()?;
    let token = token.trim();
    (token.len() >= 32).then(|| token.to_owned())
}

/// The existing token, or a new one made from `random` bytes (64 hex characters).
pub fn ensure_token(dir: &Path, random: impl FnOnce() -> [u8; 32]) -> Result<String, String> {
    if let Some(token) = read_token(dir) {
        return Ok(token);
    }
    let token: String = random().iter().map(|byte| format!("{byte:02x}")).collect();
    write_private(&dir.join("token"), token.as_bytes())?;
    Ok(token)
}

/// Largest `node.log` before it is rotated to `node.log.1` (the previous one is dropped).
pub const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Appends a timestamped line to `node.log`. Logging never fails the request that logs.
pub fn append_log(dir: &Path, line: &str) {
    static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = WRITING.lock().unwrap_or_else(|poison| poison.into_inner());
    let path = dir.join("node.log");
    if fs::metadata(&path).is_ok_and(|meta| meta.len() >= MAX_LOG_BYTES) {
        let _ = fs::rename(&path, dir.join("node.log.1"));
    }
    let stamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let line: String = line.chars().filter(|c| !c.is_control()).take(500).collect();
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if let Ok(mut file) = options.open(&path) {
        let _ = writeln!(file, "{stamp} {line}");
    }
}

pub fn create_private_dir(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|error| format!("No se pudo crear {}: {error}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("No se pudo proteger {}: {error}", dir.display()))?;
    }
    Ok(())
}

/// Written next to its final place and renamed, readable only by this user on Unix.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = options
        .open(&temp)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|error| format!("No se pudo escribir {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-node-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        create_private_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn the_token_is_made_once_kept_private_and_reused() {
        let dir = scratch("token");
        let first = ensure_token(&dir, || [7; 32]).unwrap();
        assert_eq!(first.len(), 64);
        let again = ensure_token(&dir, || panic!("must not make a new one")).unwrap();
        assert_eq!(again, first);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join("token"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
            let mode = fs::metadata(&dir).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        fs::write(dir.join("token"), "short").unwrap();
        assert_eq!(read_token(&dir), None, "a damaged token is not trusted");
    }

    #[test]
    fn the_log_is_one_line_per_entry_and_rotates_when_it_grows() {
        let dir = scratch("log");
        append_log(&dir, "uno\ncon salto");
        let text = fs::read_to_string(dir.join("node.log")).unwrap();
        assert_eq!(text.lines().count(), 1, "a line cannot forge another entry");
        assert!(text.trim_end().ends_with("unocon salto"));
        fs::write(dir.join("node.log"), vec![b'x'; MAX_LOG_BYTES as usize]).unwrap();
        append_log(&dir, "nuevo");
        assert!(fs::metadata(dir.join("node.log.1")).unwrap().len() >= MAX_LOG_BYTES);
        assert_eq!(
            fs::read_to_string(dir.join("node.log"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }

    #[test]
    fn node_info_round_trips_and_is_gone_after_removal() {
        let dir = scratch("info");
        let info = NodeInfo {
            pid: 1,
            port: 4000,
            version: "0.6.3".into(),
            protocol: 1,
            started_ms: 5,
        };
        write_info(&dir, &info).unwrap();
        assert_eq!(read_info(&dir), Some(info));
        remove_info(&dir);
        assert_eq!(read_info(&dir), None);
    }
}
