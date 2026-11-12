use crate::features::project_config_template::memory_project_name;
use crate::features::project_overview::inspect_project;
use crate::integrations::basic_memory;
use crate::integrations::mcp_clients::{self, CommandOutput};
use crate::integrations::mcp_codex;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CLIENTS: [McpClient; 3] = [McpClient::Claude, McpClient::Codex, McpClient::OpenCode];
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpClient {
    Claude,
    Codex,
    OpenCode,
}

impl McpClient {
    pub fn parse(value: &str) -> Result<Vec<Self>, String> {
        match value.to_ascii_lowercase().as_str() {
            "all" => Ok(CLIENTS.to_vec()),
            "claude" => Ok(vec![Self::Claude]),
            "codex" => Ok(vec![Self::Codex]),
            "opencode" => Ok(vec![Self::OpenCode]),
            _ => Err(format!("Cliente MCP no soportado: {value}")),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::OpenCode => "OpenCode",
        }
    }

    fn executable(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
        }
    }
}

#[derive(Clone)]
struct FileSnapshot {
    path: PathBuf,
    before: Option<Vec<u8>>,
}

#[derive(Clone)]
struct ClientPlan {
    client: McpClient,
    snapshots: Vec<FileSnapshot>,
    codex_after: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct McpIntegrationPlan {
    pub project_root: String,
    pub client_labels: Vec<String>,
    pub refresh: bool,
    pub actions: Vec<String>,
    pub warnings: Vec<String>,
    pub blocked_reason: Option<String>,
    root: PathBuf,
    memory_project: String,
    basic_memory_executable: String,
    clients: Vec<ClientPlan>,
}

#[derive(Clone, Debug, Default)]
pub struct McpIntegrationResult {
    pub completed: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
struct OpenCodeExample {
    #[serde(rename = "$schema")]
    schema: &'static str,
    mcp: OpenCodeExampleMcp,
}

#[derive(Serialize)]
struct OpenCodeExampleMcp {
    servers: std::collections::BTreeMap<String, OpenCodeExampleServer>,
}

#[derive(Serialize)]
struct OpenCodeExampleServer {
    #[serde(rename = "type")]
    kind: &'static str,
    command: Vec<String>,
}

pub fn opencode_example_contents(root: &Path, project_name: &str) -> Result<String, String> {
    let basic_memory = basic_memory::command_name().unwrap_or_else(|| "basic-memory".to_owned());
    let memory = memory_project_name(root, project_name);
    let servers = server_commands(&basic_memory, &memory)
        .into_iter()
        .map(|(name, command)| {
            (
                name.to_owned(),
                OpenCodeExampleServer {
                    kind: "local",
                    command,
                },
            )
        })
        .collect();
    let example = OpenCodeExample {
        schema: "https://opencode.ai/config.json",
        mcp: OpenCodeExampleMcp { servers },
    };
    serde_json::to_string_pretty(&example)
        .map(|contents| contents + "\n")
        .map_err(|error| format!("No se pudo generar el ejemplo MCP de OpenCode: {error}"))
}

pub fn plan_mcp_integration(
    path: &Path,
    clients: &[McpClient],
    refresh: bool,
) -> Result<McpIntegrationPlan, String> {
    let overview = inspect_project(path)?;
    let root = PathBuf::from(&overview.project.root);
    let memory_path = root.join(".ai").join("memory");
    let mut warnings = Vec::new();
    let mut blocked_reason = None;

    let client_path = basic_memory::command_name();
    let basic_memory_executable = client_path
        .as_ref()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "basic-memory".to_owned());
    if client_path.is_none() {
        blocked_reason =
            Some("Basic Memory no está instalado. Instálalo antes de configurar MCP.".to_owned());
    }
    if find_executable("serena").is_none() && blocked_reason.is_none() {
        blocked_reason = Some("Serena no está instalado.".to_owned());
    }
    if !memory_path.is_dir() && blocked_reason.is_none() {
        blocked_reason = Some(
            "No existe .ai/memory. Aplica primero `init --minimal` para preparar el proyecto."
                .to_owned(),
        );
    }

