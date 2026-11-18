//! Provider-specific enforcement of the visual chat's access choices.
//!
//! These values are passed as argument lists or environment data to official provider CLIs;
//! they are not shell fragments.

use serde_json::{Map, Value, json};

pub const MODES: [&str; 4] = [
    "read-only",
    "workspace-write",
    "workspace-commands",
    "full-access",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
    pub mode: String,
    pub network: bool,
    /// Provider-specific rules already approved for this chat.
    pub allow: Vec<String>,
}

impl Default for Access {
    fn default() -> Self {
        Self {
            mode: "read-only".into(),
            network: false,
            allow: Vec::new(),
        }
    }
}

impl Access {
    pub fn new(mode: impl Into<String>, network: bool, allow: Vec<String>) -> Result<Self, String> {
        let access = Self {
            mode: mode.into(),
            network,
            allow,
        };
        if MODES.contains(&access.mode.as_str()) {
            Ok(access)
        } else {
            Err(format!("Modo no soportado: {}", access.mode))
        }
    }

    pub fn edits(&self) -> bool {
        self.mode != "read-only"
    }
}

pub fn codex_arguments(access: &Access) -> Vec<String> {
    let sandbox = match access.mode.as_str() {
        "read-only" => "read-only",
        "workspace-write" | "workspace-commands" => "workspace-write",
        "full-access" => "danger-full-access",
        _ => return Vec::new(), // Access::new validates modes at construction.
    };
    let mut args = vec!["-s".into(), sandbox.into()];
    if access.network
        && matches!(
            access.mode.as_str(),
            "workspace-write" | "workspace-commands"
        )
    {
        args.extend([
            "-c".into(),
            "sandbox_workspace_write.network_access=true".into(),
        ]);
    }
    args
}

pub fn codex_server_settings(access: &Access) -> Value {
    match access.mode.as_str() {
        "full-access" => json!({
            "sandbox":"danger-full-access", "approvalPolicy":"never",
            "sandboxPolicy":{"type":"dangerFullAccess"}
        }),
        "read-only" => json!({
            "sandbox":"read-only", "approvalPolicy":"never",
            "sandboxPolicy":{"type":"readOnly", "networkAccess":false}
        }),
        "workspace-write" | "workspace-commands" => json!({
            "sandbox":"workspace-write",
            "approvalPolicy":if access.mode == "workspace-write" { "on-request" } else { "never" },
            "sandboxPolicy":{"type":"workspaceWrite", "networkAccess":access.network}
        }),
        _ => Value::Null,
    }
}

pub fn claude_arguments(access: &Access) -> Vec<String> {
    let permission = match access.mode.as_str() {
        "read-only" => "plan",
        "workspace-write" | "workspace-commands" => "acceptEdits",
        "full-access" => "bypassPermissions",
        _ => return Vec::new(), // Access::new validates modes at construction.
    };
    let mut args = vec!["--permission-mode".into(), permission.into()];
    if matches!(
        access.mode.as_str(),
        "workspace-write" | "workspace-commands"
    ) {
        let mut rules = access.allow.clone();
        if access.mode == "workspace-commands" {
            rules.push("Bash".into());
        }
        if access.network {
            rules.extend(["WebFetch".into(), "WebSearch".into()]);
        }
        deduplicate(&mut rules);
        if !rules.is_empty() {
            args.push("--allowedTools".into());
            args.extend(rules);
        }
    }
    args
}

/// Arguments for one non-TTY Claude turn. The prompt is supplied as a stream-json user message.
pub fn claude_turn_arguments(
    configured: &[String],
    access: &Access,
    session: Option<&str>,
    model: Option<&str>,
    effort: Option<&str>,
    ephemeral: bool,
    live: bool,
) -> Vec<String> {
    let mut args = configured.to_vec();
    args.extend([
        "-p".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
    ]);
    args.extend(claude_arguments(access));
    if live {
        args.extend([
            "--input-format".into(),
            "stream-json".into(),
            "--permission-prompt-tool".into(),
            "stdio".into(),
        ]);
    }
    if ephemeral {
        args.push("--no-session-persistence".into());
    }
    if let Some(model) = model.filter(|value| !value.is_empty()) {
        args.extend(["--model".into(), model.into()]);
    }
    if let Some(effort) = effort.filter(|value| !value.is_empty()) {
        args.extend(["--effort".into(), effort.into()]);
    }
    if let Some(session) = session.filter(|value| !value.is_empty()) {
        args.extend(["--resume".into(), session.into()]);
    }
    args
}

pub fn opencode_agent(access: &Access) -> &'static str {
    if access.edits() { "build" } else { "plan" }
}

pub fn opencode_permissions(access: &Access) -> Value {
    if access.mode == "read-only" {
        return json!({"edit":"deny", "bash":"deny"});
    }
    if access.mode == "full-access" {
        return json!({"edit":"allow", "bash":"allow", "webfetch":"allow", "external_directory":"allow"});
    }

    let mut permissions = Map::new();
    permissions.insert(
        "bash".into(),
        json!(if access.mode == "workspace-commands" {
            "allow"
        } else {
            "ask"
        }),
    );
    permissions.insert(
        "webfetch".into(),
        json!(if access.network { "allow" } else { "ask" }),
    );
    for rule in &access.allow {
        let (name, pattern) = rule.split_once(':').unwrap_or((rule.as_str(), ""));
        if pattern.is_empty() {
            permissions.insert(name.to_owned(), json!("allow"));
            continue;
        }
        let mut rules = match permissions.remove(name) {
            Some(Value::Object(rules)) => rules,
            Some(other) => Map::from_iter([("*".to_owned(), other)]),
            None => Map::from_iter([("*".to_owned(), json!("ask"))]),
        };
        rules.insert(pattern.to_owned(), json!("allow"));
        permissions.insert(name.to_owned(), Value::Object(rules));
    }
    Value::Object(permissions)
}

pub fn opencode_environment(access: &Access) -> Vec<(String, String)> {
    vec![(
        "OPENCODE_PERMISSION".into(),
        serde_json::to_string(&opencode_permissions(access)).unwrap_or_else(|_| "{}".into()),
    )]
}

fn deduplicate(values: &mut Vec<String>) {
    let mut unique = Vec::with_capacity(values.len());
    for value in values.drain(..) {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    *values = unique;
}

#[cfg(test)]
#[path = "agent_access_tests.rs"]
mod tests;
