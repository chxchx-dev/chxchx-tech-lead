//! Live provider control responses that the Studio chat can answer.

use serde_json::{Map, Value, json};

const DENIED: &str = "El usuario rechazó esta acción desde ChxChx Studio. No la reintentes; sigue sin ella o pregunta qué prefiere.";
const UNANSWERED: &str =
    "El usuario no respondió la pregunta. Continúa con tu mejor criterio o explica qué necesitas.";

/// Convert Studio's generic answer to Claude's stdio control protocol.
pub fn claude_response(request_id: &str, original: &Value, answer: &Value) -> Value {
    let payload = if let Some(answers) = answer.get("answers").and_then(Value::as_object) {
        if answers.is_empty() {
            json!({"behavior":"deny", "message":UNANSWERED})
        } else {
            let mut updated = original.as_object().cloned().unwrap_or_default();
            updated.insert("answers".into(), string_map(answers));
            json!({"behavior":"allow", "updatedInput":updated})
        }
    } else if answer.get("behavior").and_then(Value::as_str) == Some("allow") {
        json!({"behavior":"allow", "updatedInput":original})
    } else {
        json!({"behavior":"deny", "message":DENIED})
    };
    json!({
        "type":"control_response",
        "response":{"subtype":"success", "request_id":request_id, "response":payload}
    })
}

/// Refuse a control request type we cannot handle, so the provider never waits indefinitely.
pub fn claude_unsupported_response(request_id: &str) -> Value {
    json!({
        "type":"control_response",
        "response":{"subtype":"error", "error":"Solicitud no soportada", "request_id":request_id}
    })
}

fn string_map(values: &Map<String, Value>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    Value::String(
                        value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string()),
                    ),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "agent_live_tests.rs"]
mod tests;
