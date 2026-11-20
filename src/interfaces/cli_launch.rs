//! One command to open everything: `chxchx-tech` alone, or with a folder, opens the best
//! interface for where it runs. Commands of the earlier Python CLI that were not carried over say
//! where their function lives now instead of failing with a bare "unknown command".
use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
#[cfg(feature = "studio")]
use std::process::Command;

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

/// What to tell someone who types a command of the earlier CLI that this product no longer has.
/// `scope` is the command group it was typed under (`""`, `workspace`, `projects`, `agent`).
pub fn retired(scope: &str, arguments: &[OsString]) -> String {
    let name = arguments
        .first()
        .map(|argument| argument.to_string_lossy().into_owned())
        .unwrap_or_default();
    let studio = "abre el Studio (`chxchx-tech studio .`)";
    let guidance = match (scope, name.as_str()) {
        ("", "run" | "attach" | "stop") => {
            format!("usa `workspace start` y `workspace stop`, o {studio}")
        }
        ("", "ci" | "deploy") => {
            "ya no existe: la integración CI/CD de proyectos no forma parte del producto".to_owned()
        }
        ("", "editor") => format!("el editor está en el Studio: {studio}"),
        ("", "bridge") => "era el contrato interno del Studio anterior; ya no existe".to_owned(),
        ("", "install" | "setup") => {
            "usa `init . --apply` para preparar un proyecto, o el instalador visual".to_owned()
        }
        ("", "sync") => "usa `projects sync`".to_owned(),
        ("", "skill") => "ahora es `skills`".to_owned(),
        ("", "pack") => "ahora es `packs`".to_owned(),
        ("", "mod") => "ahora es `mods`".to_owned(),
        ("workspace", "attach" | "terminal" | "open") => {
            format!("las terminales y sesiones viven en el Studio: {studio}")
        }
        ("workspace", "status") => "usa `status`".to_owned(),
        ("workspace", "handoff") => {
            "el traspaso de contexto lo hace el Studio al llenarse el chat".to_owned()
        }
        ("projects", "prepare") => "usa `init . --apply`".to_owned(),
        ("agent" | "", "chat" | "chat-handoff")
        | ("agent", "add" | "start" | "attach" | "preflight" | "list" | "status")
        | ("agent", "") => {
            "los agentes se abren desde el Studio o el TUI (`chxchx-tech tui .`); `agent chat-state` sigue disponible".to_owned()
        }
        _ => {
            let command = if scope.is_empty() {
                name.clone()
            } else {
                format!("{scope} {name}")
            };
            return format!("`{command}` no es un comando de ChxChx. Mira `chxchx-tech --help`.");
        }
    };
    let command = match (scope, name.as_str()) {
        ("", _) | ("agent", "") => name,
        _ => format!("{scope} {name}"),
    };
    format!("`{}` se retiró: {guidance}.", command.trim())
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
