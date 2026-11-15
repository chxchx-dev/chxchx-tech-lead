//! One command to open everything: `chxchx-tech` alone, or with a folder, opens the best
//! interface for where it runs; commands that belong to the legacy CLI (agent, bridge,
//! local-models…) are handed to it, so a single name serves the whole product.
use crate::integrations::chat_bridge::legacy_cli;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Set on the legacy CLI we start, so a command it does not know can never come back here.
const DELEGATED: &str = "CHXCHX_DELEGATED";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    Studio,
    Tui,
    Help,
}

/// What `chxchx-tech` opens by itself: Studio when there is a desktop to draw it on, the TUI in
/// a terminal otherwise, and the help when it is neither (a pipe, a script).
pub fn choose(desktop: bool, terminal: bool, studio: bool, tui: bool) -> Choice {
    if desktop && studio {
        Choice::Studio
    } else if terminal && tui {
        Choice::Tui
    } else {
        Choice::Help
    }
}

/// Windows and macOS always have one; elsewhere a display server must be reachable, and a
/// remote shell cannot draw a window.
pub fn desktop_available() -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        return std::env::var_os("SSH_CONNECTION").is_none();
    }
    let has = |name: &str| std::env::var_os(name).is_some_and(|v| !v.is_empty());
    (has("WAYLAND_DISPLAY") || has("DISPLAY")) && std::env::var_os("SSH_CONNECTION").is_none()
}

pub fn choice_here() -> Choice {
    choose(
        desktop_available(),
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        cfg!(feature = "studio"),
        cfg!(feature = "tui"),
    )
}

#[cfg(feature = "studio")]
/// The Studio program: the lean `chxchx-studio` next to this one when installed, else ourselves.
fn studio_command(path: &Path) -> Command {
    let sibling = std::env::current_exe().ok().and_then(|exe| {
        let name = if cfg!(windows) {
            "chxchx-studio.exe"
        } else {
            "chxchx-studio"
        };
        let candidate = exe.with_file_name(name);
        candidate.is_file().then_some(candidate)
    });
    match sibling {
        Some(program) => {
            let mut command = Command::new(program);
            command.arg(path);
            command
        }
        None => {
            let mut command =
                Command::new(std::env::current_exe().unwrap_or_else(|_| "chxchx-tech".into()));
            command.arg("studio").arg(path);
            command
        }
    }
}

#[cfg(feature = "studio")]
/// Opens Studio in its own window and returns, so the terminal stays yours.
pub fn spawn_studio(path: &Path) -> Result<(), String> {
    use std::process::Stdio;
    let mut command = studio_command(path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
        command.creation_flags(0x0000_0008 | 0x0000_0200);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("No se pudo abrir ChxChx Studio: {error}"))
}

/// A first word that is a folder opens it (`chxchx-tech ~/mi-proyecto`).
pub fn folder_argument(arguments: &[OsString]) -> Option<PathBuf> {
    let first = arguments.first()?;
    (arguments.len() == 1 && Path::new(first).is_dir()).then(|| PathBuf::from(first))
}

/// Runs a command the legacy CLI owns and returns its exit code.
pub fn delegate(arguments: &[OsString]) -> Result<i32, String> {
    let name = arguments
        .first()
        .map(|a| a.to_string_lossy().into_owned())
        .unwrap_or_default();
    if std::env::var_os(DELEGATED).is_some() {
        return Err(format!(
            "`{name}` no es un comando de ChxChx. Mira `chxchx-tech --help`."
        ));
    }
    let project = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let Some(cli) = legacy_cli(&project) else {
        return Err(format!(
            "`{name}` no es un comando de esta versión. Los comandos heredados (agent, bridge, local-models…) \
             necesitan el CLI de Python que acompaña a la instalación (o CHXCHX_TECH_CLI); mira `chxchx-tech --help`."
        ));
    };
    Command::new(cli)
        .args(arguments)
        .env(DELEGATED, "1")
        .status()
        .map(|status| status.code().unwrap_or(1))
        .map_err(|error| format!("No se pudo ejecutar `{name}`: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_best_interface_for_where_it_runs() {
        assert_eq!(choose(true, true, true, true), Choice::Studio);
        assert_eq!(choose(false, true, true, true), Choice::Tui, "no desktop");
        assert_eq!(
            choose(true, true, false, true),
            Choice::Tui,
            "a build without Studio"
        );
        assert_eq!(
            choose(false, false, true, true),
            Choice::Help,
            "a pipe or a script"
        );
        assert_eq!(choose(true, false, false, false), Choice::Help);
    }

    #[test]
    fn only_a_single_existing_folder_counts_as_a_project_to_open() {
        let dir = std::env::temp_dir();
        assert_eq!(
            folder_argument(&[dir.clone().into_os_string()]),
            Some(dir.clone())
        );
        assert!(folder_argument(&["agent".into()]).is_none());
        assert!(folder_argument(&[dir.into_os_string(), "extra".into()]).is_none());
        assert!(folder_argument(&[]).is_none());
    }
}
