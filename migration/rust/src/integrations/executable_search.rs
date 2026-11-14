//! Finds agent CLIs the way a terminal would, even when Studio was not started from one.
//!
//! An app launched from a menu, the Dock or a double click gets a minimal PATH (on macOS just
//! `/usr/bin:/bin:/usr/sbin:/sbin`), so `claude`, `codex`, `opencode` and the `node` they run on are
//! not found although they work in the person's terminal. The usual install folders are searched
//! after PATH, so anything already on PATH keeps priority. Same rule as the legacy Python product.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Install folders that exist, in search order, after what PATH already has.
pub fn extra_directories(home: Option<&Path>) -> Vec<PathBuf> {
    let Some(home) = home else {
        return Vec::new();
    };
    let mut folders = vec![
        home.join(".opencode/bin"),
        home.join(".local/bin"),
        home.join(".claude/local"),
        home.join(".bun/bin"),
        home.join(".npm-global/bin"),
        home.join(".volta/bin"),
        home.join(".cargo/bin"),
        home.join(".asdf/shims"),
        home.join(".local/share/pnpm"),
        home.join(".local/share/fnm/aliases/default/bin"),
    ];
    // nvm keeps one folder per Node version; none of them is on a launcher's PATH. Newest first.
    if let Ok(read) = std::fs::read_dir(home.join(".nvm/versions/node")) {
        let mut versions: Vec<PathBuf> = read.filter_map(Result::ok).map(|e| e.path()).collect();
        versions.sort_by_key(|path| std::cmp::Reverse(version_key(path)));
        folders.extend(versions.into_iter().map(|path| path.join("bin")));
    }
    if cfg!(target_os = "macos") {
        folders.extend(
            ["/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin"].map(PathBuf::from),
        );
    } else if cfg!(windows) {
        for variable in ["APPDATA", "LOCALAPPDATA"] {
            if let Some(base) = std::env::var_os(variable) {
                folders.push(PathBuf::from(base).join("npm"));
            }
        }
        folders.push(home.join("AppData/Roaming/npm"));
    } else {
        folders.extend(
            [
                "/usr/local/bin",
                "/snap/bin",
                "/home/linuxbrew/.linuxbrew/bin",
            ]
            .map(PathBuf::from),
        );
    }
    folders
        .into_iter()
        .filter(|folder| folder.is_dir())
        .collect()
}

/// `v22.1.0` -> (22, 1, 0), so versions sort numerically (v9 before v10).
fn version_key(path: &Path) -> (u64, u64, u64) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut parts = name
        .trim_start_matches('v')
        .split('.')
        .map(|p| p.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// PATH followed by the install folders not already in it.
pub fn search_path(base: Option<OsString>, home: Option<&Path>) -> OsString {
    let mut folders: Vec<PathBuf> = base
        .as_deref()
        .map(|value| {
            std::env::split_paths(value)
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        })
        .unwrap_or_default();
    for extra in extra_directories(home) {
        if !folders.contains(&extra) {
            folders.push(extra);
        }
    }
    std::env::join_paths(folders).unwrap_or_default()
}

/// The PATH an agent should run with, built from this process's PATH and home.
pub fn agent_path() -> OsString {
    search_path(std::env::var_os("PATH"), std::env::home_dir().as_deref())
}

/// Full path of `name` (honouring PATHEXT on Windows), or `None`.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    find_in(name, &agent_path())
}

pub fn find_in(name: &str, path: &OsString) -> Option<PathBuf> {
    if name.contains(['/', '\\']) {
        let direct = PathBuf::from(name);
        return direct.is_file().then_some(direct);
    }
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
            .split(';')
            .map(str::to_ascii_lowercase)
            .collect()
    } else {
        vec![String::new()]
    };
    for folder in std::env::split_paths(path) {
        for extension in &extensions {
            let candidate = folder.join(format!("{name}{extension}"));
            if candidate.is_file() && is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-exec-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn install(folder: &Path, name: &str) -> PathBuf {
        fs::create_dir_all(folder).unwrap();
        let program = folder.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        });
        fs::write(&program, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        }
        program
    }

    #[test]
    fn an_agent_in_its_own_install_folder_is_found_without_the_shell_profile() {
        let home = scratch("home");
        let program = install(&home.join(".opencode/bin"), "opencode-xyz");
        let launcher_path = search_path(Some(OsString::from("/usr/bin")), Some(&home));

        assert_eq!(find_in("opencode-xyz", &launcher_path), Some(program));
        assert_eq!(find_in("opencode-xyz", &OsString::from("/usr/bin")), None);
    }

    #[test]
    fn what_is_already_on_path_wins_and_nothing_is_added_twice() {
        let home = scratch("priority");
        let first = install(&home.join("first"), "agente-xyz");
        install(&home.join(".local/bin"), "agente-xyz");
        let base = std::env::join_paths([home.join("first"), PathBuf::from("/usr/bin")]).unwrap();

        let path = search_path(Some(base), Some(&home));

        assert_eq!(find_in("agente-xyz", &path), Some(first));
        let again = search_path(Some(path.clone()), Some(&home));
        assert_eq!(again, path);
    }

    #[test]
    fn node_versions_are_searched_newest_first_by_number_and_missing_folders_are_ignored() {
        let home = scratch("nvm");
        install(&home.join(".nvm/versions/node/v9.0.0/bin"), "claude-xyz");
        let newest = install(&home.join(".nvm/versions/node/v22.1.0/bin"), "claude-xyz");
        install(&home.join(".nvm/versions/node/v10.0.0/bin"), "claude-xyz");

        let path = search_path(Some(OsString::from("/usr/bin")), Some(&home));

        assert_eq!(find_in("claude-xyz", &path), Some(newest));
        assert!(!std::env::split_paths(&path).any(|p| p == home.join(".bun/bin")));
    }

    #[test]
    fn a_path_given_directly_is_used_as_is() {
        let home = scratch("direct");
        let program = install(&home, "directo");
        assert_eq!(
            find_in(program.to_str().unwrap(), &OsString::new()),
            Some(program)
        );
        assert_eq!(find_in("/no/existe/agente", &OsString::new()), None);
    }
}
