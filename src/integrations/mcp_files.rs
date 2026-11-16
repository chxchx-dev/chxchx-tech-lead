//! The project-level MCP configuration files of the three agents, edited as text: Claude Code's
//! `.mcp.json`, OpenCode's `opencode.json` and Codex's `.codex/config.toml` (see `mcp_codex`).
//!
//! Everything here is pure (text in, text out) so a plan can show the exact result before a byte
//! is written. Secrets are never written: a server only names the environment variables it needs,
//! and each client's own syntax reads them from the person's environment at start-up.
use serde_json::{Map, Value, json};

/// A server as the three clients need to be told about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerSpec {
    pub name: String,
    /// Program and arguments.
    pub command: Vec<String>,
    /// Environment variables the server reads; their values come from the environment.
    pub env: Vec<String>,
}

/// What merging a set of servers into a file did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Merge {
    /// The new text, or `None` when nothing changes.
    pub text: Option<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Already configured exactly like this.
    pub kept: Vec<String>,
    /// Same name, configured differently by the person: left alone.
    pub conflicts: Vec<String>,
}

fn parse_object(current: &str, file: &str) -> Result<Map<String, Value>, String> {
    // Editors on Windows sometimes save a byte-order mark; JSON parsers reject it.
    let current = current.strip_prefix('\u{feff}').unwrap_or(current);
    if current.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(current) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(format!("{file} debe contener un objeto JSON.")),
        Err(error) => Err(format!(
            "{file} no es JSON válido ({error}); no se modifica."
        )),
    }
}

fn render(map: Map<String, Value>) -> String {
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default() + "\n"
}

/// How one client's JSON file spells its servers.
struct Format {
    file: &'static str,
    /// Where the servers live: `["mcpServers"]`, `["mcp", "servers"]`…
    section: Vec<&'static str>,
    entry: fn(&ServerSpec) -> Value,
    matches: fn(&Value, &ServerSpec) -> bool,
    /// What a file that does not exist yet starts with.
    fresh: fn(&mut Map<String, Value>),
}

/// The object at `path` inside `root`, created if missing.
fn object_at<'a>(
    root: &'a mut Map<String, Value>,
    path: &[&str],
    file: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    let mut current = root;
    for key in path {
        let value = current
            .entry((*key).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        current = match value {
            Value::Object(map) => map,
            _ => return Err(format!("`{key}` de {file} debe ser un objeto.")),
        };
    }
    Ok(current)
}

/// Removes the objects along `path` that were left empty, innermost first.
fn prune_empty(root: &mut Map<String, Value>, path: &[&str]) {
    for depth in (1..=path.len()).rev() {
        let mut map = &mut *root;
        for key in &path[..depth - 1] {
            match map.get_mut(*key) {
                Some(Value::Object(inner)) => map = inner,
                _ => return,
            }
        }
        match map.get(path[depth - 1]) {
            Some(Value::Object(inner)) if inner.is_empty() => {
                map.shift_remove(path[depth - 1]);
            }
            _ => return,
        }
    }
}

/// Adds and removes servers in the object at `format.section` of a JSON config.
fn merge_json(
    current: &str,
    format: &Format,
    add: &[ServerSpec],
    remove: &[ServerSpec],
) -> Result<Merge, String> {
    let existed = !current.trim().is_empty();
    let mut root = parse_object(current, format.file)?;
    if !existed {
        (format.fresh)(&mut root);
    }
    let mut merge = Merge::default();
    {
        let servers = object_at(&mut root, &format.section, format.file)?;
        for spec in add {
            match servers.get(&spec.name) {
                Some(existing) if (format.matches)(existing, spec) => {
                    merge.kept.push(spec.name.clone());
                }
                Some(_) => merge.conflicts.push(spec.name.clone()),
                None => {
                    servers.insert(spec.name.clone(), (format.entry)(spec));
                    merge.added.push(spec.name.clone());
                }
            }
        }
        for spec in remove {
            match servers.get(&spec.name) {
                Some(existing) if (format.matches)(existing, spec) => {
                    servers.shift_remove(&spec.name);
                    merge.removed.push(spec.name.clone());
                }
                Some(_) => merge.conflicts.push(spec.name.clone()),
                None => {}
            }
        }
    }
    if !merge.added.is_empty() || !merge.removed.is_empty() {
        // Sections left empty by the last removal are not worth keeping in a file that existed.
        if existed {
            prune_empty(&mut root, &format.section);
        }
        merge.text = Some(render(root));
    }
    Ok(merge)
}

