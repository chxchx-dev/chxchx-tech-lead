//! Which agents a project uses, and therefore where Mods are written.
use std::fs;
use std::path::Path;

/// Executable names of the agents declared in `.ai/chxchx-tech.toml`.
fn configured_agent_commands(root: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(root.join(".ai/chxchx-tech.toml")) else {
        return Vec::new();
    };
    let Ok(config) = toml::from_str::<toml::Table>(&text) else {
        return Vec::new();
    };
    let agents = config
        .get("workspace")
        .and_then(|workspace| workspace.get("agents"))
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    agents
        .iter()
        .filter_map(|agent| {
            let command = agent.get("command")?;
            let first = match command {
                toml::Value::Array(parts) => parts.first()?.as_str()?.to_owned(),
                toml::Value::String(text) => text.clone(),
                _ => return None,
            };
            let name = Path::new(&first.replace('\\', "/"))
                .file_name()?
                .to_string_lossy()
                .to_lowercase();
            Some(
                name.strip_suffix(".exe")
                    .or_else(|| name.strip_suffix(".cmd"))
                    .unwrap_or(&name)
                    .to_owned(),
            )
        })
        .collect()
}

/// Skill roots where Mods are written for this project: (label, relative directory).
pub(crate) fn install_roots(root: &Path) -> Vec<(&'static str, &'static str)> {
    let commands = configured_agent_commands(root);
    let mut roots = vec![("claude", ".claude/skills")];
    if commands.iter().any(|name| name == "codex") || root.join(".agents").is_dir() {
        roots.push(("agents", ".agents/skills"));
    }
    roots
}

/// Removal looks in every root, so a Mod is retired even after its agent was dropped.
pub(crate) const ALL_ROOTS: [(&str, &str); 2] =
    [("claude", ".claude/skills"), ("agents", ".agents/skills")];