    let mut client_plans = Vec::new();
    for client in clients {
        if blocked_reason.is_some() {
            break;
        }
        if *client == McpClient::Codex
            && let Err(error) = mcp_codex::validate_project_directory(&root)
        {
            warnings.push(format!("{} omitido: {error}", client.label()));
            continue;
        }
        if find_executable(client.executable()).is_none() {
            warnings.push(format!(
                "{} omitido: no se encontró '{}' en PATH.",
                client.label(),
                client.executable()
            ));
            continue;
        }
        let snapshots = match client_snapshots(*client, &root) {
            Ok(snapshots) => snapshots,
            Err(error) => {
                warnings.push(format!("{} omitido: {error}", client.label()));
                continue;
            }
        };
        let codex_after = if *client == McpClient::Codex {
            let commands = server_commands(
                &basic_memory_executable,
                &memory_project_name(&root, &overview.project.name),
            );
            match mcp_codex::plan_project_config(&root, &commands, refresh) {
                Ok(after) => after,
                Err(error) => {
                    warnings.push(format!("Codex omitido: {error}"));
                    continue;
                }
            }
        } else {
            None
        };
        client_plans.push(ClientPlan {
            client: *client,
            snapshots,
            codex_after,
        });
    }

    if blocked_reason.is_none() && client_plans.is_empty() {
        blocked_reason =
            Some("No hay clientes MCP seleccionados que puedan configurarse.".to_owned());
    }

    if client_plans
        .iter()
        .any(|plan| matches!(plan.client, McpClient::Claude | McpClient::OpenCode))
        && blocked_reason.is_none()
    {
        warnings.push(
            "Al aplicar, Claude Code/OpenCode consultarán el estado MCP; el cliente puede comprobar o iniciar servidores locales.".to_owned(),
        );
    }

    let mut actions = Vec::new();
    if blocked_reason.is_none() {
        actions.push(format!(
            "asegurar el proyecto de memoria `{}` en Basic Memory",
            memory_project_name(&root, &overview.project.name)
        ));
        for plan in &client_plans {
            if plan.client == McpClient::Codex {
                if plan.codex_after.is_some() {
                    actions
                        .push("actualizar .codex/config.toml con basic-memory y serena".to_owned());
                } else {
                    actions.push("los servidores MCP de Codex ya están configurados".to_owned());
                }
            } else if refresh {
                actions.push(format!(
                    "consultar y reemplazar basic-memory/serena con el CLI oficial de {}",
                    plan.client.label()
                ));
            } else {
                actions.push(format!(
                    "consultar y añadir servidores faltantes con el CLI oficial de {}",
                    plan.client.label()
                ));
            }
        }
    } else {
        actions.clear();
    }

    let memory_project = memory_project_name(&root, &overview.project.name);
    Ok(McpIntegrationPlan {
        project_root: root.display().to_string(),
        client_labels: clients
            .iter()
            .map(|client| client.label().to_owned())
            .collect(),
        refresh,
        actions,
        warnings,
        blocked_reason,
        root,
        memory_project,
        basic_memory_executable,
        clients: client_plans,
    })
}

pub fn apply_mcp_integration(plan: &McpIntegrationPlan) -> Result<McpIntegrationResult, String> {
    if let Some(reason) = &plan.blocked_reason {
        return Err(format!("Integración MCP bloqueada: {reason}"));
    }
    if !plan.root.join(".ai").join("memory").is_dir() {
        return Err(".ai/memory cambió después de la vista previa".to_owned());
    }
    for client in &plan.clients {
        if client.client == McpClient::Codex {
            mcp_codex::validate_project_directory(&plan.root)?;
        }
        check_snapshots(&client.snapshots)?;
    }
    basic_memory::ensure_project(
        &plan.basic_memory_executable,
        &plan.memory_project,
        &plan.root.join(".ai").join("memory"),
        &plan.root,
    )?;

    let mut result = McpIntegrationResult {
        completed: vec![format!(
            "proyecto Basic Memory `{}` disponible",
            plan.memory_project
        )],
        warnings: plan.warnings.clone(),
    };
    for client in &plan.clients {
        let operation = if client.client == McpClient::Codex {
            apply_codex_client(plan, client)
        } else {
            apply_cli_client(plan, client)
        };
        match operation {
            Ok(message) => result.completed.extend(message),
            Err(error) => result
                .warnings
                .push(format!("{}: {error}", client.client.label())),
        }
    }
    Ok(result)
}

fn apply_cli_client(plan: &McpIntegrationPlan, client: &ClientPlan) -> Result<Vec<String>, String> {
    check_snapshots(&client.snapshots)?;
    let executable = client.client.executable();
    let list = vec![executable.to_owned(), "mcp".to_owned(), "list".to_owned()];
    let mut changed = false;
    let operation = (|| {
        let listed = run_cli(&list, &plan.root)?;
        if listed.status != 0 {
            return Err(command_error("No se pudo consultar MCP", &listed));
        }
        let output = format!("{}\n{}", listed.stdout, listed.stderr);
        let mut completed = Vec::new();
        let mut attempted = Vec::new();
        for (name, command) in server_commands(&plan.basic_memory_executable, &plan.memory_project)
        {
            let existing = contains_server(&output, name);
            if existing && !plan.refresh {
                completed.push(format!("{name} ya estaba configurado"));
                continue;
            }
            attempted.push(name.to_owned());
            if existing {
                changed = true;
                let remove = vec![
                    executable.to_owned(),
                    "mcp".to_owned(),
                    "remove".to_owned(),
                    name.to_owned(),
                ];
                let removed = run_cli(&remove, &plan.root)?;
                if removed.status != 0 {
                    return Err(format!(
                        "No se pudo reemplazar `{name}`: {}",
                        command_error("remove", &removed)
                    ));
                }
            }
            changed = true;
            let add = client_add_command(client.client, name, &command);
            let added = run_cli(&add, &plan.root)?;
            if added.status != 0 {
                return Err(format!(
                    "No se pudo añadir `{name}`: {}",
                    command_error("add", &added)
                ));
            }
            completed.push(format!("{name} configurado"));
        }
        if attempted.is_empty() {
            return Ok(completed);
        }
        let verified = run_cli(&list, &plan.root)?;
        if verified.status != 0 {
            return Err(command_error("No se pudo verificar MCP", &verified));
        }
        let verified_output = format!("{}\n{}", verified.stdout, verified.stderr);
        let missing = attempted
            .iter()
            .filter(|name| !contains_server(&verified_output, name))
            .map(String::as_str)
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "El cliente no confirmó los servidores: {}",
                missing.join(", ")
            ));
        }
        Ok(completed)
    })();
    match operation {
        Ok(completed) => Ok(completed),
        Err(error) if changed => restore_snapshots(&client.snapshots, error),
        Err(error) => Err(error),
    }
}

