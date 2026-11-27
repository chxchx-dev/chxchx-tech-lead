//! Normalize provider JSONL records into the versioned event contract consumed by Studio.
//!
//! Codex `exec --json` and Claude `stream-json` are normalized here. Unknown and malformed
//! records are ignored so provider additions do not break an active chat.

use serde_json::{Map, Value, json};

const MAX_OUTPUT_CHARS: usize = 4000;
const MAX_PERMISSION_DETAIL_CHARS: usize = 300;

/// Convert one raw provider record into zero or more `chxchx.agent-event` v1 events.
pub fn normalize(provider: &str, record: &Value) -> Vec<Value> {
    let Some(record) = record.as_object() else {
        return Vec::new();
    };
    match provider {
        "Codex" => codex(record),
        "Claude" => claude(record),
        "OpenCode" => super::agent_stream_opencode::normalize(&Value::Object(record.clone())),
        _ => Vec::new(),
    }
}

fn codex(record: &Map<String, Value>) -> Vec<Value> {
    let kind = string(record.get("type"));
    match kind.as_str() {
        "thread.started" => record
            .get("thread_id")
            .and_then(Value::as_str)
            .map(|id| vec![event(json!({"type":"session", "session_id":id}))])
            .unwrap_or_default(),
        "turn.completed" => {
            let usage_data = record.get("usage").and_then(Value::as_object);
            vec![usage(
                usage_data.and_then(|value| value.get("input_tokens")),
                usage_data.and_then(|value| value.get("output_tokens")),
                None,
            )]
        }
        "turn.failed" | "error" => {
            let message = record
                .get("error")
                .and_then(Value::as_object)
                .and_then(|error| error.get("message"))
                .or_else(|| record.get("message"));
            vec![event(json!({
                "type":"error",
                "message": if string(message).is_empty() { "Codex terminó el turno con error".to_owned() } else { string(message) }
            }))]
        }
        "item.started" | "item.updated" | "item.completed" => {
            let Some(item) = record.get("item").and_then(Value::as_object) else {
                return Vec::new();
            };
            let item_type = string(item.get("type"));
            if item_type == "agent_message" && kind == "item.completed" {
                return vec![event(
                    json!({"type":"message", "role":"assistant", "text":string(item.get("text"))}),
                )];
            }
            if item_type == "reasoning" && kind == "item.completed" {
                return vec![event(
                    json!({"type":"reasoning", "text":string(item.get("text"))}),
                )];
            }
            if item_type == "error" {
                return vec![event(
                    json!({"type":"error", "message":string(item.get("message"))}),
                )];
            }
            let mut events = codex_tool(item).into_iter().collect::<Vec<_>>();
            if kind == "item.completed" {
                events.extend(codex_block(item));
            }
            events
        }
        _ => Vec::new(),
    }
}

