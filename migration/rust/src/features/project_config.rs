use std::collections::BTreeSet;
use std::path::Path;

pub struct ConfigValidation {
    pub supported: bool,
    pub error: Option<String>,
}

pub fn validate_project_config(document: &toml::Value, root: &Path) -> ConfigValidation {
    let result = validate(document, root);
    ConfigValidation {
        supported: result.is_ok(),
        error: result.err(),
    }
}

fn validate(document: &toml::Value, root: &Path) -> Result<(), String> {
    let version = document
        .get("version")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| "version debe ser un entero".to_owned())?;
    if !matches!(version, 1 | 2) {
        return Err(format!("Versión de configuración no soportada: {version}"));
    }
    let Some(workspace) = document.get("workspace") else {
        return if version == 1 {
            Ok(())
        } else {
            Err("workspace debe ser una tabla".to_owned())
        };
    };
    let workspace = workspace
        .as_table()
        .ok_or_else(|| "workspace debe ser una tabla".to_owned())?;
    validate_workspace(workspace, root)
}

fn validate_workspace(
    workspace: &toml::map::Map<String, toml::Value>,
    root: &Path,
) -> Result<(), String> {
    required_string(workspace, "name", "workspace.name")?;
    for key in ["adapter", "editor"] {
        optional_string(workspace, key, &format!("workspace.{key}"))?;
    }
    for key in ["auto_open_editor", "auto_attach", "auto_start"] {
        optional_bool(workspace, key, &format!("workspace.{key}"))?;
    }

    if let Some(header) = workspace.get("header") {
        let header = header
            .as_table()
            .ok_or_else(|| "workspace.header debe ser una tabla".to_owned())?;
        optional_bool(header, "enabled", "workspace.header.enabled")?;
        for key in ["label", "template"] {
            optional_string(header, key, &format!("workspace.header.{key}"))?;
        }
        if header
            .get("logo")
            .is_some_and(|value| value.as_str().is_none())
        {
            return Err("workspace.header.logo debe ser texto".to_owned());
        }
        if let Some(template) = header.get("template").and_then(toml::Value::as_str) {
            validate_template(template)?;
        }
    }
    if let Some(layout) = workspace.get("layout") {
        let layout = layout
            .as_table()
            .ok_or_else(|| "workspace.layout debe ser una tabla".to_owned())?;
        if let Some(orientation) = layout.get("orientation") {
            let orientation = orientation
                .as_str()
                .ok_or_else(|| "workspace.layout.orientation debe ser texto".to_owned())?;
            if !matches!(
                orientation.to_lowercase().as_str(),
                "horizontal" | "vertical"
            ) {
                return Err(
                    "workspace.layout.orientation debe ser horizontal o vertical".to_owned(),
                );
            }
        }
    }
    validate_resources(workspace.get("resources"))?;

    let processes = table_array(workspace.get("processes"), "workspace.processes")?;
    let agents = table_array(workspace.get("agents"), "workspace.agents")?;
    let mut ids = BTreeSet::new();
    let mut agent_ids = BTreeSet::new();
    for (index, process) in processes.iter().enumerate() {
        let id = validate_command_entry(process, "processes", index, root, true)?;
        if !ids.insert(id.to_owned()) {
            return Err("IDs duplicados en workspace".to_owned());
        }
    }
    for (index, agent) in agents.iter().enumerate() {
        let id = validate_command_entry(agent, "agents", index, root, false)?;
        if !ids.insert(id.to_owned()) {
            return Err("IDs duplicados en workspace".to_owned());
        }
        agent_ids.insert(id.to_owned());
    }
    if let Some(presets) = workspace.get("presets") {
        let presets = presets
            .as_table()
            .ok_or_else(|| "workspace.presets debe ser una tabla".to_owned())?;
        for (name, value) in presets {
            if name.trim().is_empty() {
                return Err("workspace.presets contiene un nombre inválido".to_owned());
            }
            let members = value
                .as_array()
                .ok_or_else(|| format!("workspace.presets.{name} debe ser una lista de agentes"))?;
            for member in members {
                let member = member
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        format!("workspace.presets.{name} debe ser una lista de agentes")
                    })?;
                if !agent_ids.contains(member) {
                    return Err(format!(
                        "workspace.presets.{name} referencia agentes inexistentes"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_resources(value: Option<&toml::Value>) -> Result<(), String> {
    let Some(value) = value else { return Ok(()) };
    let resources = value
        .as_table()
        .ok_or_else(|| "workspace.resources debe ser una tabla".to_owned())?;
    let mut percentages = [70, 85];
    for (index, key) in ["warn_memory_percent", "critical_memory_percent"]
        .into_iter()
        .enumerate()
    {
        percentages[index] = optional_integer(resources, key, 0, 100, "workspace.resources")?
            .unwrap_or(percentages[index]);
    }
    optional_integer(
        resources,
        "warn_swap_percent",
        0,
        100,
        "workspace.resources",
    )?;
    optional_integer(resources, "max_agents", 1, 16, "workspace.resources")?;
    if percentages[1] < percentages[0] {
        return Err(
            "critical_memory_percent no puede ser menor que warn_memory_percent".to_owned(),
        );
    }
    Ok(())
}

fn validate_command_entry<'a>(
    entry: &'a toml::map::Map<String, toml::Value>,
    group: &str,
    index: usize,
    root: &Path,
    process: bool,
) -> Result<&'a str, String> {
    let prefix = format!("workspace.{group}[{index}]");
    let id = required_string(entry, "id", &format!("{prefix}.id"))?;
    let shell = optional_bool(entry, "shell", &format!("{prefix}.shell"))?.unwrap_or(false);
    let command = entry
        .get("command")
        .ok_or_else(|| format!("{prefix}.command es obligatorio"))?;
    let valid_command = command.as_array().is_some_and(|values| {
        !values.is_empty()
            && values
                .iter()
                .all(|value| value.as_str().is_some_and(|value| !value.is_empty()))
    }) || (shell
        && command
            .as_str()
            .is_some_and(|value| !value.trim().is_empty()));
    if !valid_command {
        return Err(if shell {
            format!("{prefix}.command debe ser un comando de texto o lista no vacía")
        } else {
            format!("{prefix}.command debe ser una lista de argumentos no vacía")
        });
    }
    if let Some(cwd) = entry.get("cwd") {
        let cwd = cwd
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("{prefix}.cwd debe ser un texto no vacío"))?;
        validate_relative_path(cwd, root, &format!("{prefix}.cwd"))?;
    }
    optional_bool(entry, "auto_start", &format!("{prefix}.auto_start"))?;
    if process {
        optional_string(entry, "label", &format!("{prefix}.label"))?;
        if let Some(restart) = entry.get("restart") {
            let restart = restart
                .as_str()
                .ok_or_else(|| format!("restart inválido para {id}"))?;
            if !matches!(restart, "never" | "on-failure" | "always") {
                return Err(format!("{prefix}.restart tiene un valor no soportado"));
            }
        }
        if let Some(port) = entry.get("port") {
            port.as_integer()
                .filter(|port| (1..=65535).contains(port))
                .ok_or_else(|| format!("{prefix}.port debe ser un puerto entre 1 y 65535"))?;
        }
    }
    Ok(id)
}

fn validate_relative_path(value: &str, root: &Path, field: &str) -> Result<(), String> {
    let windows_drive = value.as_bytes().get(1) == Some(&b':');
    if Path::new(value).is_absolute()
        || matches!(value.chars().next(), Some('/' | '\\'))
        || windows_drive
        || value.split(['/', '\\']).any(|part| part == "..")
    {
        return Err(format!(
            "{field} debe ser una ruta relativa dentro del proyecto"
        ));
    }
    let root = root
        .canonicalize()
        .map_err(|_| format!("No se pudo resolver la raíz del proyecto para {field}"))?;
    let candidate = root.join(value);
    let mut existing = candidate.as_path();
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| format!("{field} debe permanecer dentro del proyecto"))?;
    }
    let canonical = existing
        .canonicalize()
        .map_err(|_| format!("No se pudo resolver {field}"))?;
    if !canonical.starts_with(&root) {
        return Err("cwd fuera del proyecto".to_owned());
    }
    Ok(())
}

