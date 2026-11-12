use std::fs;
use std::path::Path;

pub fn validate_project_directory(root: &Path) -> Result<(), String> {
    let config_directory = root.join(".codex");
    match fs::symlink_metadata(&config_directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(".codex es un enlace simbólico; no se modifica".to_owned())
        }
        Ok(metadata) if !metadata.is_dir() => Err(".codex existe y no es un directorio".to_owned()),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("No se pudo inspeccionar .codex: {error}")),
    }
}

pub fn plan_project_config(
    root: &Path,
    desired_servers: &[(&str, Vec<String>)],
    refresh: bool,
) -> Result<Option<Vec<u8>>, String> {
    validate_project_directory(root)?;
    let target = root.join(".codex").join("config.toml");
    let current = match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(".codex/config.toml es un enlace simbólico".to_owned());
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(".codex/config.toml no es un archivo regular".to_owned());
        }
        Ok(_) => fs::read_to_string(&target)
            .map_err(|error| format!("No se pudo leer .codex/config.toml: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(format!(
                "No se pudo inspeccionar .codex/config.toml: {error}"
            ));
        }
    };
    let document = parse_config(&current)?;
    let configured = match document.get("mcp_servers") {
        Some(value) => value
            .as_table()
            .ok_or_else(|| "La tabla mcp_servers de Codex no es válida".to_owned())?,
        None => return plan_after(&current, desired_servers, refresh),
    };
    for (name, command) in desired_servers {
        let Some(existing) = configured.get(*name) else {
            continue;
        };
        let table = existing
            .as_table()
            .ok_or_else(|| format!("mcp_servers.{name} no es una tabla"))?;
        if table_matches(table, command) {
            continue;
        }
        if !refresh {
            return Err(format!(
                "mcp_servers.{name} ya existe con otra configuración; revisa Codex o usa `--refresh`"
            ));
        }
    }
    plan_after(&current, desired_servers, refresh)
}

fn plan_after(
    current: &str,
    desired_servers: &[(&str, Vec<String>)],
    refresh: bool,
) -> Result<Option<Vec<u8>>, String> {
    let mut updated = current.to_owned();
    let mut changed = false;
    for (name, command) in desired_servers {
        let document = parse_config(&updated)?;
        let existing = document
            .get("mcp_servers")
            .and_then(toml::Value::as_table)
            .and_then(|servers| servers.get(*name));
        if let Some(existing) = existing {
            let table = existing
                .as_table()
                .ok_or_else(|| format!("mcp_servers.{name} no es una tabla"))?;
            if table_matches(table, command) {
                continue;
            }
            if !refresh {
                return Err(format!(
                    "mcp_servers.{name} ya existe con otra configuración; revisa Codex o usa `--refresh`"
                ));
            }
            let Some((start, end)) = table_bounds(&updated, name) else {
                return Err(format!(
                    "No se puede reemplazar mcp_servers.{name} de forma segura porque usa una sintaxis TOML no compatible"
                ));
            };
            let expected = codex_block(name, command);
            updated.replace_range(start..end, &expected);
            changed = true;
        } else {
            if !updated.trim().is_empty() && !updated.ends_with("\n\n") {
                updated = updated.trim_end().to_owned() + "\n\n";
            }
            updated.push_str(&codex_block(name, command));
            changed = true;
        }
    }
    Ok(changed.then(|| updated.into_bytes()))
}

fn parse_config(source: &str) -> Result<toml::Value, String> {
    toml::from_str::<toml::Value>(source)
        .map_err(|error| format!(".codex/config.toml no es TOML válido: {error}"))
}

fn table_matches(existing: &toml::map::Map<String, toml::Value>, command: &[String]) -> bool {
    existing.get("command").and_then(toml::Value::as_str) == command.first().map(String::as_str)
        && existing
            .get("args")
            .and_then(toml::Value::as_array)
            .is_some_and(|arguments| {
                arguments.len() == command.len().saturating_sub(1)
                    && arguments
                        .iter()
                        .zip(&command[1..])
                        .all(|(value, expected)| value.as_str() == Some(expected))
            })
}

fn codex_block(name: &str, command: &[String]) -> String {
    let command_name = toml_string(&command[0]);
    let args = command[1..]
        .iter()
        .map(|argument| toml_string(argument))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[mcp_servers.{name}]\ncommand = {command_name}\nargs = [{args}]\n\n")
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

fn table_bounds(text: &str, name: &str) -> Option<(usize, usize)> {
    let mut start = None;
    let mut cursor = 0;
    for line in text.split_inclusive('\n') {
        if is_server_header(line, name) {
            start = Some(cursor);
        } else if start.is_some() && line.trim_start().starts_with('[') {
            return Some((start?, cursor));
        }
        cursor += line.len();
    }
    start.map(|start| (start, text.len()))
}

fn is_server_header(line: &str, name: &str) -> bool {
    let header = line.split('#').next().unwrap_or_default().trim();
    if !header.starts_with('[') || header.starts_with("[[") || !header.ends_with(']') {
        return false;
    }
    let probe = format!("{header}\nchxchx_probe = true");
    toml::from_str::<toml::Value>(&probe)
        .ok()
        .and_then(|document| {
            document
                .get("mcp_servers")?
                .as_table()?
                .get(name)?
                .as_table()?
                .get("chxchx_probe")?
                .as_bool()
        })
        == Some(true)
}