fn codex_tool(item: &Map<String, Value>) -> Option<Value> {
    let item_type = string(item.get("type"));
    let status = string(item.get("status"));
    let (kind, title, output) = match item_type.as_str() {
        "command_execution" => {
            let mut status = status;
            if status == "completed"
                && item
                    .get("exit_code")
                    .and_then(Value::as_i64)
                    .is_some_and(|code| code != 0)
            {
                status = "failed".to_owned();
            }
            return Some(tool(
                string(item.get("id")),
                "command",
                strip_shell(&string(item.get("command"))),
                status_of(&status),
                string(item.get("aggregated_output")),
            ));
        }
        "file_change" => {
            let changes = item.get("changes").and_then(Value::as_array);
            let title = changes
                .into_iter()
                .flatten()
                .filter_map(Value::as_object)
                .map(|change| {
                    format!(
                        "{} {}",
                        value_string(change.get("kind"), "update"),
                        string(change.get("path"))
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            (
                "file_change",
                if title.is_empty() {
                    "Cambios de archivos".to_owned()
                } else {
                    title
                },
                String::new(),
            )
        }
        "mcp_tool_call" => (
            "mcp",
            format!(
                "{}.{}",
                value_string(item.get("server"), "mcp"),
                string(item.get("tool"))
            ),
            String::new(),
        ),
        "web_search" => (
            "search",
            value_string(item.get("query"), "Búsqueda web"),
            String::new(),
        ),
        "todo_list" => {
            let title = item
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_object)
                .map(|entry| {
                    let mark = if entry
                        .get("completed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        '✓'
                    } else {
                        '○'
                    };
                    format!("{mark} {}", string(entry.get("text")))
                })
                .collect::<Vec<_>>()
                .join(" · ");
            (
                "todo",
                if title.is_empty() {
                    "Plan".to_owned()
                } else {
                    title
                },
                String::new(),
            )
        }
        _ => return None,
    };
    Some(tool(
        string(item.get("id")),
        kind,
        title,
        status_of(&status),
        output,
    ))
}

fn codex_block(item: &Map<String, Value>) -> Vec<Value> {
    if string(item.get("type")) != "command_execution" {
        return Vec::new();
    }
    let output = string(item.get("aggregated_output"));
    let lowered = output.to_lowercase();
    let blocked = [
        "could not resolve host",
        "temporary failure in name resolution",
        "network is unreachable",
        "read-only file system",
        "operation not permitted",
        "sandbox",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    if !blocked {
        return Vec::new();
    }
    let command = string(item.get("command"));
    let identifier = value_string(item.get("id"), &command);
    vec![permission(
        &identifier,
        "command",
        &format!("Un comando pudo bloquearlo el sandbox: {command}"),
        &tail(&output, 200),
        false,
        vec![
            choice("Permitir la red en este chat", vec![], true, None, true),
            choice(
                "Acceso total solo para reintentar",
                vec![],
                false,
                Some("full-access"),
                false,
            ),
        ],
    )]
}

fn claude(record: &Map<String, Value>) -> Vec<Value> {
    let kind = string(record.get("type"));
    if kind == "control_request" {
        return claude_live_control(record);
    }
    if kind == "system"
        && string(record.get("subtype")) == "init"
        && let Some(session_id) = record.get("session_id").and_then(Value::as_str)
    {
        return vec![event(json!({"type":"session", "session_id":session_id}))];
    }
    if kind == "rate_limit_event" {
        return claude_rate_limits(record).into_iter().collect();
    }
    if kind == "result" {
        let usage_data = record.get("usage").and_then(Value::as_object);
        let context: i64 = [
            "input_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
        ]
        .iter()
        .filter_map(|key| {
            usage_data
                .and_then(|value| value.get(*key))
                .and_then(Value::as_i64)
        })
        .sum();
        let output_tokens = usage_data.and_then(|value| value.get("output_tokens"));
        let cost = record
            .get("total_cost_usd")
            .filter(|value| value.is_number());
        let mut events = vec![usage(Some(&Value::from(context)), output_tokens, cost)];
        let window = claude_context_window(record.get("modelUsage"));
        if context != 0 || window.is_some() {
            events.push(event(json!({
                "type":"limits", "context_tokens": if context == 0 { Value::Null } else { json!(context) },
                "context_window":window, "primary":null, "secondary":null, "plan":null
            })));
        }
        if record
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let message = record.get("result").or_else(|| record.get("subtype"));
            events.push(event(json!({
                "type":"error",
                "message": if string(message).is_empty() { "Error".to_owned() } else { string(message) }
            })));
        }
        events.extend(claude_denials(record.get("permission_denials")));
        return events;
    }

    if kind != "assistant" && kind != "user"
        || record
            .get("isSidechain")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return Vec::new();
    }
    let Some(content) = record
        .get("message")
        .and_then(Value::as_object)
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut events = Vec::new();
    // What this message cost so far: shown live, and replaced by the turn's final figures.
    if kind == "assistant"
        && let Some(message) = record.get("message").and_then(Value::as_object)
        && let Some(id) = message.get("id").and_then(Value::as_str)
        && let Some(usage_data) = message.get("usage").and_then(Value::as_object)
    {
        let count = |key: &str| usage_data.get(key).and_then(Value::as_i64).unwrap_or(0);
        let input = count("input_tokens")
            + count("cache_read_input_tokens")
            + count("cache_creation_input_tokens");
        events.push(event(json!({
            "type":"usage_live", "message_id": id,
            "input_tokens": input, "output_tokens": count("output_tokens"),
        })));
    }
    for block in content.iter().filter_map(Value::as_object) {
        let block_type = string(block.get("type"));
        match (kind.as_str(), block_type.as_str()) {
            ("assistant", "text") if !string(block.get("text")).is_empty() => {
                events.push(event(
                    json!({"type":"message", "role":"assistant", "text":string(block.get("text"))}),
                ));
            }
            ("assistant", "thinking") if !string(block.get("thinking")).is_empty() => {
                events.push(event(
                    json!({"type":"reasoning", "text":string(block.get("thinking"))}),
                ));
            }
            ("assistant", "tool_use") => {
                let name = value_string(block.get("name"), "Herramienta");
                events.push(tool(
                    string(block.get("id")),
                    claude_kind(&name),
                    claude_title(&name, block.get("input")),
                    "running",
                    String::new(),
                ));
            }
            ("user", "tool_result") => events.push(tool(
                string(block.get("tool_use_id")),
                "",
                "",
                if block
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "failed"
                } else {
                    "completed"
                },
                text_of(block.get("content")),
            )),
            _ => {}
        }
    }
    events
}

