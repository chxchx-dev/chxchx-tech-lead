//! Trust policy for a user-requested workspace shell or agent.
use crate::features::agents::Agent;
use crate::features::project_overview::inspect_project;
use crate::integrations::executable_search::agent_path;
use crate::integrations::terminal::Session;
use portable_pty::CommandBuilder;
use std::path::Path;

fn require_trust(root: &Path, what: &str) -> Result<(), String> {
    if inspect_project(root)?.workspace.trusted {
        Ok(())
    } else {
        Err(format!(
            "Revisa y concede confianza al proyecto antes de abrir {what}."
        ))
    }
}

/// Runs a configured agent (or a detected one) in a terminal of its own, in the project.
pub fn open_agent(root: &Path, agent: &Agent) -> Result<Session, String> {
    require_trust(root, "un agente")?;
    let executable = agent.executable.as_ref().ok_or_else(|| {
        let hint = agent.provider.install_hint();
        if hint.is_empty() {
            format!(
                "`{}` no está instalado o no está en el PATH.",
                agent.command[0]
            )
        } else {
            format!(
                "{} no está instalado. Instálalo con: {hint}",
                agent.provider.label()
            )
        }
    })?;
    let mut command = CommandBuilder::new(executable);
    command.args(&agent.command[1..]);
    command.cwd(&agent.cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    // A Studio started from a launcher has a minimal PATH: the agent's own `node` must be found.
    command.env("PATH", agent_path());
    for (key, value) in crate::integrations::process_memory::LEAN_ENV {
        command.env(key, value);
    }
    Session::spawn(command)
}

pub fn open(root: &Path) -> Result<Session, String> {
    require_trust(root, "su terminal")?;
    let mut command = if cfg!(windows) {
        let mut command = CommandBuilder::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile"]);
        command
    } else {
        let shell = user_shell(std::env::var_os("SHELL").as_deref());
        let mut command = CommandBuilder::new(&shell);
        // macOS terminals start a login shell, so the user's PATH and profile are the usual ones.
        if cfg!(target_os = "macos") {
            command.arg("-l");
        }
        command
    };
    command.cwd(root);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    for (key, value) in crate::integrations::process_memory::LEAN_ENV {
        command.env(key, value);
    }
    Session::spawn(command)
}

/// What a shell tab is called: the name of the shell that runs in it (`bash`, `zsh`, `PowerShell`).
pub fn shell_name() -> String {
    if cfg!(windows) {
        return "PowerShell".to_owned();
    }
    let shell = user_shell(std::env::var_os("SHELL").as_deref());
    shell.file_name().map_or_else(
        || "shell".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The shell the person already uses (`$SHELL`), so the terminal has their completion, history,
/// prompt and aliases. Only an absolute path to an existing file is trusted; otherwise `/bin/sh`.
fn user_shell(configured: Option<&std::ffi::OsStr>) -> std::path::PathBuf {
    configured
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("/bin/sh"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_shell_tab_is_named_after_its_shell() {
        let name = super::shell_name();
        assert!(!name.is_empty() && !name.contains('/'), "{name}");
    }

    #[test]
    fn the_terminal_uses_the_shell_of_the_system_and_falls_back_to_sh() {
        let sh = std::ffi::OsString::from("/bin/sh");
        assert_eq!(
            super::user_shell(Some(&sh)),
            std::path::PathBuf::from("/bin/sh")
        );
        for bad in ["bash", "/no/existe/zsh", "/tmp"] {
            let bad = std::ffi::OsString::from(bad);
            assert_eq!(
                super::user_shell(Some(&bad)),
                std::path::PathBuf::from("/bin/sh")
            );
        }
        assert_eq!(super::user_shell(None), std::path::PathBuf::from("/bin/sh"));
        if let Some(own) = std::env::var_os("SHELL").filter(|s| std::path::Path::new(s).is_file()) {
            assert_eq!(super::user_shell(Some(&own)), std::path::PathBuf::from(own));
        }
    }

    #[test]
    fn an_untrusted_project_cannot_start_a_terminal() {
        let root = std::env::temp_dir().join(format!(
            "chxchx-untrusted-{}-{}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let result = super::open(&root);
        let _ = std::fs::remove_dir(&root);
        assert!(result.unwrap_err().contains("confianza"));
    }

    #[test]
    fn an_untrusted_project_cannot_start_an_agent_even_if_it_is_installed() {
        let root = std::env::temp_dir().join(format!(
            "chxchx-untrusted-agent-{}-{}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let agent = crate::features::agents::Agent {
            id: "claude".into(),
            provider: crate::features::agents::Provider::Claude,
            command: vec!["claude".into()],
            cwd: root.clone(),
            executable: Some(std::path::PathBuf::from("claude")),
            configured: true,
        };
        let result = super::open_agent(&root, &agent);
        let _ = std::fs::remove_dir(&root);
        assert!(result.unwrap_err().contains("confianza"));
    }
}
