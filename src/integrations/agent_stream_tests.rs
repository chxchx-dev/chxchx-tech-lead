use super::normalize;
use serde_json::{Value, json};

fn kinds(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .map(|event| event["type"].as_str().unwrap())
        .collect()
}

fn assert_event_schema(events: &[Value]) {
    for event in events {
        assert_eq!(event["schema"], "chxchx.agent-event");
        assert_eq!(event["schema_version"], 1);
    }
}

#[test]
fn codex_jsonl_records_normalize_sessions_messages_tools_and_usage() {
    let records = [
        json!({"type":"thread.started", "thread_id":"thread-1"}),
        json!({"type":"item.completed", "item":{"id":"m1", "type":"agent_message", "text":"Voy a revisar."}}),
        json!({"type":"item.started", "item":{"id":"t1", "type":"command_execution", "command":"/bin/bash -lc 'ls -la'", "status":"in_progress"}}),
        json!({"type":"item.completed", "item":{"id":"t1", "type":"command_execution", "command":"bash -lc 'false'", "aggregated_output":"falló", "exit_code":2, "status":"completed"}}),
        json!({"type":"item.completed", "item":{"id":"f1", "type":"file_change", "status":"completed", "changes":[{"path":"src/main.rs", "kind":"update"}]}}),
        json!({"type":"turn.completed", "usage":{"input_tokens":120, "output_tokens":8}}),
    ];
    let events = records
        .iter()
        .flat_map(|record| normalize("Codex", record))
        .collect::<Vec<_>>();

    assert_eq!(
        kinds(&events),
        ["session", "message", "tool", "tool", "tool", "usage"]
    );
    assert_eq!(events[0]["session_id"], "thread-1");
    assert_eq!(events[1]["text"], "Voy a revisar.");
    assert_eq!(events[2]["title"], "ls -la");
    assert_eq!(events[2]["status"], "running");
    assert_eq!(events[3]["status"], "failed");
    assert_eq!(events[3]["output"], "falló");
    assert_eq!(events[4]["title"], "update src/main.rs");
    assert_eq!(events[5]["input_tokens"], 120);
    assert_eq!(events[5]["output_tokens"], 8);
    assert_event_schema(&events);
}

#[test]
fn codex_sandbox_denials_become_permission_cards_and_unknown_events_are_ignored() {
    let record = json!({"type":"item.completed", "item":{"id":"cmd-1", "type":"command_execution",
        "command":"curl https://example.test", "aggregated_output":"Could not resolve host: example.test", "status":"failed"}});
    let events = normalize("Codex", &record);

    assert_eq!(kinds(&events), ["tool", "permission"]);
    assert_eq!(events[1]["certain"], false);
    assert_eq!(events[1]["choices"][0]["network"], true);
    assert_eq!(events[1]["choices"][1]["mode"], "full-access");
    assert!(normalize("Codex", &json!({"type":"future.record"})).is_empty());
    assert!(normalize("OpenCode", &record).is_empty());
    assert_event_schema(&events);
}

#[test]
fn claude_stream_json_normalizes_text_tools_usage_limits_and_denials() {
    let records = [
        json!({"type":"system", "subtype":"init", "session_id":"claude-1"}),
        json!({"type":"assistant", "message":{"content":[
            {"type":"text", "text":"Listo."},
            {"type":"thinking", "thinking":"Primero reviso."},
            {"type":"tool_use", "id":"tool-1", "name":"Bash", "input":{"command":"cargo test"}}
        ]}}),
        json!({"type":"user", "message":{"content":[{"type":"tool_result", "tool_use_id":"tool-1", "content":[{"type":"text", "text":"ok"}]}]}}),
        json!({"type":"assistant", "isSidechain":true, "message":{"content":[{"type":"text", "text":"oculto"}]}}),
        json!({"type":"result", "is_error":false, "total_cost_usd":0.02,
            "usage":{"input_tokens":10, "cache_read_input_tokens":90, "cache_creation_input_tokens":5, "output_tokens":7},
            "modelUsage":{"sonnet":{"contextWindow":200000}},
            "permission_denials":[{"tool_use_id":"denied-1", "tool_name":"Bash", "tool_input":{"command":"sudo reboot"}}]}),
    ];
    let events = records
        .iter()
        .flat_map(|record| normalize("Claude", record))
        .collect::<Vec<_>>();

    assert_eq!(
        kinds(&events),
        [
            "session",
            "message",
            "reasoning",
            "tool",
            "tool",
            "usage",
            "limits",
            "permission"
        ]
    );
    assert_eq!(events[0]["session_id"], "claude-1");
    assert_eq!(events[3]["kind"], "command");
    assert_eq!(events[3]["title"], "Bash: cargo test");
    assert_eq!(events[4]["status"], "completed");
    assert_eq!(events[4]["output"], "ok");
    assert_eq!(events[5]["input_tokens"], 105);
    assert_eq!(events[5]["cost_usd"], 0.02);
    assert_eq!(events[6]["context_window"], 200000);
    assert_eq!(events[7]["choices"][0]["allow"][0], "Bash(sudo reboot)");
    assert_eq!(events[7]["choices"][1]["persist"], true);
    assert_event_schema(&events);
}