fn claude_live_control(record: &Map<String, Value>) -> Vec<Value> {
    let request_id = string(record.get("request_id"));
    let Some(request) = record.get("request").and_then(Value::as_object) else {
        return Vec::new();
    };
    if request_id.is_empty() || string(request.get("subtype")) != "can_use_tool" {
        return Vec::new();
    }
    let data = request.get("input").and_then(Value::as_object);
    let name = value_string(request.get("tool_name"), "Herramienta");
    let identifier = value_string(request.get("tool_use_id"), &request_id);
    if name == "AskUserQuestion" {
        let questions = data
            .and_then(|data| data.get("questions"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
            .filter_map(|question| {
                let text = question.get("question").and_then(Value::as_str)?;
                if text.is_empty() {
                    return None;
                }
                let options = question
                    .get("options")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_object)
                    .filter_map(|option| {
                        let label = option.get("label").and_then(Value::as_str)?;
                        (!label.is_empty()).then(|| json!({
                            "label":label, "description":string(option.get("description"))
                        }))
                    })
                    .collect::<Vec<_>>();
                Some(json!({
                    "question":text,
                    "header":value_string(question.get("header"), ""),
                    "multiSelect":question.get("multiSelect").and_then(Value::as_bool).unwrap_or(false),
                    "options":options
                }))
            })
            .collect::<Vec<_>>();
        return vec![event(json!({
            "type":"question", "request_id":request_id, "id":identifier, "questions":questions
        }))];
    }

    let mut permission = claude_permission(&identifier, &name, data);
    if name == "ExitPlanMode"
        && let Some(plan) = data
            .and_then(|data| data.get("plan"))
            .and_then(Value::as_str)
    {
        permission["summary"] = json!("Aprobar el plan y empezar a trabajar");
        permission["detail"] = json!(truncate(plan, MAX_PERMISSION_DETAIL_CHARS));
    }
    let persisted = permission["choices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|choice| choice["persist"].as_bool().unwrap_or(false))
        .map(|choice| {
            json!({
                "label":choice["label"], "allow":choice["allow"], "network":choice["network"],
                "mode":null, "persist":true
            })
        })
        .collect::<Vec<_>>();
    let mut choices = vec![choice("Permitir esta vez", vec![], false, None, false)];
    choices.extend(persisted);
    permission["choices"] = json!(choices);
    permission["live"] = json!(true);
    permission["request_id"] = json!(request_id);
    vec![permission]
}

fn claude_denials(denials: Option<&Value>) -> Vec<Value> {
    denials
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|denial| {
            let name = value_string(denial.get("tool_name"), "Herramienta");
            let identifier = value_string(denial.get("tool_use_id"), &name);
            let data = denial.get("tool_input").and_then(Value::as_object);
            claude_permission(&identifier, &name, data)
        })
        .collect()
}

fn claude_permission(identifier: &str, name: &str, data: Option<&Map<String, Value>>) -> Value {
    let data = data.cloned().unwrap_or_default();
    if name == "Bash" && data.get("command").and_then(Value::as_str).is_some() {
        let command = string(data.get("command")).trim().to_owned();
        let first = command.split_whitespace().next().unwrap_or(&command);
        let mut choices = Vec::new();
        if !command.contains('(') && !command.contains(')') {
            choices.push(choice(
                "Permitir este comando",
                vec![format!("Bash({command})")],
                false,
                None,
                false,
            ));
        }
        choices.push(choice(
            &format!("Permitir «{first}» en este chat"),
            vec![format!("Bash({first}:*)")],
            false,
            None,
            true,
        ));
        return permission(
            identifier,
            name,
            &format!("Ejecutar un comando: {command}"),
            &string(data.get("description")),
            true,
            choices,
        );
    }
    if name == "WebFetch" && data.get("url").and_then(Value::as_str).is_some() {
        let url = string(data.get("url"));
        let host = url_host(&url);
        let rule = if host.is_empty() {
            "WebFetch".to_owned()
        } else {
            format!("WebFetch(domain:{host})")
        };
        return permission(
            identifier,
            name,
            &format!("Leer una página web: {url}"),
            "",
            true,
            vec![
                choice("Permitir esta vez", vec![rule], false, None, false),
                choice("Permitir la red en este chat", vec![], true, None, true),
            ],
        );
    }
    let target = ["file_path", "path", "pattern", "query", "url"]
        .iter()
        .find_map(|key| {
            data.get(*key)
                .filter(|value| !value.is_null())
                .map(|value| string(Some(value)))
        })
        .unwrap_or_default();
    let summary = if target.is_empty() {
        name.to_owned()
    } else {
        format!("{name}: {target}")
    };
    permission(
        identifier,
        name,
        &summary,
        "",
        true,
        vec![
            choice(
                "Permitir esta vez",
                vec![name.to_owned()],
                false,
                None,
                false,
            ),
            choice(
                &format!("Permitir {name} en este chat"),
                vec![name.to_owned()],
                false,
                None,
                true,
            ),
        ],
    )
}

/// What is left of the plan, as Claude reports it with each reply (`rate_limit_event`).
/// `utilization` is a 0..1 fraction; the 5 hour and weekly windows come as `unifiedWindows`.
fn claude_rate_limits(record: &Map<String, Value>) -> Option<Value> {
    let info = record.get("rate_limit_info")?;
    let window = |value: Option<&Value>, minutes: u64, fallback: Option<&Value>| {
        let value = value?;
        let used = value
            .get("utilization")
            .and_then(Value::as_f64)
            .or_else(|| fallback.and_then(Value::as_f64))?;
        (used >= 0.0).then(|| {
            json!({
                "used_percent": (used * 10000.0).round().clamp(0.0, 10000.0) / 100.0,
                "window_minutes": minutes,
                "resets_at": value.get("resetsAt").and_then(Value::as_i64).unwrap_or(0),
            })
        })
    };
    let windows = info.get("unifiedWindows");
    let mut primary = window(windows.and_then(|w| w.get("five_hour")), 300, None);
    let mut secondary = window(windows.and_then(|w| w.get("seven_day")), 10080, None);
    if primary.is_none() && secondary.is_none() {
        // Older shape: a single window named by `rateLimitType`.
        let single = window(Some(info), 0, None);
        match info.get("rateLimitType").and_then(Value::as_str) {
            Some("five_hour") => primary = single.map(|w| with_minutes(w, 300)),
            Some(kind) if kind.starts_with("seven_day") => {
                secondary = single.map(|w| with_minutes(w, 10080));
            }
            _ => {}
        }
    }
    if primary.is_none() && secondary.is_none() {
        return None;
    }
    Some(event(json!({
        "type":"limits", "context_tokens":null, "context_window":null,
        "primary":primary, "secondary":secondary, "plan":null
    })))
}

fn with_minutes(mut window: Value, minutes: u64) -> Value {
    window["window_minutes"] = json!(minutes);
    window
}

fn claude_context_window(model_usage: Option<&Value>) -> Option<u64> {
    model_usage?
        .as_object()?
        .values()
        .filter_map(|entry| entry.get("contextWindow").and_then(Value::as_u64))
        .max()
}

fn claude_kind(name: &str) -> &'static str {
    match name {
        "Bash" => "command",
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => "file_change",
        "WebSearch" | "WebFetch" | "Grep" | "Glob" => "search",
        "TodoWrite" => "todo",
        _ if name.starts_with("mcp__") => "mcp",
        _ => "tool",
    }
}

