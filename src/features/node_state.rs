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

/// Where the node may also be reached: the name `tailscale serve` gives this machine (ADR 0017).
/// Without this file the node answers only to `127.0.0.1` and `localhost`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// MagicDNS name, e.g. `mi-pc.tail1234.ts.net`.
    pub host: String,
    /// The HTTPS port `tailscale serve` listens on (443 by default).
    pub https_port: u16,
}

impl RemoteConfig {
    /// A configuration only for a Tailscale name: it cannot be used to authorise any other
    /// domain, whatever the file holds.
    pub fn new(host: &str, https_port: u16) -> Result<Self, String> {
        let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
        let label_ok = |label: &str| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        };
        let valid = host.len() <= 253
            && host.ends_with(".ts.net")
            && host.split('.').count() >= 4
            && host.split('.').all(label_ok);
        if !valid || https_port == 0 {
            return Err(format!(
                "«{host}:{https_port}» no es un nombre de Tailscale válido (termina en .ts.net)"
            ));
        }
        Ok(Self { host, https_port })
    }

    /// What a client puts in `Host`: the name, plus the port unless it is 443.
    pub fn authority(&self) -> String {
        if self.https_port == 443 {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.https_port)
        }
    }

    pub fn origin(&self) -> String {
        format!("https://{}", self.authority())
    }

    pub fn url(&self) -> String {
        format!("{}/", self.origin())
    }
}

pub fn read_remote(dir: &Path) -> Option<RemoteConfig> {
    let config: RemoteConfig =
        serde_json::from_slice(&fs::read(dir.join("remote.json")).ok()?).ok()?;
    RemoteConfig::new(&config.host, config.https_port).ok()
}

pub fn write_remote(dir: &Path, config: &RemoteConfig) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
    write_private(&dir.join("remote.json"), &bytes)
}

pub fn remove_remote(dir: &Path) {
    let _ = fs::remove_file(dir.join("remote.json"));
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
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
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
    fn only_a_tailscale_name_can_be_authorised_as_the_remote_host() {
        let remote = RemoteConfig::new("Fedora.Tail33E808.ts.net.", 443).unwrap();
        assert_eq!(remote.host, "fedora.tail33e808.ts.net");
        assert_eq!(remote.authority(), "fedora.tail33e808.ts.net");
        assert_eq!(remote.origin(), "https://fedora.tail33e808.ts.net");
        let other_port = RemoteConfig::new("fedora.tail33e808.ts.net", 8443).unwrap();
        assert_eq!(other_port.authority(), "fedora.tail33e808.ts.net:8443");
        for bad in [
            "evil.example",
            "ts.net",
            "x.ts.net",
            "evil.example/x.tail1.ts.net",
            "a b.tail1.ts.net",
            "-a.tail1.ts.net",
            "fedora.tail33e808.ts.net.evil.com",
        ] {
            assert!(RemoteConfig::new(bad, 443).is_err(), "{bad}");
        }
        assert!(RemoteConfig::new("a.tail1.ts.net", 0).is_err());

        let dir = scratch("remote");
        assert_eq!(read_remote(&dir), None);
        write_remote(&dir, &remote).unwrap();
        assert_eq!(read_remote(&dir), Some(remote));
        fs::write(
            dir.join("remote.json"),
            r#"{"host":"evil.example","https_port":443}"#,
        )
        .unwrap();
        assert_eq!(read_remote(&dir), None, "a damaged file authorises nothing");
        remove_remote(&dir);
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
