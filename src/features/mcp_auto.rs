//! Keeps Basic Memory and Serena registered in Claude Code for a prepared, trusted project.
//!
//! Studio calls this at start and restart. It is idempotent and quiet: it reads Claude's own
//! config (read-only) and only when a server is missing runs the same planned, backed-up
//! integration as `integrate --client claude --apply`, which uses the official CLI.
use crate::features::mcp_setup::{McpClient, apply_mcp_integration, plan_mcp_integration};
use crate::features::project_overview::inspect_project;
use std::path::Path;

const SERVERS: [&str; 2] = ["basic-memory", "serena"];

/// Whether Claude's config (`~/.claude.json` contents) has every server for the project at `root`.
fn registered(config: &str, root: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(config) else {
        return false;
    };
    let servers = value
        .pointer(&format!("/projects/{}/mcpServers", escape(root)))
        .or_else(|| value.get("mcpServers"));
    servers
        .and_then(|s| s.as_object())
        .is_some_and(|s| SERVERS.iter().all(|name| s.contains_key(*name)))
        || SERVERS.iter().all(|name| {
            value
                .get("mcpServers")
                .and_then(|s| s.as_object())
                .is_some_and(|s| s.contains_key(*name))
        })
}

/// JSON-pointer escaping of a path used as an object key.
fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// Registers the servers when they are missing. `Ok(None)` when there was nothing to do (not
/// trusted, not prepared, no Claude Code, or already registered).
pub fn ensure_claude(root: &Path) -> Result<Option<String>, String> {
    let overview = inspect_project(root)?;
    if !overview.workspace.trusted {
        return Ok(None);
    }
    let project = overview.project.root.clone();
    let config = std::env::home_dir().map(|home| home.join(".claude.json"));
    if let Some(config) = config
        && let Ok(text) = std::fs::read_to_string(&config)
        && registered(&text, &project)
    {
        return Ok(None);
    }
    let plan = plan_mcp_integration(Path::new(&project), &[McpClient::Claude], false)?;
    if plan.blocked_reason.is_some() {
        return Ok(None);
    }
    let result = apply_mcp_integration(&plan)?;
    // The plan's own notice ("Claude will query MCP state") is not a failure.
    let problems: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| !plan.warnings.contains(w))
        .cloned()
        .collect();
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(Some(
        "Basic Memory y Serena quedaron activos en Claude Code".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_servers_must_be_present_for_the_project() {
        let both = r#"{"projects":{"/p":{"mcpServers":{"basic-memory":{},"serena":{}}}}}"#;
        let one = r#"{"projects":{"/p":{"mcpServers":{"serena":{}}}}}"#;
        assert!(registered(both, "/p"));
        assert!(!registered(one, "/p"));
        assert!(!registered(both, "/other"));
    }

    #[test]
    fn user_level_servers_count_and_broken_json_does_not() {
        let global = r#"{"mcpServers":{"basic-memory":{},"serena":{}}}"#;
        assert!(registered(global, "/p"));
        assert!(!registered("{not json", "/p"));
    }

    #[test]
    fn slashes_in_the_path_are_escaped_for_the_pointer() {
        assert_eq!(escape("/a/b~c"), "~1a~1b~0c");
    }
}