#[test]
fn claude_permission_targets_and_malformed_records_are_handled_safely() {
    let record = json!({"type":"result", "is_error":true, "subtype":"error_max_turns",
    "permission_denials":[
        {"tool_name":"WebFetch", "tool_use_id":"web-1", "tool_input":{"url":"https://docs.example.org:8443/path"}},
        {"tool_name":"Read", "tool_use_id":"read-1", "tool_input":{"file_path":"src/lib.rs"}}
    ]});
    let events = normalize("Claude", &record);

    assert_eq!(
        kinds(&events),
        ["usage", "error", "permission", "permission"]
    );
    assert_eq!(events[1]["message"], "error_max_turns");
    assert_eq!(
        events[2]["choices"][0]["allow"][0],
        "WebFetch(domain:docs.example.org)"
    );
    assert_eq!(events[3]["summary"], "Read: src/lib.rs");
    assert!(normalize("Claude", &json!("bad record")).is_empty());
    assert!(
        normalize(
            "Claude",
            &json!({"type":"assistant", "message":{"content":"bad"}})
        )
        .is_empty()
    );
    assert_event_schema(&events);
}

#[test]
fn claude_live_control_requests_become_permission_and_question_cards() {
    let permission = json!({"type":"control_request", "request_id":"request-1", "request":{
        "subtype":"can_use_tool", "tool_name":"Bash", "tool_use_id":"tool-1",
        "input":{"command":"python3 -V", "description":"Check Python"}
    }});
    let events = normalize("Claude", &permission);
    assert_eq!(kinds(&events), ["permission"]);
    assert_eq!(events[0]["id"], "tool-1");
    assert_eq!(events[0]["request_id"], "request-1");
    assert_eq!(events[0]["live"], true);
    assert_eq!(events[0]["choices"][0]["label"], "Permitir esta vez");
    assert_eq!(events[0]["choices"][1]["allow"][0], "Bash(python3:*)");

    let question = json!({"type":"control_request", "request_id":"request-2", "request":{
        "subtype":"can_use_tool", "tool_name":"AskUserQuestion", "tool_use_id":"question-1",
        "input":{"questions":[{"question":"Color?", "header":"Color", "multiSelect":true,
            "options":[{"label":"Azul", "description":"Frío"}]}]}
    }});
    let events = normalize("Claude", &question);
    assert_eq!(kinds(&events), ["question"]);
    assert_eq!(events[0]["request_id"], "request-2");
    assert_eq!(events[0]["id"], "question-1");
    assert_eq!(events[0]["questions"][0]["multiSelect"], true);
    assert_eq!(events[0]["questions"][0]["options"][0]["label"], "Azul");
    assert_event_schema(&events);
}

#[test]
fn a_claude_message_reports_what_it_used_live_once_per_message_id() {
    let record = json!({"type":"assistant", "message":{"id":"msg_1",
        "usage":{"input_tokens":10, "cache_read_input_tokens":90, "cache_creation_input_tokens":5, "output_tokens":7},
        "content":[{"type":"text", "text":"Hola"}]}});
    let events = normalize("Claude", &record);
    assert_eq!(kinds(&events), ["usage_live", "message"]);
    assert_eq!(events[0]["message_id"], "msg_1");
    assert_eq!(
        (
            events[0]["input_tokens"].as_i64(),
            events[0]["output_tokens"].as_i64()
        ),
        (Some(105), Some(7))
    );
    assert_event_schema(&events);
    // A message without an id or without usage reports nothing live.
    let bare = json!({"type":"assistant", "message":{"content":[{"type":"text", "text":"x"}]}});
    assert_eq!(kinds(&normalize("Claude", &bare)), ["message"]);
}

#[test]
fn a_claude_rate_limit_event_becomes_the_five_hour_and_weekly_windows() {
    let record = json!({"type":"rate_limit_event", "rate_limit_info":{
        "status":"allowed", "rateLimitType":"five_hour", "resetsAt":100,
        "unifiedWindows":{
            "five_hour":{"utilization":0.02, "resetsAt":100, "status":"allowed"},
            "seven_day":{"utilization":0.25, "resetsAt":200, "status":"allowed"}}}});
    let events = normalize("Claude", &record);
    assert_eq!(kinds(&events), ["limits"]);
    assert_event_schema(&events);
    assert_eq!(events[0]["primary"]["used_percent"], 2.0);
    assert_eq!(events[0]["primary"]["window_minutes"], 300);
    assert_eq!(events[0]["secondary"]["used_percent"], 25.0);
    assert_eq!(events[0]["secondary"]["resets_at"], 200);

    let bare = json!({"type":"rate_limit_event", "rate_limit_info":{"status":"allowed"}});
    assert!(normalize("Claude", &bare).is_empty());
}
