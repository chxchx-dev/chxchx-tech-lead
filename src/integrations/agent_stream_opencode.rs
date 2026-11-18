//! Normalize `opencode run --format json` records into `chxchx.agent-event` v1.
//!
//! This adapter is deliberately independent of Studio so the same stream contract can be used by
//! other Rust interfaces. Unknown records are ignored and provider output is bounded before it is
//! shown or logged.

use serde_json::{Map, Value, json};

const MAX_OUTPUT_CHARS: usize = 4000;
const MAX_DETAIL_CHARS: usize = 300;

/// Convert one OpenCode JSONL record into zero or more chat events.
pub fn normalize(record: &Value) -> Vec<Value> {
    let Some(record) = record.as_object() else {
        return Vec::new();
    };
    let kind = string(record.get("type"));
    let part = record.get("part").and_then(Value::as_object);
    match kind.as_str() {
        "step_start" => record
            .get("sessionID")
            .and_then(Value::as_str)
            .map(|session| vec![event(json!({"type":"session", "session_id":session}))])
            .unwrap_or_default(),
        "text" => part
            .and_then(|part| part.get("text"))
            .filter(|text| !string(Some(text)).is_empty())
            .map(|text| {
                vec![event(
                    json!({"type":"message", "role":"assistant", "text":string(Some(text))}),
                )]
            })
            .unwrap_or_default(),
        "reasoning" => part
            .and_then(|part| part.get("text"))
            .filter(|text| !string(Some(text)).is_empty())
            .map(|text| {
                vec![event(
                    json!({"type":"reasoning", "text":string(Some(text))}),
                )]
            })
            .unwrap_or_default(),
        "tool_use" => {
            let Some(part) = part else {
                return Vec::new();
            };
            let tool = tool(part);
            let mut events = vec![tool.clone()];
            events.extend(denial(part, &string(tool.get("title"))));
            events
        }
        "step_finish" => step_finish(part),
        "error" => vec![event(json!({
            "type":"error", "message":error_message(record.get("error"))
        }))],
        _ => Vec::new(),
    }
}

fn step_finish(part: Option<&Map<String, Value>>) -> Vec<Value> {
    let tokens = part
        .and_then(|part| part.get("tokens"))
        .and_then(Value::as_object);
    let cache = tokens
        .and_then(|tokens| tokens.get("cache"))
        .and_then(Value::as_object);
    let context = count(tokens.and_then(|tokens| tokens.get("input")))
        + count(cache.and_then(|cache| cache.get("read")))
        + count(cache.and_then(|cache| cache.get("write")));
    let output = count(tokens.and_then(|tokens| tokens.get("output")));
    let cost = part
        .and_then(|part| part.get("cost"))
        .filter(|cost| cost.is_number());
    vec![
        event(json!({
            "type":"usage", "input_tokens":context, "output_tokens":output, "cost_usd":cost
        })),
        event(json!({
            "type":"limits", "context_tokens":if context == 0 { Value::Null } else { json!(context) },
            "context_window":null, "primary":null, "secondary":null, "plan":null
        })),
    ]
}

fn tool(part: &Map<String, Value>) -> Value {
    let name = value_string(part.get("tool"), "tool");
    let state = part.get("state").and_then(Value::as_object);
    let status = string(state.and_then(|state| state.get("status")));
    let failed = status == "error";
    let output = if failed {
        string(state.and_then(|state| state.get("error")))
    } else {
        string(state.and_then(|state| state.get("output")))
    };
    event(json!({
        "type":"tool",
        "id":value_string(part.get("callID"), &string(part.get("id"))),
        "kind":tool_kind(&name),
        "title":tool_title(&name, state),
        "status":if failed { "failed" } else if status == "completed" { "completed" } else { "running" },
        "output":truncate(&output, MAX_OUTPUT_CHARS)
    }))
}

fn tool_kind(name: &str) -> &'static str {
    let lowered = name.to_lowercase();
    if matches!(lowered.as_str(), "bash" | "shell") {
        "command"
    } else if matches!(
        lowered.as_str(),
        "edit" | "write" | "patch" | "multiedit" | "apply_patch"
    ) {
        "file_change"
    } else if matches!(
        lowered.as_str(),
        "grep" | "glob" | "list" | "webfetch" | "websearch" | "codesearch"
    ) {
        "search"
    } else if matches!(lowered.as_str(), "todowrite" | "todoread") {
        "todo"
    } else if lowered.contains('_') {
        "mcp"
    } else {
        "tool"
    }
}

