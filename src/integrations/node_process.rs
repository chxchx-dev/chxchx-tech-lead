//! Starts the local node as a background process of this user, from Studio ("Iniciar el nodo").
//!
//! The node is the `chxchx-tech node start` command, so this needs a `chxchx-tech` built with the
//! node in it. The process is detached: it has its own process group, so closing Studio (or the
//! terminal that opened it) does not stop it, and it writes to a log file rather than to a pipe
//! Studio would have to keep open. For starting at every login, see `chxchx-tech node service`.

use crate::features::node_state;
use crate::integrations::executable_search::{agent_path, find_in};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

fn program_name() -> &'static str {
    if cfg!(windows) {
        "chxchx-tech.exe"
    } else {
        "chxchx-tech"
    }
}

/// The `chxchx-tech` that goes with this program: next to it when installed, else on the `PATH`.
pub fn locate() -> Option<PathBuf> {
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name(program_name()))
        .filter(|path| path.is_file());
    beside.or_else(|| find_in(program_name(), &agent_path()))
}

/// Starts the node and waits a moment to see it did not exit at once. A `chxchx-tech` built
/// without the node says so and stops, and that message is what the person gets.
pub fn start() -> Result<(), String> {
    let program = locate().ok_or(
        "No encuentro `chxchx-tech`. Instala el paquete completo o ejecuta `chxchx-tech node start` en una terminal.",
    )?;
    let dir = node_state::dir().ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    node_state::create_private_dir(&dir)?;
    let log = dir.join("start.log");
    let sink = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .map_err(|error| format!("No se pudo abrir {}: {error}", log.display()))?;
    let errors = sink
        .try_clone()
        .map_err(|error| format!("No se pudo abrir {}: {error}", log.display()))?;
    let mut command = Command::new(&program);
    command
        .args(["node", "start"])
        .stdin(Stdio::null())
        .stdout(Stdio::from(sink))
        .stderr(Stdio::from(errors));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
        command.creation_flags(0x0000_0008 | 0x0000_0200 | 0x0800_0000);
    }
    let size_before = std::fs::metadata(&log).map_or(0, |meta| meta.len());
    let mut child = command
        .spawn()
        .map_err(|error| format!("No se pudo iniciar {}: {error}", program.display()))?;
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(100));
        if let Ok(Some(status)) = child.try_wait() {
            if status.success() {
                return Ok(());
            }
            return Err(format!(
                "El nodo no arrancó: {}",
                new_text(&log, size_before).unwrap_or_else(|| "sin detalles".to_owned())
            ));
        }
    }
    Ok(()) // still running: the node is on its own now
}

/// What was added to the log after `from`, as one short line.
fn new_text(log: &std::path::Path, from: u64) -> Option<String> {
    let bytes = std::fs::read(log).ok()?;
    let text = String::from_utf8_lossy(bytes.get(from as usize..)?).into_owned();
    let line = text.lines().rev().find(|line| !line.trim().is_empty())?;
    Some(line.chars().take(240).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_new_line_of_the_log_is_the_reason() {
        let dir = std::env::temp_dir().join(format!("chxchx-node-process-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("start.log");
        std::fs::write(&log, "arranque viejo\n").unwrap();
        let before = std::fs::metadata(&log).unwrap().len();
        assert_eq!(new_text(&log, before), None, "nothing new yet");
        std::fs::write(
            &log,
            "arranque viejo\nerror: unrecognized subcommand 'node'\n\n",
        )
        .unwrap();
        assert_eq!(
            new_text(&log, before).as_deref(),
            Some("error: unrecognized subcommand 'node'")
        );
        assert_eq!(new_text(&log, 99_999), None);
    }
}
