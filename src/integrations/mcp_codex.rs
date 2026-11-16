use super::mcp_files::{Merge, ServerSpec};
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

fn spec_matches(table: &toml::map::Map<String, toml::Value>, spec: &ServerSpec) -> bool {
    let mut wanted = spec.env.clone();
    wanted.sort();
    let mut found: Vec<String> = table
        .get("env_vars")
        .and_then(toml::Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(|n| n.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    table_matches(table, &spec.command) && found == wanted
}

fn spec_block(spec: &ServerSpec) -> String {
    let mut block = codex_block(&spec.name, &spec.command);
    if !spec.env.is_empty() {
        // Codex forwards these variables from the environment it was started in.
        let names = spec
            .env
            .iter()
            .map(|name| toml_string(name))
            .collect::<Vec<_>>()
            .join(", ");
        block = block.trim_end().to_owned() + &format!("\nenv_vars = [{names}]\n\n");
    }
    block
}

/// Adds and removes servers in the text of `.codex/config.toml`, leaving everything else as it is.
pub fn merge_servers(
    current: &str,
    add: &[ServerSpec],
    remove: &[ServerSpec],
) -> Result<Merge, String> {
    let mut text = current.to_owned();
    let mut merge = Merge::default();
    let existing =
        |text: &str, name: &str| -> Result<Option<toml::map::Map<String, toml::Value>>, String> {
            let document = parse_config(text)?;
            Ok(document
                .get("mcp_servers")
                .and_then(toml::Value::as_table)
                .and_then(|servers| servers.get(name))
                .and_then(toml::Value::as_table)
                .cloned())
        };
    for spec in add {
        match existing(&text, &spec.name)? {
            Some(table) if spec_matches(&table, spec) => merge.kept.push(spec.name.clone()),
            Some(_) => merge.conflicts.push(spec.name.clone()),
            None => {
                if !text.trim().is_empty() && !text.ends_with("\n\n") {
                    text = text.trim_end().to_owned() + "\n\n";
                }
                text.push_str(&spec_block(spec));
                merge.added.push(spec.name.clone());
            }
        }
    }
    for spec in remove {
        match existing(&text, &spec.name)? {
            Some(table) if spec_matches(&table, spec) => {
                let Some((start, end)) = table_bounds(&text, &spec.name) else {
                    return Err(format!(
                        "No se puede quitar mcp_servers.{} de forma segura porque usa una sintaxis TOML no compatible",
                        spec.name
                    ));
                };
                text.replace_range(start..end, "");
                merge.removed.push(spec.name.clone());
            }
            Some(_) => merge.conflicts.push(spec.name.clone()),
            None => {}
        }
    }
    if !merge.added.is_empty() || !merge.removed.is_empty() {
        let trimmed = text.trim_end();
        merge.text = Some(if trimmed.is_empty() {
            String::new()
        } else {
            format!("{trimmed}\n")
        });
    }
    Ok(merge)
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

#[cfg(test)]
mod merge_tests {
    use super::*;

    fn spec(name: &str, command: &[&str], env: &[&str]) -> ServerSpec {
        ServerSpec {
            name: name.into(),
            command: command.iter().map(|s| (*s).to_owned()).collect(),
            env: env.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn a_server_is_appended_with_the_variables_it_reads_and_the_rest_is_untouched() {
        let current = "model = \"gpt\"\n\n[mcp_servers.serena]\ncommand = \"serena\"\nargs = []\n";
        let pg = spec("postgres", &["uvx", "postgres-mcp"], &["DATABASE_URI"]);
        let merge = merge_servers(current, &[pg], &[]).unwrap();
        let text = merge.text.unwrap();
        assert!(text.starts_with(current.trim_end()), "{text}");
        let document: toml::Value = toml::from_str(&text).unwrap();
        let table = &document["mcp_servers"]["postgres"];
        assert_eq!(table["command"].as_str(), Some("uvx"));
        assert_eq!(table["env_vars"][0].as_str(), Some("DATABASE_URI"));
        assert!(!text.contains("://"));
    }

    #[test]
    fn it_is_idempotent_and_does_not_overwrite_a_server_the_person_changed() {
        let excel = spec("excel", &["npx", "--yes", "x"], &[]);
        let once = merge_servers("", std::slice::from_ref(&excel), &[])
            .unwrap()
            .text
            .unwrap();
        let again = merge_servers(&once, std::slice::from_ref(&excel), &[]).unwrap();
        assert!(again.text.is_none() && again.kept == ["excel"]);
        let changed = once.replace("--yes", "--no");
        let clash = merge_servers(&changed, &[excel], &[]).unwrap();
        assert!(clash.text.is_none() && clash.conflicts == ["excel"]);
    }

    #[test]
    fn removing_a_server_leaves_the_others_and_the_settings() {
        let a = spec("excel", &["npx", "x"], &[]);
        let b = spec("word", &["uvx", "y"], &[]);
        let both = merge_servers("model = \"gpt\"\n", &[a.clone(), b.clone()], &[])
            .unwrap()
            .text
            .unwrap();
        let gone = merge_servers(&both, &[], &[a]).unwrap();
        let document: toml::Value = toml::from_str(&gone.text.unwrap()).unwrap();
        assert!(document["mcp_servers"].get("excel").is_none());
        assert!(
            document["mcp_servers"]["word"].is_table() && document["model"].as_str() == Some("gpt")
        );
        let empty = merge_servers("", std::slice::from_ref(&b), &[])
            .unwrap()
            .text
            .unwrap();
        assert_eq!(
            merge_servers(&empty, &[], &[b]).unwrap().text.as_deref(),
            Some("")
        );
    }

    #[test]
    fn a_broken_toml_file_is_reported_not_rewritten() {
        let error = merge_servers("= roto", &[spec("a", &["x"], &[])], &[]).unwrap_err();
        assert!(error.contains("TOML"), "{error}");
    }
}
