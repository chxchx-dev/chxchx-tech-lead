use super::*;

fn access(mode: &str, network: bool, allow: &[&str]) -> Access {
    Access::new(
        mode,
        network,
        allow.iter().map(|rule| (*rule).to_owned()).collect(),
    )
    .unwrap()
}

#[test]
fn access_modes_are_validated_and_default_to_read_only() {
    assert_eq!(Access::default().mode, "read-only");
    assert!(!access("read-only", false, &[]).edits());
    assert!(Access::new("unrestricted-ish", false, Vec::new()).is_err());
}

#[test]
fn claude_modes_and_approved_rules_match_the_live_chat_policy() {
    assert_eq!(
        claude_arguments(&access("read-only", true, &["Bash"])),
        ["--permission-mode", "plan"]
    );
    assert_eq!(
        claude_arguments(&access(
            "workspace-write",
            true,
            &["Bash(ls:*)", "WebSearch", "Bash(ls:*)"]
        )),
        [
            "--permission-mode",
            "acceptEdits",
            "--allowedTools",
            "Bash(ls:*)",
            "WebSearch",
            "WebFetch",
        ]
    );
    assert_eq!(
        claude_arguments(&access("workspace-commands", false, &[])),
        ["--permission-mode", "acceptEdits", "--allowedTools", "Bash"]
    );
    assert_eq!(
        claude_arguments(&access("full-access", true, &["Bash"])),
        ["--permission-mode", "bypassPermissions"]
    );
}

#[test]
fn claude_turn_arguments_keep_configured_flags_and_add_live_stream_options() {
    let args = claude_turn_arguments(
        &["--settings".into(), "custom.json".into()],
        &access("workspace-write", true, &["Bash(ls:*)"]),
        Some("session-1"),
        Some("sonnet"),
        Some("high"),
        false,
        true,
    );
    assert_eq!(
        args,
        [
            "--settings",
            "custom.json",
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "acceptEdits",
            "--allowedTools",
            "Bash(ls:*)",
            "WebFetch",
            "WebSearch",
            "--input-format",
            "stream-json",
            "--permission-prompt-tool",
            "stdio",
            "--model",
            "sonnet",
            "--effort",
            "high",
            "--resume",
            "session-1"
        ]
    );
}

#[test]
fn codex_cli_and_app_server_settings_keep_their_distinct_approval_rules() {
    assert_eq!(
        codex_arguments(&access("workspace-write", true, &[])),
        [
            "-s",
            "workspace-write",
            "-c",
            "sandbox_workspace_write.network_access=true"
        ]
    );
    assert_eq!(
        codex_server_settings(&access("workspace-write", true, &[])),
        json!({"sandbox":"workspace-write", "approvalPolicy":"on-request",
            "sandboxPolicy":{"type":"workspaceWrite", "networkAccess":true}})
    );
    assert_eq!(
        codex_server_settings(&access("workspace-commands", false, &[]))["approvalPolicy"],
        "never"
    );
    assert_eq!(
        codex_server_settings(&access("full-access", false, &[]))["sandboxPolicy"]["type"],
        "dangerFullAccess"
    );
}

#[test]
fn opencode_permission_maps_cover_read_only_full_and_chat_allow_rules() {
    assert_eq!(opencode_agent(&access("read-only", false, &[])), "plan");
    assert_eq!(
        opencode_permissions(&access("read-only", false, &[])),
        json!({"edit":"deny", "bash":"deny"})
    );
    assert_eq!(
        opencode_permissions(&access("full-access", false, &[])),
        json!({"edit":"allow", "bash":"allow", "webfetch":"allow", "external_directory":"allow"})
    );
    assert_eq!(
        opencode_permissions(&access("workspace-write", false, &["bash:git *", "edit"])),
        json!({"bash":{"*":"ask", "git *":"allow"}, "webfetch":"ask", "edit":"allow"})
    );
    assert_eq!(
        opencode_environment(&access("workspace-write", true, &[]))[0].0,
        "OPENCODE_PERMISSION"
    );
}

#[test]
fn opencode_two_gets_an_ordered_ruleset_where_approved_rules_win() {
    let rules = |mode: &str, network: bool, allow: &[&str]| -> Vec<serde_json::Value> {
        let access = Access::new(
            mode,
            network,
            allow.iter().map(|rule| (*rule).to_owned()).collect(),
        )
        .unwrap();
        let env = opencode_environment_for(&access, Some(2));
        let (name, value) = env.first().cloned().unwrap_or_default();
        assert_eq!(name, "OPENCODE_CONFIG_CONTENT");
        serde_json::from_str::<serde_json::Value>(&value).unwrap()["permissions"]
            .as_array()
            .unwrap()
            .clone()
    };
    let rule = |action: &str, effect: &str, resource: &str| serde_json::json!({"action": action, "resource": resource, "effect": effect});

    assert_eq!(
        rules("read-only", false, &[]),
        [rule("edit", "deny", "*"), rule("shell", "deny", "*")]
    );
    assert!(rules("workspace-write", false, &[]).contains(&rule("shell", "ask", "*")));
    let commands = rules("workspace-commands", true, &[]);
    assert!(commands.contains(&rule("shell", "allow", "*")));
    assert!(commands.contains(&rule("webfetch", "allow", "*")));
    let approved = rules("workspace-write", false, &["bash:git *", "webfetch"]);
    assert_eq!(
        approved[approved.len() - 2..],
        [
            rule("shell", "allow", "git *"),
            rule("webfetch", "allow", "*")
        ]
    );
}

#[test]
fn opencode_two_full_access_sets_nothing_and_older_versions_keep_the_map() {
    let full = Access::new("full-access", false, Vec::new()).unwrap();
    assert!(opencode_environment_for(&full, Some(2)).is_empty());
    let read = Access::new("read-only", false, Vec::new()).unwrap();
    for major in [None, Some(1)] {
        assert_eq!(
            opencode_environment_for(&read, major)[0].0,
            "OPENCODE_PERMISSION"
        );
    }
}
