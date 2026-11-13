//! Trust policy for a user-requested workspace shell.
use crate::features::project_overview::inspect_project;
use crate::integrations::terminal::Session;
use portable_pty::CommandBuilder;
use std::path::Path;

pub fn open(root: &Path) -> Result<Session, String> {
    if !inspect_project(root)?.workspace.trusted {
        return Err("Revisa y concede confianza al proyecto antes de abrir su terminal.".into());
    }
    let mut command = if cfg!(windows) {
        let mut command = CommandBuilder::new("powershell.exe");
        command.args(["-NoLogo", "-NoProfile"]);
        command
    } else {
        CommandBuilder::new("/bin/sh")
    };
    command.cwd(root);
    command.env("TERM", "xterm-256color");
    Session::spawn(command)
}

#[cfg(test)]
mod tests {
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
}
