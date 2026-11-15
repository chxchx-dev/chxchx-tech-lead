use crate::features::mcp_setup::expected_server_commands;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct McpDiagnostic {
    pub client: String,
    pub server: String,
    pub status: &'static str,
    pub scope: String,
    pub detail: String,
}

#[derive(Clone)]
struct JsonRead {
    value: Value,
    error: Option<&'static str>,
}

#[derive(Clone)]
struct TomlRead {
    value: toml::Value,
    error: Option<&'static str>,
}

#[derive(Clone, Copy)]
enum Entry<'a> {
    Json(&'a Value),
    Toml(&'a toml::Value),
}

pub fn diagnose_project_mcp(root: &Path, project_name: &str) -> Result<Vec<McpDiagnostic>, String> {
    let commands = expected_server_commands(root, project_name);
    let home = std::env::home_dir().ok_or_else(|| {
        "No se pudo resolver el directorio de usuario para el diagnóstico".to_owned()
    })?;
    let claude_config = read_json(&claude_config_path(&home));
    let codex_project = read_toml(&root.join(".codex").join("config.toml"));
    let codex_user = read_toml(&codex_user_config_path(&home));
    let claude_project_shared = read_json(&root.join(".mcp.json"));

    let root_key = root.display().to_string();
    let claude_project = mapping_at(Some(&claude_config.value), "projects")
        .and_then(|projects| projects.get(&root_key))
        .and_then(|project| mapping_at(Some(project), "mcpServers"));
    let claude_global = mapping_at(Some(&claude_config.value), "mcpServers");
    let claude_team = mapping_at(Some(&claude_project_shared.value), "mcpServers");
    let codex_project_servers = mapping_at_toml(Some(&codex_project.value), "mcp_servers");
    let codex_user_servers = mapping_at_toml(Some(&codex_user.value), "mcp_servers");

    let mut diagnostics = Vec::new();
    for (name, command) in commands {
        diagnostics.push(diagnose_entry(
            "Claude Code",
            name,
            &command,
            &[
                (
                    "local",
                    claude_project
                        .and_then(|servers| servers.get(name))
                        .map(Entry::Json),
                    claude_config.error,
                ),
                (
                    "proyecto compartido",
                    claude_team
                        .and_then(|servers| servers.get(name))
                        .map(Entry::Json),
                    claude_project_shared.error,
                ),
                (
                    "usuario/global",
                    claude_global
                        .and_then(|servers| servers.get(name))
                        .map(Entry::Json),
                    claude_config.error,
                ),
            ],
        ));
        diagnostics.push(diagnose_entry(
            "Codex",
            name,
            &command,
            &[
                (
                    "proyecto",
                    codex_project_servers
                        .and_then(|servers| servers.get(name))
                        .map(Entry::Toml),
                    codex_project.error,
                ),
                (
                    "usuario/global",
                    codex_user_servers
                        .and_then(|servers| servers.get(name))
                        .map(Entry::Toml),
                    codex_user.error,
                ),
            ],
        ));
    }
    Ok(diagnostics)
}

fn diagnose_entry(
    client: &str,
    name: &str,
    expected: &[String],
    scopes: &[(&str, Option<Entry<'_>>, Option<&'static str>)],
) -> McpDiagnostic {
    for (scope, entry, _error) in scopes {
        let Some(entry) = entry else { continue };
        if !entry_matches(*entry, expected) {
            return McpDiagnostic {
                client: client.to_owned(),
                server: name.to_owned(),
                status: "ERROR",
                scope: (*scope).to_owned(),
                detail: "configuración distinta a la esperada para este proyecto".to_owned(),
            };
        }
        if *scope == "usuario/global" {
            return McpDiagnostic {
                client: client.to_owned(),
                server: name.to_owned(),
                status: "AVISO",
                scope: (*scope).to_owned(),
                detail: "solo está configurado globalmente; puede compartirse con otros proyectos"
                    .to_owned(),
            };
        }
        let detail = if *scope == "proyecto compartido" {
            "configuración válida; puede requerir aprobación del cliente"
        } else {
            "comando y argumentos corresponden al proyecto"
        };
        return McpDiagnostic {
            client: client.to_owned(),
            server: name.to_owned(),
            status: "OK",
            scope: (*scope).to_owned(),
            detail: detail.to_owned(),
        };
    }
    if let Some(error) = scopes.iter().find_map(|(_, _, error)| *error) {
        return McpDiagnostic {
            client: client.to_owned(),
            server: name.to_owned(),
            status: "ERROR",
            scope: "configuración".to_owned(),
            detail: error.to_owned(),
        };
    }
    McpDiagnostic {
        client: client.to_owned(),
        server: name.to_owned(),
        status: "FALTA",
        scope: "—".to_owned(),
        detail: format!(
            "ejecuta `chxchx-tech integrate --client {}`",
            if client == "Claude Code" {
                "claude"
            } else {
                "codex"
            }
        ),
    }
}

fn entry_matches(entry: Entry<'_>, expected: &[String]) -> bool {
    match entry {
        Entry::Json(value) => {
            let Some(table) = value.as_object() else {
                return false;
            };
            matches_json(table, expected)
        }
        Entry::Toml(value) => value
            .as_table()
            .is_some_and(|table| matches_toml(table, expected)),
    }
}

fn matches_json(table: &Map<String, Value>, expected: &[String]) -> bool {
    let command = table.get("command").and_then(Value::as_str);
    let args = table
        .get("args")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    command == expected.first().map(String::as_str)
        && args.len() == expected.len().saturating_sub(1)
        && args
            .iter()
            .zip(&expected[1..])
            .all(|(value, expected)| value.as_str() == Some(expected))
}

fn matches_toml(table: &toml::map::Map<String, toml::Value>, expected: &[String]) -> bool {
    table.get("command").and_then(toml::Value::as_str) == expected.first().map(String::as_str)
        && table
            .get("args")
            .and_then(toml::Value::as_array)
            .is_some_and(|args| {
                args.len() == expected.len().saturating_sub(1)
                    && args
                        .iter()
                        .zip(&expected[1..])
                        .all(|(value, expected)| value.as_str() == Some(expected))
            })
}

fn read_json(path: &Path) -> JsonRead {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return JsonRead {
                value: Value::Object(Map::new()),
                error: None,
            };
        }
        Err(_) => {
            return JsonRead {
                value: Value::Object(Map::new()),
                error: Some("no se pudo leer o parsear"),
            };
        }
    };
    let value = match serde_json::from_str::<Value>(&contents) {
        Ok(value) if value.is_object() => value,
        Ok(_) => {
            return JsonRead {
                value: Value::Object(Map::new()),
                error: Some("raíz inválida"),
            };
        }
        Err(_) => {
            return JsonRead {
                value: Value::Object(Map::new()),
                error: Some("no se pudo leer o parsear"),
            };
        }
    };
    JsonRead { value, error: None }
}

