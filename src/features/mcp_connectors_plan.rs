//! Adding and removing MCP connectors in a project: which file of which agent changes, shown
//! exactly before it is written. The files are the project-level ones each agent reads
//! (`.mcp.json`, `.codex/config.toml`, `opencode.json`); nothing is started and no secret is saved.
use crate::features::mcp_connectors::{
    Connector, connector, current_platform, valid_env_name, valid_server_name,
};
use crate::features::mcp_setup::McpClient;
use crate::features::project_changes::{apply_changes, missing_directories, resolve_root};
use crate::features::project_profiles::application_home;
use crate::features::project_setup::{ProjectFileChange, plan_file_change};
use crate::integrations::mcp_clients::find_executable;
use crate::integrations::mcp_codex;
use crate::integrations::mcp_files::{self, Merge, ServerSpec};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum ConnectorsChange {
    Add {
        names: Vec<String>,
        /// Values for the connector parameters (`db_path=./datos.db`).
        values: BTreeMap<String, String>,
    },
    Remove {
        names: Vec<String>,
    },
    /// A server that is not in the catalog.
    Custom {
        name: String,
        command: Vec<String>,
        env: Vec<String>,
    },
}

#[derive(Clone, Debug)]
pub struct ConnectorsPlan {
    pub project_root: String,
    pub clients: Vec<String>,
    pub actions: Vec<String>,
    pub warnings: Vec<String>,
    pub blocked_reason: Option<String>,
    root: PathBuf,
    directories: Vec<PathBuf>,
    files: Vec<ProjectFileChange>,
}

impl ConnectorsPlan {
    pub fn changes_state(&self) -> bool {
        !self.files.is_empty()
    }