fn claude_title(name: &str, raw_input: Option<&Value>) -> String {
    let data = raw_input.and_then(Value::as_object);
    for key in [
        "command",
        "file_path",
        "notebook_path",
        "pattern",
        "query",
        "url",
        "description",
    ] {
        if let Some(value) = data
            .and_then(|data| data.get(key))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            return format!("{name}: {value}");
        }
    }
    name.to_owned()
}

fn text_of(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_object)
            .map(|item| string(item.get("text")))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn usage(input: Option<&Value>, output: Option<&Value>, cost: Option<&Value>) -> Value {
    let input = input.and_then(Value::as_i64).unwrap_or(0);
    let output = output.and_then(Value::as_i64).unwrap_or(0);
    let cost = cost.and_then(Value::as_f64);
    event(json!({"type":"usage", "input_tokens":input, "output_tokens":output, "cost_usd":cost}))
}

fn tool(id: String, kind: &str, title: impl AsRef<str>, status: &str, output: String) -> Value {
    json!({
        "schema":"chxchx.agent-event", "schema_version":1, "type":"tool", "id":id,
        "kind":kind, "title":title.as_ref(), "status":status, "output":truncate(&output, MAX_OUTPUT_CHARS)
    })
}

fn permission(
    id: &str,
    tool_name: &str,
    summary: &str,
    detail: &str,
    certain: bool,
    choices: Vec<Value>,
) -> Value {
    event(json!({
        "type":"permission", "id":id, "tool":tool_name,
        "summary":truncate(summary, MAX_PERMISSION_DETAIL_CHARS),
        "detail":truncate(detail, MAX_PERMISSION_DETAIL_CHARS), "certain":certain, "choices":choices
    }))
}

