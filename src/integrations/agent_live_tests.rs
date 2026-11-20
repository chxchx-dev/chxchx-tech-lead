use super::*;

#[test]
fn claude_permission_answers_use_the_original_tool_input() {
    let original = json!({"command":"python3 -V", "description":"Check Python"});
    let answer = claude_response(
        "request-1",
        &original,
        &json!({"type":"answer", "request_id":"request-1", "behavior":"allow"}),
    );

    assert_eq!(answer["type"], "control_response");
    assert_eq!(answer["response"]["subtype"], "success");
    assert_eq!(answer["response"]["request_id"], "request-1");
    assert_eq!(answer["response"]["response"]["updatedInput"], original);
}

#[test]
fn claude_question_answers_are_stringified_and_empty_answers_are_refused() {
    let original = json!({"questions":[{"question":"Color?"}]});
    let answer = claude_response(
        "request-2",
        &original,
        &json!({"answers":{"Color?":"Azul"}}),
    );
    assert_eq!(answer["response"]["response"]["behavior"], "allow");
    assert_eq!(
        answer["response"]["response"]["updatedInput"]["answers"]["Color?"],
        "Azul"
    );

    let skipped = claude_response("request-2", &original, &json!({"answers":{}}));
    assert_eq!(skipped["response"]["response"]["behavior"], "deny");
    assert!(
        skipped["response"]["response"]["message"]
            .as_str()
            .unwrap()
            .contains("no respondió")
    );
}

#[test]
fn claude_denials_and_unsupported_control_requests_get_a_response() {
    let denied = claude_response("request-3", &json!({}), &json!({"behavior":"deny"}));
    assert_eq!(denied["response"]["response"]["behavior"], "deny");
    assert!(
        denied["response"]["response"]["message"]
            .as_str()
            .unwrap()
            .contains("rechazó")
    );
    assert_eq!(
        claude_unsupported_response("request-4")["response"]["subtype"],
        "error"
    );
}