    /// The files that would change, as `(relative path, new text)`; for showing the exact result.
    pub fn previews(&self) -> Vec<(String, String)> {
        self.files
            .iter()
            .map(|f| {
                (
                    f.relative.clone(),
                    String::from_utf8_lossy(&f.after).into_owned(),
                )
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConnectorsApplication {
    pub completed: Vec<String>,
    pub backup: Option<String>,
}

/// The file each agent reads for its project's MCP servers.
pub fn config_file(client: McpClient) -> &'static str {
    match client {
        McpClient::Claude => ".mcp.json",
        McpClient::Codex => ".codex/config.toml",
        McpClient::OpenCode => "opencode.json",
    }
}

/// The agents worth configuring here: the installed ones and those that already have a file.
pub fn default_clients(root: &Path) -> Vec<McpClient> {
    McpClient::parse("all")
        .unwrap_or_default()
        .into_iter()
        .filter(|c| {
            find_executable(c.executable()).is_some() || root.join(config_file(*c)).is_file()
        })
        .collect()
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    let path = root.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(m) if m.file_type().is_symlink() => {
            Err(format!("{relative} es un enlace simbólico; no se modifica"))
        }
        Ok(m) if !m.is_file() => Err(format!("{relative} no es un archivo regular")),
        Ok(_) => fs::read_to_string(&path).map_err(|e| format!("No se pudo leer {relative}: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("No se pudo inspeccionar {relative}: {e}")),
    }
}

fn merge_for(
    client: McpClient,
    current: &str,
    add: &[ServerSpec],
    remove: &[ServerSpec],
) -> Result<Merge, String> {
    match client {
        McpClient::Claude => mcp_files::claude_project(current, add, remove),
        McpClient::OpenCode => {
            mcp_files::opencode_project(current, add, remove, mcp_files::OpenCodeStyle::Flat)
        }
        McpClient::Codex => mcp_codex::merge_servers(current, add, remove),
    }
}

/// How a catalog connector becomes a server definition on this machine.
pub fn spec_for(c: &Connector, values: &BTreeMap<String, String>) -> Result<ServerSpec, String> {
    Ok(ServerSpec {
        name: c.name.clone(),
        command: c.command_for(cfg!(windows), values)?,
        env: c.required_env().map(|e| e.name.clone()).collect(),
    })
}

pub fn plan_connectors(
    path: &Path,
    change: &ConnectorsChange,
    clients: &[McpClient],
) -> Result<ConnectorsPlan, String> {
    let root = resolve_root(path)?;
    let (mut add, mut remove) = (Vec::new(), Vec::new());
    let mut warnings = Vec::new();
    let mut blocked_reason = None;
    match change {
        ConnectorsChange::Add { names, values } => {
            for name in names {
                let c = connector(name).ok_or_else(|| {
                    format!("Conector desconocido: {name}. Mira `connectors list`.")
                })?;
                if !c.supported_here() {
                    return Err(format!(
                        "{} no está disponible en {}.",
                        c.title,
                        current_platform()
                    ));
                }
                add.push(spec_for(c, values)?);
                for program in c.missing_requirements() {
                    warnings.push(format!("{}: falta `{program}` en el PATH; el agente no podrá iniciarlo hasta instalarlo.", c.title));
                }
                for env in c.required_env() {
                    if std::env::var_os(&env.name).is_none() {
                        warnings.push(format!("{}: define {} en tu entorno ({}) antes de abrir el agente; no se guarda en el proyecto.", c.title, env.name, env.description));
                    }
                }
                if !c.note.is_empty() {
                    warnings.push(format!("{}: {}", c.title, c.note));
                }
            }
            if !add.is_empty() {
                if clients.contains(&McpClient::Claude) {
                    warnings.push("Claude Code pide aprobar los servidores del proyecto la primera vez que lo abres (o ejecuta `claude mcp reset-project-choices`).".to_owned());
                }
                if clients.contains(&McpClient::Codex) {
                    warnings.push("Codex solo lee `.codex/config.toml` de proyectos en los que confías; márcalo como de confianza en Codex.".to_owned());
                }
            }
            if cfg!(windows) && add.iter().any(|s| s.command[0] == "cmd") {
                warnings.push("El archivo del proyecto lleva el comando de Windows (`cmd /c`); quien lo abra en macOS o Linux necesitará la forma sin `cmd`.".to_owned());
            }
        }
        ConnectorsChange::Remove { names } => {
            for name in names {
                let c = connector(name).ok_or_else(|| {
                    format!(
                        "`{name}` no está en el catálogo; quítalo a mano del archivo del agente."
                    )
                })?;
                // A connector with a parameter is matched by name, whatever value it was given.
                remove.push(spec_for(
                    c,
                    &c.params
                        .iter()
                        .map(|p| (p.name.clone(), "?".to_owned()))
                        .collect(),
                )?);
            }
        }
        ConnectorsChange::Custom { name, command, env } => {
            if !valid_server_name(name) {
                return Err(
                    "El nombre solo puede llevar letras, números, - y _ (hasta 48).".to_owned(),
                );
            }
            if command.is_empty() || command[0].trim().is_empty() {
                return Err("Falta el comando del servidor.".to_owned());
            }
            if let Some(bad) = env.iter().find(|e| !valid_env_name(e)) {
                return Err(format!(
                    "`{bad}` no es un nombre de variable de entorno válido."
                ));
            }
            add.push(ServerSpec {
                name: name.clone(),
                command: command.clone(),
                env: env.clone(),
            });
        }
    }
    if clients.is_empty() {
        blocked_reason = Some("No hay agentes donde configurar: instala Claude Code, Codex u OpenCode, o elige uno con --client.".to_owned());
    }
    let (mut actions, mut files) = (Vec::new(), Vec::new());
    for client in clients {
        let relative = config_file(*client);
        if *client == McpClient::Codex {
            mcp_codex::validate_project_directory(&root)?;
        }
        let current = read(&root, relative)?;
        let merge = merge_for(*client, &current, &add, &remove)?;
        let who = format!("{relative} ({})", client.label());
        for name in &merge.added {
            actions.push(format!("añadir `{name}` a {who}"));
        }
        for name in &merge.removed {
            actions.push(format!("quitar `{name}` de {who}"));
        }
        for name in &merge.kept {
            warnings.push(format!("`{name}` ya estaba en {who}"));
        }
        for name in &merge.conflicts {
            warnings.push(format!(
                "`{name}` ya existe en {who} con otra configuración: no se toca"
            ));
        }
        if let Some(text) = merge.text
            && let Some(file) = plan_file_change(&root, relative, text.into_bytes())?
        {
            files.push(file);
        }
    }
    if files.is_empty() && blocked_reason.is_none() {
        actions = vec!["todo está actualizado".to_owned()];
    }
    let directories = missing_directories(&root, &files)?;
    Ok(ConnectorsPlan {
        project_root: root.display().to_string(),
        clients: clients.iter().map(|c| c.label().to_owned()).collect(),
        actions,
        warnings,
        blocked_reason,
        root,
        directories,
        files,
    })
}

fn backup_key(root: &Path) -> String {
    let digest: String = Sha1::digest(root.display().to_string().as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "project".to_owned());
    format!("{name}-{}", &digest[..8])
}

/// Copies the files about to change into `backups-mcp/<project>/<time>/`, outside the project and
/// apart from `rollback`'s backups, which this must not become the "latest" of.
fn backup_configs(root: &Path, files: &[ProjectFileChange]) -> Result<Option<PathBuf>, String> {
    let existing: Vec<&ProjectFileChange> = files.iter().filter(|f| f.before.is_some()).collect();
    if existing.is_empty() {
        return Ok(None);
    }
    let home = application_home()
        .ok_or("No se pudo resolver CHXCHX_TECH_HOME para guardar el respaldo")?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let target = home
        .join("backups-mcp")
        .join(backup_key(root))
        .join(stamp.to_string());
    for file in existing {
        let destination = target.join(&file.relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("No se pudo crear el respaldo: {e}"))?;
        }
        fs::write(&destination, file.before.as_deref().unwrap_or_default())
            .map_err(|e| format!("No se pudo respaldar {}: {e}", file.relative))?;
    }
    Ok(Some(target))
}

pub fn apply_connectors(plan: &ConnectorsPlan) -> Result<ConnectorsApplication, String> {
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("No se aplicó: {reason}"));
    }
    if plan.files.is_empty() {
        return Ok(ConnectorsApplication::default());
    }
    let backup = backup_configs(&plan.root, &plan.files)?;
    apply_changes(&plan.root, &plan.directories, &plan.files, false)?;
    Ok(ConnectorsApplication {
        completed: plan.actions.clone(),
        backup: backup.map(|p| p.display().to_string()),
    })
}

/// Where a connector stands in one agent's project file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    Absent,
    Configured,
    /// A server of that name exists but is not what the catalog writes.
    Different,
}

