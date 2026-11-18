use super::normalize;
use serde_json::{Value, json};

fn kinds(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .map(|event| event["type"].as_str().unwrap())
        .collect()
}

#[test]
fn opencode_records_normalize_session_messages_tools_usage_and_errors() {
    let records = [
        json!({"type":"step_start", "sessionID":"ses_abc", "part":{"type":"step-start"}}),
        json!({"type":"reasoning", "part":{"type":"reasoning", "text":"Pienso."}}),
        json!({"type":"tool_use", "part":{"type":"tool", "callID":"c1", "tool":"bash", "state":{
            "status":"completed", "input":{"command":"ls"}, "output":"a.py\n", "title":"ls"}}}),
        json!({"type":"tool_use", "part":{"type":"tool", "callID":"c2", "tool":"edit", "state":{
            "status":"error", "input":{"filePath":"app.py"}, "error":"no coincide"}}}),
        json!({"type":"tool_use", "part":{"type":"tool", "callID":"c3", "tool":"memory_search", "state":{
            "status":"completed", "input":{"query":"x"}}}}),
        json!({"type":"text", "part":{"type":"text", "text":"Listo."}}),
        json!({"type":"step_finish", "part":{"type":"step-finish", "reason":"stop", "cost":0.01,
            "tokens":{"input":100, "output":7, "reasoning":3, "cache":{"read":50, "write":10}}}}),
        json!({"type":"error", "error":{"name":"APIError", "data":{"message":"sin cuota"}}}),
    ];
    let events = records.iter().flat_map(normalize).collect::<Vec<_>>();

    assert_eq!(
        kinds(&events),
        [
            "session",
            "reasoning",
            "tool",
            "tool",
            "tool",
            "message",
            "usage",
            "limits",
            "error"
        ]
    );
    assert_eq!(events[0]["session_id"], "ses_abc");
    assert_eq!(events[1]["text"], "Pienso.");
    assert_eq!(events[2]["kind"], "command");
    assert_eq!(events[2]["title"], "bash: ls");
    assert_eq!(events[2]["output"], "a.py\n");
    assert_eq!(events[3]["kind"], "file_change");
    assert_eq!(events[3]["status"], "failed");
    assert_eq!(events[3]["output"], "no coincide");
    assert_eq!(events[4]["kind"], "mcp");
    assert_eq!(events[5]["text"], "Listo.");
    assert_eq!(events[6]["input_tokens"], 160);
    assert_eq!(events[6]["output_tokens"], 7);
    assert_eq!(events[6]["cost_usd"], 0.01);
    assert_eq!(events[7]["context_tokens"], 160);
    assert_eq!(events[8]["message"], "sin cuota");
    assert!(
        events.iter().all(|event| {
            event["schema"] == "chxchx.agent-event" && event["schema_version"] == 1
        })
    );
}

#[test]
fn rejected_headless_permissions_become_cards_and_unrelated_errors_do_not() {
    let record = json!({"type":"tool_use", "part":{"type":"tool", "callID":"c9", "tool":"shell", "state":{
        "status":"error", "input":{"command":"echo hola"},
        "error":"This non-interactive run cannot ask the user for permission, so the request was rejected."}}});
    let events = normalize(&record);
    assert_eq!(kinds(&events), ["tool", "permission"]);
    assert_eq!(events[0]["kind"], "command");
    assert_eq!(events[1]["tool"], "shell");
    assert_eq!(events[1]["choices"][0]["allow"][0], "bash");
    assert_eq!(events[1]["choices"][1]["persist"], true);

    let unrelated = json!({"type":"tool_use", "part":{"type":"tool", "tool":"shell", "state":{
        "status":"error", "input":{"command":"echo hola"}, "error":"no such file"}}});
    assert_eq!(normalize(&unrelated).len(), 1);
}

#[test]
fn opencode_handles_missing_fields_and_bounds_tool_output() {
    assert!(normalize(&json!({"type":"mystery"})).is_empty());
    assert!(normalize(&json!({"type":"text", "part":"not an object"})).is_empty());
    assert_eq!(
        normalize(&json!({"type":"step_finish"}))[0]["input_tokens"],
        0
    );

    let record = json!({"type":"tool_use", "part":{"tool":"read", "state":{
        "status":"completed", "output":"x".repeat(4100)}}});
    let output = normalize(&record)[0]["output"].as_str().unwrap().to_owned();
    assert_eq!(output.chars().count(), 4002);
    assert!(output.ends_with("\n…"));
}
