use crate::integrations::mcp_clients::{self, CommandOutput};
use std::path::Path;

pub fn command_name() -> Option<String> {
    if mcp_clients::find_executable("basic-memory").is_some() {
        Some("basic-memory".to_owned())
    } else if mcp_clients::find_executable("bm").is_some() {
        Some("bm".to_owned())
    } else {
        None
    }
}

/// Ensures the local memory directory is registered, returning true when it added it.
pub fn ensure_project(
    executable: &str,
    project: &str,
    memory_path: &Path,
    cwd: &Path,
) -> Result<bool, String> {
    let info = vec![
        executable.to_owned(),
        "project".to_owned(),
        "info".to_owned(),
        project.to_owned(),
        "--json".to_owned(),
    ];
    if run(&info, cwd)?.status == 0 {
        return Ok(false);
    }
    let add = vec![
        executable.to_owned(),
        "project".to_owned(),
        "add".to_owned(),
        project.to_owned(),
        memory_path.display().to_string(),
    ];
    let added = run(&add, cwd)?;
    if added.status == 0 {
        return Ok(true);
    }
    if run(&info, cwd)?.status == 0 {
        return Ok(false);
    }
    Err(command_error(&added))
}

fn run(command: &[String], cwd: &Path) -> Result<CommandOutput, String> {
    mcp_clients::run(command, cwd)
}

fn command_error(output: &CommandOutput) -> String {
    let detail = if output.stderr.trim().is_empty() {
        output.stdout.trim()
    } else {
        output.stderr.trim()
    };
    if detail.is_empty() {
        format!("Basic Memory respondió con código {}", output.status)
    } else {
        format!(
            "Basic Memory respondió con código {}: {}",
            output.status,
            detail.chars().take(500).collect::<String>()
        )
    }
}