fn apply_codex_client(
    plan: &McpIntegrationPlan,
    client: &ClientPlan,
) -> Result<Vec<String>, String> {
    mcp_codex::validate_project_directory(&plan.root)?;
    check_snapshots(&client.snapshots)?;
    let Some(after) = &client.codex_after else {
        return Ok(vec!["servidores MCP ya configurados".to_owned()]);
    };
    let target = plan.root.join(".codex").join("config.toml");
    let parent = target
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta .codex".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("No se pudo crear .codex: {error}"))?;
    if let Err(error) = write_atomic(&target, after) {
        return restore_snapshots(&client.snapshots, error);
    }
    Ok(vec![".codex/config.toml actualizado".to_owned()])
}

fn server_commands(basic_memory: &str, memory_project: &str) -> [(&'static str, Vec<String>); 2] {
    [
        (
            "basic-memory",
            vec![
                basic_memory.to_owned(),
                "mcp".to_owned(),
                "--project".to_owned(),
                memory_project.to_owned(),
            ],
        ),
        (
            "serena",
            vec![
                "serena".to_owned(),
                "start-mcp-server".to_owned(),
                "--context".to_owned(),
                "ide-assistant".to_owned(),
                "--project-from-cwd".to_owned(),
            ],
        ),
    ]
}

pub(crate) fn expected_server_commands(
    root: &Path,
    project_name: &str,
) -> [(&'static str, Vec<String>); 2] {
    let executable = basic_memory::command_name().unwrap_or_else(|| "basic-memory".to_owned());
    let memory = memory_project_name(root, project_name);
    server_commands(&executable, &memory)
}

fn client_add_command(client: McpClient, name: &str, command: &[String]) -> Vec<String> {
    match client {
        McpClient::Claude => [
            vec!["claude", "mcp", "add", "--scope", "local", name, "--"],
            command.iter().map(String::as_str).collect::<Vec<_>>(),
        ]
        .concat()
        .into_iter()
        .map(str::to_owned)
        .collect(),
        McpClient::Codex | McpClient::OpenCode => {
            let executable = client.executable();
            let mut result = vec![
                executable.to_owned(),
                "mcp".to_owned(),
                "add".to_owned(),
                name.to_owned(),
                "--".to_owned(),
            ];
            result.extend_from_slice(command);
            result
        }
    }
}

fn client_snapshots(client: McpClient, root: &Path) -> Result<Vec<FileSnapshot>, String> {
    let paths = match client {
        McpClient::Claude => {
            let home = std::env::home_dir()
                .ok_or_else(|| "No se pudo resolver el directorio de usuario".to_owned())?;
            vec![home.join(".claude.json")]
        }
        McpClient::Codex => vec![root.join(".codex").join("config.toml")],
        McpClient::OpenCode => vec![root.join("opencode.json"), root.join("opencode.jsonc")],
    };
    paths.into_iter().map(snapshot_file).collect()
}

fn snapshot_file(path: PathBuf) -> Result<FileSnapshot, String> {
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "No se modifica un archivo MCP enlazado: {}",
            path.display()
        )),
        Ok(metadata) if !metadata.is_file() => Err(format!(
            "El destino MCP no es archivo regular: {}",
            path.display()
        )),
        Ok(_) => Ok(FileSnapshot {
            before: Some(
                fs::read(&path)
                    .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?,
            ),
            path,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(FileSnapshot { path, before: None })
        }
        Err(error) => Err(format!("No se pudo inspeccionar un archivo MCP: {error}")),
    }
}