fn validate_template(template: &str) -> Result<(), String> {
    let allowed = ["logo", "label", "project", "profile"];
    let mut chars = template.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
            }
            '{' => {
                let mut field = String::new();
                let mut closed = false;
                for next in chars.by_ref() {
                    if next == '}' {
                        closed = true;
                        break;
                    }
                    field.push(next);
                }
                let field = field.split(['!', ':']).next().unwrap_or_default();
                if !closed || (!field.is_empty() && !allowed.contains(&field)) {
                    return Err(format!(
                        "workspace.header.template solo admite: {}",
                        allowed.join(", ")
                    ));
                }
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
            }
            '}' => {
                return Err("workspace.header.template tiene una llave sin pareja".to_owned());
            }
            _ => {}
        }
    }
    Ok(())
}

fn table_array<'a>(
    value: Option<&'a toml::Value>,
    field: &str,
) -> Result<Vec<&'a toml::map::Map<String, toml::Value>>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .ok_or_else(|| format!("{field} debe ser una lista"))?;
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_table()
                .ok_or_else(|| format!("{field}[{index}] debe ser una tabla"))
        })
        .collect()
}

fn required_string<'a>(
    table: &'a toml::map::Map<String, toml::Value>,
    key: &str,
    field: &str,
) -> Result<&'a str, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::trim)
        .ok_or_else(|| format!("{field} debe ser un texto no vacío"))
}

fn optional_string<'a>(
    table: &'a toml::map::Map<String, toml::Value>,
    key: &str,
    field: &str,
) -> Result<Option<&'a str>, String> {
    table
        .get(key)
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| format!("{field} debe ser un texto no vacío"))
        })
        .transpose()
}

fn optional_bool(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    field: &str,
) -> Result<Option<bool>, String> {
    table
        .get(key)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| format!("{field} debe ser booleano"))
        })
        .transpose()
}

fn optional_integer(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    minimum: i64,
    maximum: i64,
    prefix: &str,
) -> Result<Option<i64>, String> {
    table
        .get(key)
        .map(|value| {
            value
                .as_integer()
                .filter(|value| (minimum..=maximum).contains(value))
                .ok_or_else(|| format!("{prefix}.{key} debe estar entre {minimum} y {maximum}"))
        })
        .transpose()
}