fn choice(
    label: &str,
    allow: Vec<String>,
    network: bool,
    mode: Option<&str>,
    persist: bool,
) -> Value {
    json!({"label":label, "allow":allow, "network":network, "mode":mode, "persist":persist})
}

fn event(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object.insert("schema".to_owned(), json!("chxchx.agent-event"));
        object.insert("schema_version".to_owned(), json!(1));
    }
    payload
}

fn string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(value) => value.to_string(),
    }
}

fn value_string(value: Option<&Value>, default: &str) -> String {
    let value = string(value);
    if value.is_empty() {
        default.to_owned()
    } else {
        value
    }
}

fn status_of(status: &str) -> &'static str {
    match status {
        "completed" => "completed",
        "failed" | "declined" | "error" => "failed",
        _ => "running",
    }
}

fn strip_shell(command: &str) -> String {
    for prefix in ["/bin/bash -lc ", "bash -lc ", "/bin/sh -c ", "sh -c "] {
        if let Some(inner) = command.strip_prefix(prefix) {
            let chars = inner.chars().collect::<Vec<_>>();
            if chars.len() >= 2 && chars.first() == chars.last() && matches!(chars[0], '\'' | '"') {
                return chars[1..chars.len() - 1].iter().collect();
            }
            return inner.to_owned();
        }
    }
    command.to_owned()
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let first = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{first}\n…")
    } else {
        first
    }
}

fn tail(text: &str, max_chars: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    chars[chars.len().saturating_sub(max_chars)..]
        .iter()
        .collect()
}

fn url_host(url: &str) -> String {
    let Some((_, rest)) = url.split_once("://") else {
        return String::new();
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or("");
    if let Some(bracketed) = host_port.strip_prefix('[') {
        return bracketed
            .split_once(']')
            .map(|(host, _)| host.to_owned())
            .unwrap_or_default();
    }
    host_port.split(':').next().unwrap_or("").to_owned()
}

#[cfg(test)]
#[path = "agent_stream_tests.rs"]
mod tests;
