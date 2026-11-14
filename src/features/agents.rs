//! The agents of a project: what its configuration declares and what is installed.
//!
//! Studio shows them in one place and opens each in two forms: a terminal (the agent's own TUI) or a
//! visual chat. Configured agents come from `workspace.agents` of `.ai/chxchx-tech.toml`; Claude Code,
//! Codex and OpenCode found on this machine but not configured are offered too, so the first
//! agent is one click away. Nothing here starts a process.
use crate::integrations::executable_search::{agent_path, find_in};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Claude,
    Codex,
    OpenCode,
    Other,
}

impl Provider {
    pub const KNOWN: [Provider; 3] = [Provider::Claude, Provider::Codex, Provider::OpenCode];

    /// Provider of a command, from the name of its program (`C:\npm\claude.cmd` -> Claude).
    pub fn from_command(program: &str) -> Self {
        let name = program.replace('\\', "/");
        let name = name.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
        let name = [".exe", ".cmd", ".bat"]
            .iter()
            .find_map(|suffix| name.strip_suffix(suffix))
            .unwrap_or(&name);
        match name {
            "claude" => Provider::Claude,
            "codex" => Provider::Codex,
            "opencode" => Provider::OpenCode,
            _ => Provider::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Claude => "Claude Code",
            Provider::Codex => "Codex",
            Provider::OpenCode => "OpenCode",
            Provider::Other => "Agente",
        }
    }

    pub fn program(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
            Provider::OpenCode => "opencode",
            Provider::Other => "",
        }
    }

    pub fn install_hint(self) -> &'static str {
        match self {
            Provider::Claude => "npm install -g @anthropic-ai/claude-code",
            Provider::Codex => "npm install -g @openai/codex",
            Provider::OpenCode => "npm install -g opencode-ai",
            Provider::Other => "",
        }
    }

    /// The visual chat speaks these three (through the shared bridge).
    pub fn has_visual_chat(self) -> bool {
        self != Provider::Other
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    pub id: String,
    pub provider: Provider,
    /// Program and arguments, exactly as configured (or the provider's own name when detected).
    pub command: Vec<String>,
    /// Working directory, always inside the project.
    pub cwd: PathBuf,
    /// Where the program was found, if it is installed.
    pub executable: Option<PathBuf>,
    /// Declared in `.ai/chxchx-tech.toml`; otherwise only detected on this machine.
    pub configured: bool,
}

impl Agent {
    pub fn available(&self) -> bool {
        self.executable.is_some()
    }