fn tool_title(name: &str, state: Option<&Map<String, Value>>) -> String {
    let input = state
        .and_then(|state| state.get("input"))
        .and_then(Value::as_object);
    if name.eq_ignore_ascii_case("todowrite")
        && let Some(todos) = input
            .and_then(|input| input.get("todos"))
            .and_then(Value::as_array)
    {
        let entries = todos
            .iter()
            .filter_map(Value::as_object)
            .map(|todo| {
                let mark = if todo.get("status").and_then(Value::as_str) == Some("completed") {
                    '✓'
                } else {
                    '○'
                };
                format!("{mark} {}", string(todo.get("content")))
            })
            .collect::<Vec<_>>();
        if !entries.is_empty() {
            return entries.join(" · ");
        }
    }
    for key in [
        "command",
        "filePath",
        "path",
        "pattern",
        "query",
        "url",
        "description",
    ] {
        if let Some(value) = input
            .and_then(|input| input.get(key))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            return format!("{name}: {value}");
        }
    }
    state
        .and_then(|state| state.get("title"))
        .and_then(Value::as_str)
        .filter(|title| !title.is_empty())
        .unwrap_or(name)
        .to_owned()
}

fn denial(part: &Map<String, Value>, title: &str) -> Vec<Value> {
    let state = part.get("state").and_then(Value::as_object);
    let error = value_string(
        state.and_then(|state| state.get("error")),
        &string(state.and_then(|state| state.get("output"))),
    )
    .to_lowercase();
    if !["rejected permission", "cannot ask the user for permission"]
        .iter()
        .any(|marker| error.contains(marker))
    {
        return Vec::new();
    }
    let name = value_string(part.get("tool"), "tool");
    let lowered = name.to_lowercase();
    let permission = match lowered.as_str() {
        "write" | "patch" | "multiedit" | "apply_patch" => "edit",
        "shell" => "bash",
        other => other,
    }
    .to_owned();
    let label = match permission.as_str() {
        "bash" => "comandos",
        "edit" => "ediciones",
        "webfetch" => "la web",
        _ => &name,
    };
    let identifier = value_string(part.get("callID"), &value_string(part.get("id"), &name));
    vec![event(json!({
        "type":"permission", "id":identifier, "tool":name,
        "summary":truncate(title, MAX_DETAIL_CHARS), "detail":"", "certain":true,
        "choices":[
            choice("Permitir esta vez", &permission, false),
            choice(&format!("Permitir {label} en este chat"), &permission, true)
        ]
    }))]
}

fn choice(label: &str, allow: &str, persist: bool) -> Value {
    json!({"label":label, "allow":[allow], "network":false, "mode":null, "persist":persist})
}

fn error_message(error: Option<&Value>) -> String {
    if let Some(error) = error.and_then(Value::as_object) {
        let message = error
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| data.get("message"))
            .or_else(|| error.get("message"))
            .or_else(|| error.get("name"));
        if let Some(message) = message.filter(|message| !string(Some(message)).is_empty()) {
            return string(Some(message));
        }
    }
    error
        .and_then(Value::as_str)
        .filter(|message| !message.is_empty())
        .unwrap_or("OpenCode terminó el turno con error")
        .to_owned()
}

fn count(value: Option<&Value>) -> u64 {
    value.and_then(Value::as_u64).unwrap_or_default()
}

fn string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(value) => value.to_string(),
    }
}

fn value_string(value: Option<&Value>, fallback: &str) -> String {
    let value = string(value);
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value
    }
}

fn truncate(text: &str, maximum: usize) -> String {
    let mut chars = text.chars();
    let prefix = chars.by_ref().take(maximum).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}\n…")
    } else {
        prefix
    }
}

fn event(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert("schema".into(), json!("chxchx.agent-event"));
        object.insert("schema_version".into(), json!(1));
    }
    payload
}

#[cfg(test)]
#[path = "agent_stream_opencode_tests.rs"]
mod tests;
