//! MCP connectors: servers that give an agent hands in an app (Excel, Power BI, a database…).
//! The catalog lives in `catalog/connectors/*.toml`; this module reads it and answers what a
//! connector needs here (a runtime, a value) without starting anything.
use crate::features::catalog;
use crate::integrations::mcp_clients::find_executable;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvVar {
    pub name: String,
    pub description: String,
    /// Holds a password or a key path: it is read from the environment and never written down.
    pub secret: bool,
    pub required: bool,
}

/// A value the person gives when adding the connector (a database path).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub description: String,
    pub placeholder: String,
}

#[derive(Clone, Debug)]
pub struct Connector {
    pub name: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub docs: String,
    /// `oficial`, `comunidad` or `referencia`.
    pub origin: String,
    /// Programs it needs on the PATH (`npx`, `uvx`).
    pub requires: Vec<String>,
    command: Vec<String>,
    command_windows: Option<Vec<String>>,
    pub platforms: Vec<String>,
    pub note: String,
    pub env: Vec<EnvVar>,
    pub params: Vec<Param>,
}

fn text(values: &toml::Table, key: &str) -> Option<String> {
    values.get(key)?.as_str().map(str::to_owned)
}

fn list(values: &toml::Table, key: &str) -> Vec<String> {
    values
        .get(key)
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn tables<'a>(values: &'a toml::Table, key: &str) -> impl Iterator<Item = &'a toml::Table> {
    values
        .get(key)
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table)
}

fn parse(manifest: &str) -> Option<Connector> {
    let values: toml::Table = toml::from_str(manifest).ok()?;
    let command = list(&values, "command");
    if command.is_empty() {
        return None;
    }
    let windows = list(&values, "command_windows");
    Some(Connector {
        name: text(&values, "name")?,
        title: text(&values, "title")?,
        category: text(&values, "category")?,
        description: text(&values, "description")?,
        docs: text(&values, "docs").unwrap_or_default(),
        origin: text(&values, "origin").unwrap_or_else(|| "comunidad".to_owned()),
        requires: list(&values, "requires"),
        command,
        command_windows: (!windows.is_empty()).then_some(windows),
        platforms: list(&values, "platforms"),
        note: text(&values, "note").unwrap_or_default(),
        env: tables(&values, "env")
            .filter_map(|t| {
                Some(EnvVar {
                    name: text(t, "name")?,
                    description: text(t, "description").unwrap_or_default(),
                    secret: t
                        .get("secret")
                        .and_then(toml::Value::as_bool)
                        .unwrap_or(true),
                    required: t
                        .get("required")
                        .and_then(toml::Value::as_bool)
                        .unwrap_or(false),
                })
            })
            .collect(),
        params: tables(&values, "param")
            .filter_map(|t| {
                Some(Param {
                    name: text(t, "name")?,
                    description: text(t, "description").unwrap_or_default(),
                    placeholder: text(t, "placeholder").unwrap_or_default(),
                })
            })
            .collect(),
    })
}

pub fn connectors() -> &'static [Connector] {
    static CACHE: OnceLock<Vec<Connector>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut found: Vec<Connector> = catalog::connector_manifests()
            .iter()
            .filter_map(|(_, manifest, _)| parse(manifest))
            .collect();
        found.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        found
    })
}

pub fn connector(name: &str) -> Option<&'static Connector> {
    connectors()
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(name))
}

/// The name of this operating system as the catalog spells it.
pub fn current_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

impl Connector {
    pub fn supported_here(&self) -> bool {
        self.platforms.is_empty() || self.platforms.iter().any(|p| p == current_platform())
    }

    /// Programs it needs that are not installed.
    pub fn missing_requirements(&self) -> Vec<String> {
        self.requires
            .iter()
            .filter(|program| find_executable(program).is_none())
            .cloned()
            .collect()
    }

    /// The program and arguments to start it, with the given values in their places.
    /// `windows` picks the `cmd /c` form some Windows setups need for `npx`.
    pub fn command_for(
        &self,
        windows: bool,
        values: &BTreeMap<String, String>,
    ) -> Result<Vec<String>, String> {
        let template = match (&self.command_windows, windows) {
            (Some(command), true) => command,
            _ => &self.command,
        };
        for param in &self.params {
            let value = values.get(&param.name).map(|v| v.trim()).unwrap_or("");
            if value.is_empty() {
                return Err(format!(
                    "{} necesita el valor `{}` ({}).",
                    self.title, param.name, param.description
                ));
            }
            if value.contains(['\n', '\r', '\0']) {
                return Err(format!("El valor de `{}` no es válido.", param.name));
            }
        }
        Ok(template
            .iter()
            .map(|word| {
                let mut word = word.clone();
                for (name, value) in values {
                    word = word.replace(&format!("{{{name}}}"), value.trim());
                }
                word
            })
            .collect())
    }