    pub fn label(&self) -> String {
        match self.provider {
            Provider::Other => self.id.clone(),
            provider => provider.label().to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentCatalog {
    pub agents: Vec<Agent>,
    /// Why the project's own agents could not be read (the detected ones are still listed).
    pub config_error: Option<String>,
}

pub fn list(root: &Path) -> AgentCatalog {
    list_with(root, &agent_path())
}

pub fn list_with(root: &Path, path: &OsString) -> AgentCatalog {
    let mut catalog = AgentCatalog::default();
    match configured(root) {
        Ok(entries) => {
            for (id, command, cwd) in entries {
                catalog.agents.push(Agent {
                    provider: Provider::from_command(&command[0]),
                    executable: find_in(&command[0], path),
                    id,
                    command,
                    cwd,
                    configured: true,
                });
            }
        }
        Err(error) => catalog.config_error = Some(error),
    }
    for provider in Provider::KNOWN {
        let covered = catalog
            .agents
            .iter()
            .any(|agent| agent.provider == provider);
        if let (false, Some(executable)) = (covered, find_in(provider.program(), path)) {
            catalog.agents.push(Agent {
                id: provider.program().to_owned(),
                provider,
                command: vec![provider.program().to_owned()],
                cwd: root.to_path_buf(),
                executable: Some(executable),
                configured: false,
            });
        }
    }
    catalog
}

/// `(id, command, cwd)` of each `workspace.agents` entry. A missing file is not an error: no agents.
fn configured(root: &Path) -> Result<Vec<(String, Vec<String>, PathBuf)>, String> {
    let path = root.join(".ai").join("chxchx-tech.toml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("No pude leer la configuración: {error}")),
    };
    let config: toml::Table = text
        .parse()
        .map_err(|error| format!("La configuración no es TOML válido: {error}"))?;
    let Some(entries) = config
        .get("workspace")
        .and_then(|workspace| workspace.get("agents"))
        .and_then(toml::Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut agents = Vec::new();
    for entry in entries.iter().filter_map(toml::Value::as_table) {
        let Some(id) = entry.get("id").and_then(toml::Value::as_str) else {
            continue;
        };
        // Studio runs agents by argument list; a shell string is not integrated.
        let Some(command) = entry.get("command").and_then(toml::Value::as_array) else {
            continue;
        };
        let command: Vec<String> = command
            .iter()
            .filter_map(toml::Value::as_str)
            .map(str::to_owned)
            .collect();
        if command.is_empty() {
            continue;
        }
        let cwd = entry
            .get("cwd")
            .and_then(toml::Value::as_str)
            .unwrap_or(".");
        // A configured directory outside the project is ignored: agents start inside it. `/etc` is not
        // "absolute" on Windows (no drive) but it still escapes, so the components are what count.
        let bytes = cwd.as_bytes();
        let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        let escapes = drive
            || cwd.starts_with(['/', '\\'])
            || cwd.split(['/', '\\']).any(|part| part == "..")
            || Path::new(cwd).components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            });
        let cwd = if escapes {
            root.to_path_buf()
        } else {
            root.join(cwd)
        };
        agents.push((id.to_owned(), command, cwd));
    }
    Ok(agents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project(name: &str, config: Option<&str>) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("chxchx-agents-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".ai")).unwrap();
        if let Some(config) = config {
            fs::write(root.join(".ai/chxchx-tech.toml"), config).unwrap();
        }
        root
    }

    fn bin(root: &Path, names: &[&str]) -> OsString {
        let folder = root.join("bin");
        fs::create_dir_all(&folder).unwrap();
        for name in names {
            let program = folder.join(if cfg!(windows) {
                format!("{name}.exe")
            } else {
                (*name).to_owned()
            });
            fs::write(&program, "").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        folder.into_os_string()
    }

    #[test]
    fn the_provider_comes_from_the_program_whatever_the_platform_spells() {
        assert_eq!(Provider::from_command("claude"), Provider::Claude);
        assert_eq!(
            Provider::from_command("C:\\npm\\claude.cmd"),
            Provider::Claude
        );
        assert_eq!(
            Provider::from_command("/usr/local/bin/Codex"),
            Provider::Codex
        );
        assert_eq!(Provider::from_command("opencode.exe"), Provider::OpenCode);
        assert_eq!(Provider::from_command("aider"), Provider::Other);
        assert!(Provider::OpenCode.has_visual_chat() && !Provider::Other.has_visual_chat());
    }

    #[test]
    fn configured_agents_report_whether_they_are_installed() {
        let root = project(
            "configured",
            Some(
                "[[workspace.agents]]\nid = \"claude\"\ncommand = [\"claude\"]\ncwd = \".\"\n\n[[workspace.agents]]\nid = \"revisor\"\ncommand = [\"codex\", \"--model\", \"x\"]\ncwd = \"sub\"\n",
            ),
        );
        let catalog = list_with(&root, &bin(&root, &["claude"]));

        assert_eq!(catalog.config_error, None);
        let claude = catalog.agents.iter().find(|a| a.id == "claude").unwrap();
        assert!(claude.configured && claude.available() && claude.provider == Provider::Claude);
        let revisor = catalog.agents.iter().find(|a| a.id == "revisor").unwrap();
        assert!(revisor.configured && !revisor.available() && revisor.provider == Provider::Codex);
        assert_eq!(revisor.cwd, root.join("sub"));
        assert_eq!(revisor.command, ["codex", "--model", "x"]);
    }

    #[test]
    fn installed_agents_nobody_configured_are_offered_once_each() {
        let root = project(
            "detected",
            Some("[[workspace.agents]]\nid = \"claude\"\ncommand = [\"claude\"]\n"),
        );
        let catalog = list_with(&root, &bin(&root, &["claude", "codex", "opencode"]));

        let ids: Vec<_> = catalog
            .agents
            .iter()
            .map(|a| (a.id.as_str(), a.configured))
            .collect();
        assert_eq!(
            ids,
            [("claude", true), ("codex", false), ("opencode", false)]
        );
        assert_eq!(catalog.agents[1].command, ["codex"]);
        assert!(catalog.agents.iter().all(Agent::available));
    }

    #[test]
    fn a_project_without_configuration_still_lists_what_is_installed() {
        let root = project("none", None);
        let catalog = list_with(&root, &bin(&root, &["opencode"]));

        assert_eq!(catalog.config_error, None);
        assert_eq!(catalog.agents.len(), 1);
        assert_eq!(catalog.agents[0].provider, Provider::OpenCode);
        assert!(list_with(&root, &OsString::new()).agents.is_empty());
    }

    #[test]
    fn a_broken_configuration_is_reported_and_does_not_hide_installed_agents() {
        let root = project("broken", Some("[[workspace.agents\nid = "));
        let catalog = list_with(&root, &bin(&root, &["claude"]));

        assert!(
            catalog
                .config_error
                .as_deref()
                .is_some_and(|e| e.contains("TOML"))
        );
        assert_eq!(catalog.agents.len(), 1);
    }

    #[test]
    fn an_agent_cannot_start_outside_the_project() {
        let root = project(
            "escape",
            Some(
                "[[workspace.agents]]\nid = \"a\"\ncommand = [\"claude\"]\ncwd = \"../fuera\"\n\n[[workspace.agents]]\nid = \"b\"\ncommand = [\"codex\"]\ncwd = \"/etc\"\n",
            ),
        );
        let catalog = list_with(&root, &OsString::new());

        assert!(catalog.agents.iter().all(|agent| agent.cwd == root));
    }

    #[test]
    fn drive_and_rooted_paths_do_not_escape_on_any_platform() {
        let root = project(
            "escape-windows",
            Some(
                "[[workspace.agents]]\nid = \"c\"\ncommand = [\"opencode\"]\ncwd = 'C:\\Windows'\n\n[[workspace.agents]]\nid = \"d\"\ncommand = [\"opencode\"]\ncwd = '\\raiz'\n\n[[workspace.agents]]\nid = \"e\"\ncommand = [\"opencode\"]\ncwd = 'sub\\..\\..\\fuera'\n\n[[workspace.agents]]\nid = \"ok\"\ncommand = [\"opencode\"]\ncwd = 'sub'\n",
            ),
        );
        let catalog = list_with(&root, &OsString::new());

        let cwd = |id: &str| {
            catalog
                .agents
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .cwd
                .clone()
        };
        assert_eq!(cwd("c"), root);
        assert_eq!(cwd("d"), root);
        assert_eq!(cwd("e"), root);
        assert_eq!(cwd("ok"), root.join("sub"));
    }
}