fn claude_entry(spec: &ServerSpec) -> Value {
    let mut entry = Map::new();
    entry.insert("type".into(), json!("stdio"));
    entry.insert("command".into(), json!(spec.command[0]));
    entry.insert("args".into(), json!(&spec.command[1..]));
    if !spec.env.is_empty() {
        // Claude Code expands `${NAME}` from the environment when it starts the server.
        let env: Map<String, Value> = spec
            .env
            .iter()
            .map(|name| (name.clone(), json!(format!("${{{name}}}"))))
            .collect();
        entry.insert("env".into(), Value::Object(env));
    }
    Value::Object(entry)
}

fn claude_matches(existing: &Value, spec: &ServerSpec) -> bool {
    let words = |key: &str| -> Option<Vec<String>> {
        existing
            .get(key)?
            .as_array()?
            .iter()
            .map(|v| v.as_str().map(str::to_owned))
            .collect()
    };
    let env: Vec<String> = existing
        .get("env")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    existing.get("command").and_then(Value::as_str) == Some(spec.command[0].as_str())
        && match existing.get("args") {
            None => spec.command.len() == 1,
            Some(_) => words("args").is_some_and(|args| args == spec.command[1..]),
        }
        && sorted(env) == sorted(spec.env.clone())
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names
}

/// Claude Code: `.mcp.json` of the project (`mcpServers`).
pub fn claude_project(
    current: &str,
    add: &[ServerSpec],
    remove: &[ServerSpec],
) -> Result<Merge, String> {
    merge_json(
        current,
        &Format {
            file: ".mcp.json",
            section: vec!["mcpServers"],
            entry: claude_entry,
            matches: claude_matches,
            fresh: |_| {},
        },
        add,
        remove,
    )
}

/// How an `opencode.json` spells its servers: OpenCode 1.x keeps them directly under `mcp`; 2.x
/// (whose own `mcp add` writes this) keeps them under `mcp.servers`. 2.x also reads the 1.x form,
/// but 1.x would take `servers` for a server's name, so a file keeps the style it already has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenCodeStyle {
    /// `mcp.<name>`, with `enabled`.
    Flat,
    /// `mcp.servers.<name>`.
    Servers,
}

/// The style a file is already written in, if it has servers at all.
pub fn opencode_style_of(current: &str) -> Option<OpenCodeStyle> {
    let current = current.strip_prefix('\u{feff}').unwrap_or(current);
    let root: Value = serde_json::from_str(current).ok()?;
    let mcp = root.get("mcp")?.as_object()?;
    if mcp.get("servers").is_some_and(Value::is_object) {
        return Some(OpenCodeStyle::Servers);
    }
    mcp.values()
        .any(|v| v.get("type").is_some() || v.get("command").is_some())
        .then_some(OpenCodeStyle::Flat)
}

fn opencode_entry_for(style: OpenCodeStyle, spec: &ServerSpec) -> Value {
    let mut entry = Map::new();
    entry.insert("type".into(), json!("local"));
    entry.insert("command".into(), json!(spec.command));
    if style == OpenCodeStyle::Flat {
        entry.insert("enabled".into(), json!(true));
    }
    if !spec.env.is_empty() {
        // OpenCode substitutes `{env:NAME}` from the environment.
        let env: Map<String, Value> = spec
            .env
            .iter()
            .map(|name| (name.clone(), json!(format!("{{env:{name}}}"))))
            .collect();
        entry.insert("environment".into(), Value::Object(env));
    }
    Value::Object(entry)
}

fn opencode_matches(existing: &Value, spec: &ServerSpec) -> bool {
    let command: Option<Vec<String>> = existing
        .get("command")
        .and_then(Value::as_array)
        .and_then(|a| a.iter().map(|v| v.as_str().map(str::to_owned)).collect());
    let env: Vec<String> = existing
        .get("environment")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    command.as_deref() == Some(spec.command.as_slice()) && sorted(env) == sorted(spec.env.clone())
}

