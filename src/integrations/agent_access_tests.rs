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