    /// Variables the connector cannot start without.
    pub fn required_env(&self) -> impl Iterator<Item = &EnvVar> {
        self.env.iter().filter(|e| e.required)
    }
}

/// Whether `name` is usable as an MCP server name in every client's configuration.
pub fn valid_server_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 48
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Whether `name` is usable as an environment variable name.
pub fn valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_has_the_apps_people_asked_for() {
        for name in [
            "excel",
            "powerbi",
            "word",
            "google-sheets",
            "postgres",
            "sqlite",
        ] {
            assert!(connector(name).is_some(), "{name} is missing");
        }
        assert_eq!(
            catalog::connector_manifests().len(),
            connectors().len(),
            "a manifest that does not parse would silently vanish"
        );
    }

    #[test]
    fn every_connector_is_well_formed() {
        let mut seen = std::collections::BTreeSet::new();
        for c in connectors() {
            assert!(seen.insert(c.name.clone()), "duplicate {}", c.name);
            assert!(valid_server_name(&c.name), "{}", c.name);
            assert!(
                !c.description.is_empty() && !c.category.is_empty(),
                "{}",
                c.name
            );
            assert!(
                !c.requires.is_empty(),
                "{} must say what it runs with",
                c.name
            );
            assert!(
                ["oficial", "comunidad", "referencia"].contains(&c.origin.as_str()),
                "{}",
                c.name
            );
            for env in &c.env {
                assert!(valid_env_name(&env.name), "{}: {}", c.name, env.name);
            }
            // Every {placeholder} of the command is a declared parameter, and the other way round.
            let words = c.command.join(" ");
            for param in &c.params {
                assert!(
                    words.contains(&format!("{{{}}}", param.name)),
                    "{}: {}",
                    c.name,
                    param.name
                );
            }
            assert_eq!(words.matches('{').count(), c.params.len(), "{}", c.name);
            // The first word is the program the connector says it requires.
            assert!(c.requires.contains(&c.command[0]), "{}", c.name);
            if let Some(windows) = &c.command_windows {
                assert!(windows.iter().any(|w| *w == c.command[0]), "{}", c.name);
            }
        }
    }

    #[test]
    fn secrets_are_declared_as_such_and_never_have_a_value_in_the_catalog() {
        let postgres = connector("postgres").unwrap();
        let uri = postgres.required_env().next().unwrap();
        assert!(uri.secret && uri.name == "DATABASE_URI");
        assert!(
            postgres
                .command_for(false, &BTreeMap::new())
                .unwrap()
                .iter()
                .all(|w| !w.contains("://"))
        );
    }

    #[test]
    fn a_connector_with_a_parameter_needs_its_value_and_fills_it_in() {
        let sqlite = connector("sqlite").unwrap();
        let error = sqlite.command_for(false, &BTreeMap::new()).unwrap_err();
        assert!(error.contains("db_path"), "{error}");
        let values = BTreeMap::from([("db_path".to_owned(), " ./datos.db ".to_owned())]);
        let command = sqlite.command_for(false, &values).unwrap();
        assert_eq!(
            command,
            ["uvx", "mcp-server-sqlite", "--db-path", "./datos.db"]
        );
        let bad = BTreeMap::from([("db_path".to_owned(), "a\nb".to_owned())]);
        assert!(sqlite.command_for(false, &bad).is_err());
    }

    #[test]
    fn windows_gets_the_cmd_form_when_the_catalog_has_one() {
        let excel = connector("excel").unwrap();
        let none = BTreeMap::new();
        assert_eq!(excel.command_for(false, &none).unwrap()[0], "npx");
        assert_eq!(excel.command_for(true, &none).unwrap()[0], "cmd");
        let word = connector("word").unwrap();
        assert_eq!(
            word.command_for(true, &none).unwrap()[0],
            "uvx",
            "no special form"
        );
    }

    #[test]
    fn names_are_checked_before_they_reach_a_config_file() {
        assert!(
            valid_server_name("mi-servidor_2")
                && !valid_server_name("a b")
                && !valid_server_name("")
        );
        assert!(
            valid_env_name("DATABASE_URI") && !valid_env_name("database") && !valid_env_name("1A")
        );
    }
}
