use super::*;
use std::fs;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-adapter-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn access(mode: &str, network: bool, allow: &[&str]) -> Access {
    Access::new(
        mode,
        network,
        allow.iter().map(|rule| (*rule).to_owned()).collect(),
    )
    .unwrap()
}

fn turn<'a>(
    provider: Provider,
    command: &'a [String],
    access: &'a Access,
    cwd: &'a Path,
) -> TurnRequest<'a> {
    TurnRequest {
        provider,
        command,
        executable: Path::new("/opt/bin/agent"),
        cwd,
        prompt: "hola",
        session: None,
        model: None,
        effort: None,
        images: &[],
        access,
        opencode_major: None,
    }
}

#[test]
fn the_description_must_be_the_services_and_speak_this_contract() {
    let good = r#"{"schema":"chxchx.agent-adapter","contract_version":1,"version":"0.1.0","providers":["Codex","OpenCode"]}"#;
    let description = parse_description(good).unwrap();
    assert_eq!(description.version, "0.1.0");
    assert_eq!(description.providers, ["Codex", "OpenCode"]);

    assert!(
        parse_description(r#"{"schema":"otro","contract_version":1}"#)
            .unwrap_err()
            .contains("no es el servicio")
    );
    let newer = r#"{"schema":"chxchx.agent-adapter","contract_version":2,"providers":["Codex"]}"#;
    assert!(parse_description(newer).unwrap_err().contains("contrato"));
    assert!(parse_description("no es json").is_err());
}

#[test]
fn the_major_version_is_the_first_number() {
    for (text, major) in [
        ("1.19.5\n", 1),
        ("v2.2.6", 2),
        ("opencode 2.0.1", 2),
        ("10.1.0", 10),
    ] {
        assert_eq!(parse_major(text), Some(major), "{text}");
    }
    for text in ["", "unknown", "7", "v.x"] {
        assert_eq!(parse_major(text), None, "{text}");
    }
}

#[test]
fn a_checkouts_environment_is_found_from_the_project_and_from_the_binary() {
    let root = scratch("checkout");
    let service = checkout_environment(&root);
    fs::create_dir_all(service.parent().unwrap()).unwrap();
    fs::write(&service, "#!/bin/sh\n").unwrap();
    let nowhere = OsString::new();

    assert_eq!(
        candidates(&nowhere, None, Some(&root)),
        std::slice::from_ref(&service)
    );
    let binary = root.join("target").join("debug").join("chxchx-tech");
    assert_eq!(
        candidates(&nowhere, Some(&binary), None),
        std::slice::from_ref(&service)
    );
    assert!(candidates(&nowhere, None, Some(&scratch("empty"))).is_empty());
}

#[test]
fn a_binary_finds_the_service_installed_next_to_it_before_the_path() {
    let root = scratch("installed");
    let beside = root.join(file_name());
    fs::write(&beside, "").unwrap();
    let elsewhere = scratch("elsewhere");
    fs::write(elsewhere.join(file_name()), "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // The PATH only offers what can be run.
        fs::set_permissions(
            elsewhere.join(file_name()),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }

    let found = candidates(
        &std::env::join_paths([&elsewhere]).unwrap(),
        Some(&root.join("chxchx-tech")),
        None,
    );

    assert_eq!(found, [beside, elsewhere.join(file_name())]);
}

#[test]
fn a_codex_request_carries_the_policy_rust_decided() {
    let dir = scratch("codex");
    let command = ["codex", "-c", "model_provider=x", "--search", "-m", "gpt-x"].map(String::from);
    let access = access("workspace-write", true, &[]);
    let mut spec = turn(Provider::Codex, &command, &access, &dir);
    spec.session = Some("thread-1");
    spec.effort = Some("high");

    let request = request(&spec).unwrap();

    assert_eq!(request["schema"], REQUEST_SCHEMA);
    assert_eq!(request["schema_version"], 1);
    assert_eq!(request["provider"], "Codex");
    assert_eq!(
        request["argv"],
        json!(["/opt/bin/agent", "app-server", "-c", "model_provider=x"]),
        "only the -c overrides of the configured command survive"
    );
    assert_eq!(
        request["model"], "gpt-x",
        "the configured model is the default"
    );
    assert_eq!(request["effort"], "high");
    assert_eq!(request["session_id"], "thread-1");
    assert_eq!(request["cwd"], dir.to_string_lossy().as_ref());
    assert_eq!(request["codex"]["sandbox"], "workspace-write");
    assert_eq!(request["codex"]["approvalPolicy"], "on-request");
    assert_eq!(request["codex"]["sandboxPolicy"]["networkAccess"], true);
    assert!(request["env"]["PATH"].as_str().is_some());
}

#[test]
fn a_chosen_model_wins_over_the_configured_one() {
    let dir = scratch("codex-model");
    let command = ["codex", "-m", "viejo"].map(String::from);
    let access = access("read-only", false, &[]);
    let mut spec = turn(Provider::Codex, &command, &access, &dir);
    spec.model = Some("nuevo");

    assert_eq!(request(&spec).unwrap()["model"], "nuevo");
}

#[test]
fn an_opencode_request_names_the_agent_the_version_and_its_permissions() {
    let dir = scratch("opencode");
    let command = ["opencode".to_owned()];
    let edits = access("workspace-write", false, &["bash:python3 *"]);
    let mut spec = turn(Provider::OpenCode, &command, &edits, &dir);
    spec.opencode_major = Some(1);

    let v1 = request(&spec).unwrap();
    assert_eq!(v1["provider"], "OpenCode");
    assert_eq!(
        v1["argv"],
        json!([
            "/opt/bin/agent",
            "serve",
            "--port",
            "0",
            "--hostname",
            "127.0.0.1"
        ])
    );
    assert_eq!(v1["opencode"], json!({"agent": "build", "major": 1}));
    let permission: Value =
        serde_json::from_str(v1["env"]["OPENCODE_PERMISSION"].as_str().unwrap()).unwrap();
    assert_eq!(
        permission["bash"],
        json!({"*": "ask", "python3 *": "allow"})
    );
    assert!(v1["env"].get("OPENCODE_CONFIG_CONTENT").is_none());

    spec.opencode_major = Some(2);
    let v2 = request(&spec).unwrap();
    assert!(v2["env"].get("OPENCODE_PERMISSION").is_none());
    assert!(
        v2["env"]["OPENCODE_CONFIG_CONTENT"]
            .as_str()
            .unwrap()
            .contains("python3 *")
    );

    let reading = access("read-only", false, &[]);
    spec.access = &reading;
    assert_eq!(request(&spec).unwrap()["opencode"]["agent"], "plan");
}

#[test]
fn the_service_only_drives_codex_and_opencode() {
    let dir = scratch("other");
    let command = ["claude".to_owned()];
    let access = access("read-only", false, &[]);
    for provider in [Provider::Claude, Provider::Other] {
        assert!(request(&turn(provider, &command, &access, &dir)).is_err());
    }
}