fn read_toml(path: &Path) -> TomlRead {
    let empty = || TomlRead {
        value: toml::Value::Table(toml::map::Map::new()),
        error: None,
    };
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return empty(),
        Err(_) => {
            return TomlRead {
                value: toml::Value::Table(toml::map::Map::new()),
                error: Some("no se pudo leer o parsear"),
            };
        }
    };
    match toml::from_str::<toml::Value>(&contents) {
        Ok(value) => TomlRead { value, error: None },
        Err(_) => TomlRead {
            value: toml::Value::Table(toml::map::Map::new()),
            error: Some("no se pudo leer o parsear"),
        },
    }
}

fn mapping_at<'a>(value: Option<&'a Value>, key: &str) -> Option<&'a Map<String, Value>> {
    value
        .and_then(Value::as_object)
        .and_then(|object| object.get(key))
        .and_then(Value::as_object)
}

fn mapping_at_toml<'a>(
    value: Option<&'a toml::Value>,
    key: &str,
) -> Option<&'a toml::map::Map<String, toml::Value>> {
    value
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get(key))
        .and_then(toml::Value::as_table)
}

fn claude_config_path(home: &Path) -> PathBuf {
    let config_dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .map(expand_home)
        .unwrap_or_else(|| home.join(".claude"));
    if config_dir.file_name().is_some_and(|name| name == ".claude") {
        config_dir.parent().unwrap_or(home).join(".claude.json")
    } else {
        config_dir.join(".claude.json")
    }
}

fn codex_user_config_path(home: &Path) -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .map(expand_home)
        .unwrap_or_else(|| home.join(".codex"))
        .join("config.toml")
}

fn expand_home(path: PathBuf) -> PathBuf {
    if path == Path::new("~") {
        return std::env::home_dir().unwrap_or(path);
    }
    let value = path.to_string_lossy();
    if let Some(rest) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
        && let Some(home) = std::env::home_dir()
    {
        return home.join(rest);
    }
    path
}