fn check_snapshots(snapshots: &[FileSnapshot]) -> Result<(), String> {
    for snapshot in snapshots {
        if fs::symlink_metadata(&snapshot.path)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(format!(
                "{} ahora es un enlace simbólico",
                snapshot.path.display()
            ));
        }
        let current = match fs::read(&snapshot.path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(format!(
                    "No se pudo releer {}: {error}",
                    snapshot.path.display()
                ));
            }
        };
        if current != snapshot.before {
            return Err(format!(
                "{} cambió desde la vista previa",
                snapshot.path.display()
            ));
        }
    }
    Ok(())
}

fn restore_snapshots<T>(snapshots: &[FileSnapshot], reason: String) -> Result<T, String> {
    let mut errors = Vec::new();
    for snapshot in snapshots.iter().rev() {
        if fs::symlink_metadata(&snapshot.path)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            errors.push(format!(
                "No se restauró un archivo MCP que ahora es un enlace: {}",
                snapshot.path.display()
            ));
            continue;
        }
        let restore = match &snapshot.before {
            Some(bytes) => write_atomic(&snapshot.path, bytes),
            None => match fs::remove_file(&snapshot.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!(
                    "No se pudo quitar {}: {error}",
                    snapshot.path.display()
                )),
            },
        };
        if let Err(error) = restore {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Err(format!("{reason}; se restauró la configuración previa"))
    } else {
        Err(format!(
            "{reason}; recuperación incompleta: {}",
            errors.join("; ")
        ))
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!(
                "No se reemplaza un archivo MCP enlazado: {}",
                path.display()
            ));
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(format!(
                "No se reemplaza un destino MCP no regular: {}",
                path.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "No se pudo inspeccionar {}: {error}",
                path.display()
            ));
        }
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("No se resolvió el directorio de {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear {}: {error}", parent.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = parent.join(format!(".chxchx-mcp-{}-{nonce}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("No se pudo preparar {}: {error}", path.display()))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudo escribir {}: {error}", path.display()));
    }
    drop(file);
    if path.exists()
        && let Err(error) = fs::remove_file(path)
    {
        let _ = fs::remove_file(&temp);
        return Err(format!("No se pudo reemplazar {}: {error}", path.display()));
    }
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("No se pudo guardar {}: {error}", path.display())
    })
}

fn contains_server(output: &str, name: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    let target = name.to_ascii_lowercase();
    lower.match_indices(&target).any(|(start, _)| {
        let before = lower[..start].chars().next_back();
        let after = lower[start + target.len()..].chars().next();
        !before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '-')
            && !after.is_some_and(|character| character.is_ascii_alphanumeric() || character == '-')
    })
}

fn run_cli(command: &[String], cwd: &Path) -> Result<CommandOutput, String> {
    mcp_clients::run(command, cwd)
}

fn command_error(context: &str, output: &CommandOutput) -> String {
    let detail = if output.stderr.trim().is_empty() {
        output.stdout.trim()
    } else {
        output.stderr.trim()
    };
    if detail.is_empty() {
        format!("{context} (código {})", output.status)
    } else {
        format!(
            "{context} (código {}): {}",
            output.status,
            detail.chars().take(500).collect::<String>()
        )
    }
}

fn find_executable(name: &str) -> Option<PathBuf> {
    mcp_clients::find_executable(name)
}
