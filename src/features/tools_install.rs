//! `tools install`: installs the two tools ChxChx's memory and code integrations rely on —
//! Basic Memory and Serena — as `uv` tools, the way the installers always did.
//!
//! A tool that is already on disk is left alone, `--dry-run` only prints the `uv` command, and
//! nothing here edits any configuration: connecting them to the agents is `integrate`.
use crate::integrations::executable_search::{agent_path, find_in};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Tool {
    pub name: &'static str,
    /// Arguments of `uv` that install it.
    pub install: &'static [&'static str],
    /// The commands it leaves in the tool directory; any one of them means it is installed.
    pub commands: &'static [&'static str],
}

pub const TOOLS: [Tool; 2] = [
    Tool {
        name: "basic-memory",
        install: &[
            "tool",
            "install",
            "-p",
            "3.12",
            "--prerelease",
            "allow",
            "basic-memory",
        ],
        commands: &["basic-memory", "bm"],
    },
    Tool {
        name: "serena",
        install: &["tool", "install", "-p", "3.13", "serena-agent"],
        commands: &["serena"],
    },
];

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    AlreadyInstalled,
    Installed,
    /// The command that would run.
    Preview(Vec<String>),
}

pub fn find_tool(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|tool| tool.name == name)
}

/// Where `uv` puts the commands of its tools: `UV_TOOL_BIN_DIR`, the `tools` folder of an MSI
/// install (marked by `.chxchx-msi` above the running program), or `~/.local/bin`.
pub fn tool_bin_directory() -> PathBuf {
    if let Some(configured) = std::env::var_os("UV_TOOL_BIN_DIR").filter(|value| !value.is_empty())
    {
        return PathBuf::from(configured);
    }
    if let Ok(executable) = std::env::current_exe() {
        for root in executable.ancestors().skip(1).take(6) {
            if root.join(".chxchx-msi").is_file()
                && let Some(parent) = root.parent()
            {
                return parent.join("tools");
            }
        }
    }
    std::env::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("bin")
}

/// `uv`: the one an MSI install carries in its private Python runtime, the one on the `PATH`,
/// then the folders the official installer uses.
pub fn find_uv() -> Option<PathBuf> {
    if let Ok(executable) = std::env::current_exe() {
        for root in executable.ancestors().skip(1).take(6) {
            let bundled = root.join(".venv").join("python").join("uv.exe");
            if root.join(".chxchx-msi").is_file() && bundled.is_file() {
                return Some(bundled);
            }
        }
    }
    if let Some(found) = find_in("uv", &agent_path()) {
        return Some(found);
    }
    let home = std::env::home_dir()?;
    let local = home.join(".local").join("bin");
    ["uv", "uv.exe"]
        .into_iter()
        .map(|name| local.join(name))
        .find(|candidate| candidate.is_file())
}

pub fn is_installed(tool: &Tool, tool_bin: &Path) -> bool {
    tool.commands.iter().any(|command| {
        ["", ".exe", ".cmd"]
            .iter()
            .any(|suffix| tool_bin.join(format!("{command}{suffix}")).is_file())
    })
}

/// Installs one tool. `uv` and `tool_bin` are given so tests do not touch the machine.
pub fn install_with(
    tool: &Tool,
    uv: Option<&Path>,
    tool_bin: &Path,
    dry_run: bool,
) -> Result<Outcome, String> {
    if is_installed(tool, tool_bin) {
        return Ok(Outcome::AlreadyInstalled);
    }
    let program = uv.map_or_else(|| "uv".to_owned(), |uv| uv.display().to_string());
    let mut shown = vec![program];
    shown.extend(tool.install.iter().map(|part| (*part).to_owned()));
    if dry_run {
        return Ok(Outcome::Preview(shown));
    }
    let uv = uv.ok_or("uv no está instalado o no está disponible en el PATH")?;
    let mut path = vec![tool_bin.to_path_buf()];
    path.extend(uv.parent().map(Path::to_path_buf));
    path.extend(std::env::split_paths(&agent_path()));
    let path = std::env::join_paths(path).map_err(|error| error.to_string())?;
    let status = Command::new(uv)
        .args(tool.install)
        .env("PATH", path)
        .env("UV_TOOL_BIN_DIR", tool_bin)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("No pude ejecutar uv: {error}"))?;
    if !status.success() {
        return Err(format!(
            "No se pudo instalar {} (uv terminó con {}). Revisa tu conexión e inténtalo de nuevo.",
            tool.name,
            status
                .code()
                .map_or("una señal".to_owned(), |code| code.to_string())
        ));
    }
    Ok(Outcome::Installed)
}

pub fn install(name: &str, dry_run: bool) -> Result<Outcome, String> {
    let tool = find_tool(name).ok_or_else(|| format!("Herramienta desconocida: {name}"))?;
    install_with(tool, find_uv().as_deref(), &tool_bin_directory(), dry_run)
}

#[cfg(test)]
#[path = "tools_install_tests.rs"]
mod tests;
