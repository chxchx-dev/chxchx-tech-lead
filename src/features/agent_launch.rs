//! Runs an agent in the terminal that is already open (the TUI hands its screen over and takes
//! it back). The same trust rule as the Studio terminals: nothing runs in an untrusted project.
use crate::features::agents::Agent;
use crate::features::project_overview::inspect_project;
use crate::integrations::executable_search::agent_path;
use std::path::Path;
use std::process::{Command, ExitStatus};

/// Why `agent` cannot be launched in `root`, if it cannot.
pub fn check(root: &Path, agent: &Agent) -> Result<(), String> {
    if !inspect_project(root)?.workspace.trusted {
        return Err("Revisa y concede confianza al proyecto antes de abrir un agente.".into());
    }
    if agent.executable.is_none() {
        let hint = agent.provider.install_hint();
        return Err(if hint.is_empty() {
            format!(
                "`{}` no está instalado o no está en el PATH.",
                agent.command[0]
            )
        } else {
            format!(
                "{} no está instalado. Instálalo con: {hint}",
                agent.provider.label()
            )
        });
    }
    Ok(())
}

/// Runs the agent with this process's terminal until it exits.
pub fn run(root: &Path, agent: &Agent) -> Result<ExitStatus, String> {
    check(root, agent)?;
    let executable = agent.executable.as_ref().ok_or("sin ejecutable")?;
    let mut command = Command::new(executable);
    command.args(&agent.command[1..]);
    command.current_dir(if agent.cwd.as_os_str().is_empty() {
        root
    } else {
        &agent.cwd
    });
    command.env("PATH", agent_path());
    for (key, value) in crate::integrations::process_memory::LEAN_ENV {
        command.env(key, value);
    }
    command
        .status()
        .map_err(|error| format!("No se pudo iniciar {}: {error}", agent.label()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::agents::Provider;

    fn agent(executable: Option<&str>) -> Agent {
        Agent {
            id: "claude".into(),
            provider: Provider::Claude,
            command: vec!["claude".into()],
            cwd: std::path::PathBuf::new(),
            executable: executable.map(Into::into),
            configured: false,
        }
    }

    #[test]
    fn an_agent_that_is_not_installed_says_how_to_get_it() {
        let dir = std::env::temp_dir();
        // The folder is not trusted, which is checked first.
        assert!(check(&dir, &agent(None)).unwrap_err().contains("confianza"));
    }
}