/// OpenCode: `opencode.json` of the project. An existing file keeps its style; a new one (or one
/// without servers) gets `preferred`.
pub fn opencode_project(
    current: &str,
    add: &[ServerSpec],
    remove: &[ServerSpec],
    preferred: OpenCodeStyle,
) -> Result<Merge, String> {
    let style = opencode_style_of(current).unwrap_or(preferred);
    let (section, entry): (Vec<&'static str>, fn(&ServerSpec) -> Value) = match style {
        OpenCodeStyle::Flat => (vec!["mcp"], |s| opencode_entry_for(OpenCodeStyle::Flat, s)),
        OpenCodeStyle::Servers => (vec!["mcp", "servers"], |s| {
            opencode_entry_for(OpenCodeStyle::Servers, s)
        }),
    };
    merge_json(
        current,
        &Format {
            file: "opencode.json",
            section,
            entry,
            matches: opencode_matches,
            fresh: |root| {
                root.insert("$schema".into(), json!("https://opencode.ai/config.json"));
            },
        },
        add,
        remove,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(name: &str, command: &[&str], env: &[&str]) -> ServerSpec {
        ServerSpec {
            name: name.into(),
            command: command.iter().map(|s| (*s).to_owned()).collect(),
            env: env.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn claude_gets_a_stdio_entry_that_reads_secrets_from_the_environment() {
        let pg = spec("postgres", &["uvx", "postgres-mcp"], &["DATABASE_URI"]);
        let merge = claude_project("", &[pg], &[]).unwrap();
        let text = merge.text.unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        let entry = &json["mcpServers"]["postgres"];
        assert_eq!(entry["type"], "stdio");
        assert_eq!(entry["command"], "uvx");
        assert_eq!(entry["args"], json!(["postgres-mcp"]));
        assert_eq!(entry["env"]["DATABASE_URI"], "${DATABASE_URI}");
        assert!(!text.contains("://"), "no secret value is ever written");
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn the_rest_of_the_file_survives_in_its_own_order() {
        let current =
            r#"{"zeta": 1, "mcpServers": {"mio": {"command": "x", "args": []}}, "alfa": 2}"#;
        let merge = claude_project(current, &[spec("excel", &["npx", "excel"], &[])], &[]).unwrap();
        let text = merge.text.unwrap();
        assert!(text.find("zeta").unwrap() < text.find("mcpServers").unwrap());
        assert!(text.find("mcpServers").unwrap() < text.find("alfa").unwrap());
        assert!(text.contains("\"mio\"") && text.contains("\"excel\""));
    }

    #[test]
    fn adding_twice_changes_nothing_and_a_different_server_of_that_name_is_not_overwritten() {
        let excel = spec("excel", &["npx", "--yes", "x"], &[]);
        let once = claude_project("", std::slice::from_ref(&excel), &[])
            .unwrap()
            .text
            .unwrap();
        let twice = claude_project(&once, std::slice::from_ref(&excel), &[]).unwrap();
        assert!(twice.text.is_none() && twice.kept == ["excel"]);
        let other = spec("excel", &["npx", "otro"], &[]);
        let clash = claude_project(&once, &[other], &[]).unwrap();
        assert!(clash.text.is_none() && clash.conflicts == ["excel"]);
    }

    #[test]
    fn removing_takes_only_what_we_wrote_and_tidies_an_emptied_section() {
        let excel = spec("excel", &["npx", "--yes", "x"], &[]);
        let once = claude_project(r#"{"otro": true}"#, std::slice::from_ref(&excel), &[])
            .unwrap()
            .text
            .unwrap();
        let gone = claude_project(&once, &[], std::slice::from_ref(&excel)).unwrap();
        assert_eq!(gone.removed, ["excel"]);
        let after: Value = serde_json::from_str(&gone.text.unwrap()).unwrap();
        assert!(after.get("mcpServers").is_none() && after["otro"] == true);
        // The person edited it: it is theirs now.
        let edited = r#"{"mcpServers": {"excel": {"command": "propio"}}}"#;
        let keep = claude_project(edited, &[], &[excel]).unwrap();
        assert!(keep.text.is_none() && keep.conflicts == ["excel"]);
    }

    #[test]
    fn invalid_files_are_reported_not_rewritten() {
        let error = claude_project("{ roto", &[spec("a", &["x"], &[])], &[]).unwrap_err();
        assert!(
            error.contains(".mcp.json") && error.contains("no se modifica"),
            "{error}"
        );
        assert!(claude_project("[]", &[], &[]).is_err());
        assert!(claude_project(r#"{"mcpServers": 3}"#, &[spec("a", &["x"], &[])], &[]).is_err());
    }

    #[test]
    fn opencode_two_point_x_gets_what_its_own_mcp_add_writes() {
        let pg = spec("postgres", &["uvx", "postgres-mcp"], &["DATABASE_URI"]);
        let text = opencode_project("", &[pg], &[], OpenCodeStyle::Servers)
            .unwrap()
            .text
            .unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["$schema"], "https://opencode.ai/config.json");
        let entry = &json["mcp"]["servers"]["postgres"];
        assert_eq!(entry["type"], "local");
        assert_eq!(entry["command"], json!(["uvx", "postgres-mcp"]));
        assert_eq!(entry["environment"]["DATABASE_URI"], "{env:DATABASE_URI}");
        assert!(entry.get("enabled").is_none(), "2.x has no `enabled`");
    }

    #[test]
    fn opencode_one_point_x_gets_the_flat_form_with_enabled() {
        let text = opencode_project(
            "",
            &[spec("excel", &["npx", "x"], &[])],
            &[],
            OpenCodeStyle::Flat,
        )
        .unwrap()
        .text
        .unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["mcp"]["excel"]["enabled"], true);
        assert!(json["mcp"].get("servers").is_none());
    }

    #[test]
    fn a_file_keeps_the_style_it_already_has_whatever_is_preferred() {
        let flat = r#"{"mcp": {"viejo": {"type": "local", "command": ["x"], "enabled": true}}}"#;
        let merged = opencode_project(
            flat,
            &[spec("excel", &["npx", "x"], &[])],
            &[],
            OpenCodeStyle::Servers,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&merged.text.unwrap()).unwrap();
        assert!(
            json["mcp"]["excel"].is_object() && json["mcp"]["viejo"].is_object(),
            "no `servers` is introduced"
        );
        let servers = r#"{"mcp": {"servers": {"viejo": {"type": "local", "command": ["x"]}}}}"#;
        let merged = opencode_project(
            servers,
            &[spec("excel", &["npx", "x"], &[])],
            &[],
            OpenCodeStyle::Flat,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&merged.text.unwrap()).unwrap();
        assert!(
            json["mcp"]["servers"]["excel"].is_object()
                && json["mcp"]["servers"]["viejo"].is_object()
        );
        assert_eq!(opencode_style_of(flat), Some(OpenCodeStyle::Flat));
        assert_eq!(opencode_style_of(servers), Some(OpenCodeStyle::Servers));
        assert_eq!(opencode_style_of(r#"{"model": "x"}"#), None);
    }

    #[test]
    fn an_existing_opencode_file_keeps_its_models_and_providers() {
        let current = r#"{"model": "ollama/x", "provider": {"ollama": {}}}"#;
        let merge = opencode_project(
            current,
            &[spec("excel", &["npx", "x"], &[])],
            &[],
            OpenCodeStyle::Servers,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&merge.text.unwrap()).unwrap();
        assert_eq!(json["model"], "ollama/x");
        assert!(
            json["provider"]["ollama"].is_object() && json["mcp"]["servers"]["excel"].is_object()
        );
        assert!(
            json.get("$schema").is_none(),
            "an existing file keeps its own header"
        );
    }

    #[test]
    fn removing_the_last_opencode_server_leaves_no_empty_husk_but_keeps_the_rest() {
        let excel = spec("excel", &["npx", "x"], &[]);
        for style in [OpenCodeStyle::Servers, OpenCodeStyle::Flat] {
            let once = opencode_project(
                r#"{"model": "m"}"#,
                std::slice::from_ref(&excel),
                &[],
                style,
            )
            .unwrap()
            .text
            .unwrap();
            let gone = opencode_project(&once, &[], std::slice::from_ref(&excel), style).unwrap();
            let json: Value = serde_json::from_str(&gone.text.unwrap()).unwrap();
            assert!(
                json.get("mcp").is_none() && json["model"] == "m",
                "{style:?}: {json}"
            );
        }
    }

    #[test]
    fn a_byte_order_mark_and_odd_shapes_are_handled_not_panicked_on() {
        let bom = "\u{feff}{\"mcpServers\": {}}";
        let merge = claude_project(bom, &[spec("a", &["x"], &[])], &[]).unwrap();
        assert!(merge.text.unwrap().contains("\"a\""));
        // Entries that are not objects, commands that are not strings: a conflict, never a crash.
        for odd in [
            r#"{"mcpServers": {"a": 5}}"#,
            r#"{"mcpServers": {"a": {"command": 5, "args": "x"}}}"#,
            r#"{"mcpServers": {"a": {"command": "x", "args": [1, 2]}}}"#,
            r#"{"mcpServers": {"a": null}}"#,
        ] {
            let merge = claude_project(odd, &[spec("a", &["x"], &[])], &[]).unwrap();
            assert_eq!(merge.conflicts, ["a"], "{odd}");
            assert!(merge.text.is_none());
        }
    }
}