#[derive(Clone, Debug)]
pub struct ConnectorState {
    pub name: String,
    pub standing: Vec<(McpClient, Standing)>,
    pub missing_requirements: Vec<String>,
    pub missing_env: Vec<String>,
    pub supported: bool,
}

/// What each connector looks like in the project, reading the files and nothing else.
pub fn connector_states(path: &Path, clients: &[McpClient]) -> Result<Vec<ConnectorState>, String> {
    let root = resolve_root(path)?;
    let mut texts = Vec::new();
    for client in clients {
        texts.push((
            *client,
            read(&root, config_file(*client)).unwrap_or_default(),
        ));
    }
    let mut states = Vec::new();
    for c in crate::features::mcp_connectors::connectors() {
        let placeholder: BTreeMap<String, String> = c
            .params
            .iter()
            .map(|p| (p.name.clone(), "?".to_owned()))
            .collect();
        let spec = spec_for(c, &placeholder)?;
        let mut standing = Vec::new();
        for (client, text) in &texts {
            let merge = merge_for(*client, text, std::slice::from_ref(&spec), &[]).ok();
            standing.push((
                *client,
                match merge {
                    Some(m) if !m.kept.is_empty() => Standing::Configured,
                    // With a parameter the value is the person's: any entry of that name is ours.
                    Some(m) if !m.conflicts.is_empty() && !c.params.is_empty() => {
                        Standing::Configured
                    }
                    Some(m) if !m.conflicts.is_empty() => Standing::Different,
                    _ => Standing::Absent,
                },
            ));
        }
        states.push(ConnectorState {
            name: c.name.clone(),
            standing,
            missing_requirements: c.missing_requirements(),
            missing_env: c
                .required_env()
                .filter(|e| std::env::var_os(&e.name).is_none())
                .map(|e| e.name.clone())
                .collect(),
            supported: c.supported_here(),
        });
    }
    Ok(states)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "chxchx-conn-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const ALL: [McpClient; 3] = [McpClient::Claude, McpClient::Codex, McpClient::OpenCode];

    fn add(names: &[&str]) -> ConnectorsChange {
        ConnectorsChange::Add {
            names: names.iter().map(|n| (*n).to_owned()).collect(),
            values: BTreeMap::new(),
        }
    }

    #[test]
    fn one_connector_reaches_the_file_of_every_agent_and_nothing_is_written_by_planning() {
        let dir = project("add");
        let plan = plan_connectors(&dir, &add(&["excel"]), &ALL).unwrap();
        assert_eq!(plan.files.len(), 3);
        for file in [".mcp.json", ".codex/config.toml", "opencode.json"] {
            assert!(
                plan.actions.iter().any(|a| a.contains(file)),
                "{file}: {:?}",
                plan.actions
            );
            assert!(!dir.join(file).exists(), "a plan writes nothing");
        }
        let previews = plan.previews();
        assert!(previews.iter().all(|(_, text)| text.contains("excel")));
    }

    #[test]
    fn applying_writes_the_files_and_a_second_plan_has_nothing_left_to_do() {
        let dir = project("apply");
        let plan = plan_connectors(&dir, &add(&["excel", "word"]), &ALL).unwrap();
        let done = apply_connectors(&plan).unwrap();
        assert_eq!(done.completed.len(), 6);
        let claude = fs::read_to_string(dir.join(".mcp.json")).unwrap();
        assert!(
            claude.contains("@negokaz/excel-mcp-server")
                && claude.contains("office-word-mcp-server")
        );
        let again = plan_connectors(&dir, &add(&["excel", "word"]), &ALL).unwrap();
        assert!(!again.changes_state() && again.actions == ["todo está actualizado"]);
    }

    #[test]
    fn a_secret_is_only_ever_named_never_written_and_its_absence_is_warned() {
        let dir = project("secret");
        let plan = plan_connectors(&dir, &add(&["postgres"]), &ALL).unwrap();
        for (_, text) in plan.previews() {
            assert!(text.contains("DATABASE_URI"));
            assert!(
                !text.contains("postgresql://") && !text.contains("clave"),
                "{text}"
            );
        }
        if std::env::var_os("DATABASE_URI").is_none() {
            assert!(
                plan.warnings
                    .iter()
                    .any(|w| w.contains("DATABASE_URI") && w.contains("no se guarda"))
            );
        }
    }

    #[test]
    fn a_connector_with_a_parameter_takes_its_value_and_asks_for_it_when_missing() {
        let dir = project("param");
        assert!(
            plan_connectors(&dir, &add(&["sqlite"]), &ALL)
                .unwrap_err()
                .contains("db_path")
        );
        let change = ConnectorsChange::Add {
            names: vec!["sqlite".into()],
            values: BTreeMap::from([("db_path".to_owned(), "./datos.db".to_owned())]),
        };
        let plan = plan_connectors(&dir, &change, &[McpClient::Claude]).unwrap();
        assert!(plan.previews()[0].1.contains("./datos.db"));
    }

    #[test]
    fn what_the_person_already_has_is_respected_and_removal_takes_only_ours() {
        let dir = project("respect");
        // Ours (applied on a clean folder, so nothing needs saving) and the person's own, different.
        apply_connectors(&plan_connectors(&dir, &add(&["word"]), &[McpClient::Claude]).unwrap())
            .unwrap();
        let mut file: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".mcp.json")).unwrap()).unwrap();
        file["mcpServers"]["excel"] = serde_json::json!({"command": "mio"});
        fs::write(dir.join(".mcp.json"), file.to_string()).unwrap();
        // Planning only: applying over an existing file would save a copy in the real home.
        let plan = plan_connectors(&dir, &add(&["excel", "word"]), &[McpClient::Claude]).unwrap();
        assert!(
            plan.warnings
                .iter()
                .any(|w| w.contains("`excel`") && w.contains("otra configuración"))
        );
        assert!(
            !plan.changes_state(),
            "excel is theirs and word is already there"
        );
        let rm = ConnectorsChange::Remove {
            names: vec!["excel".into(), "word".into()],
        };
        let plan = plan_connectors(&dir, &rm, &[McpClient::Claude]).unwrap();
        assert!(plan.actions.iter().any(|a| a.contains("quitar `word`")));
        assert!(
            plan.warnings.iter().any(|w| w.contains("`excel`")),
            "their own server is not removed"
        );
    }

    #[test]
    fn unknown_connectors_bad_custom_servers_and_no_agents_are_refused_with_a_reason() {
        let dir = project("refuse");
        assert!(
            plan_connectors(&dir, &add(&["no-existe"]), &ALL)
                .unwrap_err()
                .contains("desconocido")
        );
        let bad = |name: &str, command: &[&str], env: &[&str]| ConnectorsChange::Custom {
            name: name.into(),
            command: command.iter().map(|s| (*s).to_owned()).collect(),
            env: env.iter().map(|s| (*s).to_owned()).collect(),
        };
        assert!(plan_connectors(&dir, &bad("a b", &["x"], &[]), &ALL).is_err());
        assert!(plan_connectors(&dir, &bad("ok", &[], &[]), &ALL).is_err());
        assert!(plan_connectors(&dir, &bad("ok", &["x"], &["minuscula"]), &ALL).is_err());
        let none = plan_connectors(&dir, &add(&["excel"]), &[]).unwrap();
        assert!(none.blocked_reason.is_some() && apply_connectors(&none).is_err());
    }

    #[test]
    fn a_custom_server_is_configured_like_a_catalog_one() {
        let dir = project("custom");
        let change = ConnectorsChange::Custom {
            name: "mi-api".into(),
            command: vec!["npx".into(), "-y".into(), "mi-paquete".into()],
            env: vec!["MI_TOKEN".into()],
        };
        let plan = plan_connectors(&dir, &change, &[McpClient::OpenCode]).unwrap();
        let text = &plan.previews()[0].1;
        assert!(
            text.contains("mi-api") && text.contains("{env:MI_TOKEN}") && !text.contains("secreto")
        );
    }

    #[test]
    fn the_state_of_each_connector_follows_the_files() {
        let dir = project("state");
        let plan = plan_connectors(&dir, &add(&["excel"]), &[McpClient::Claude]).unwrap();
        apply_connectors(&plan).unwrap();
        let states = connector_states(&dir, &ALL).unwrap();
        let excel = states.iter().find(|s| s.name == "excel").unwrap();
        assert_eq!(excel.standing[0], (McpClient::Claude, Standing::Configured));
        assert_eq!(excel.standing[1], (McpClient::Codex, Standing::Absent));
        let word = states.iter().find(|s| s.name == "word").unwrap();
        assert!(word.standing.iter().all(|(_, s)| *s == Standing::Absent));
        assert_eq!(
            states.len(),
            crate::features::mcp_connectors::connectors().len()
        );
    }
}
