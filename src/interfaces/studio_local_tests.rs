use super::*;
use serde_json::json;

#[test]
fn a_download_reports_its_progress_and_the_end() {
    let mut local = State::default();
    apply(
        &mut local,
        &json!({"type": "step", "step": "download", "model": "qwen2.5-coder:7b"}),
    );
    assert!(
        local
            .progress
            .text
            .starts_with("Descargando qwen2.5-coder:7b")
    );
    apply(
        &mut local,
        &json!({"type": "progress", "status": "pulling", "completed": 50, "total": 100}),
    );
    assert_eq!((local.progress.done, local.progress.total), (50, 100));
    apply(
        &mut local,
        &json!({"type": "done", "model": "ollama/x", "changed": true}),
    );
    assert!(local.result.unwrap().contains("conectado"));
}

#[test]
fn errors_and_removals_are_reported_plainly() {
    let mut local = State::default();
    apply(
        &mut local,
        &json!({"type": "error", "message": "Ollama no responde"}),
    );
    assert_eq!(local.error.as_deref(), Some("Ollama no responde"));
    apply(
        &mut local,
        &json!({"type": "done", "model": "qwen", "removed": ["qwen"]}),
    );
    assert_eq!(local.result.as_deref(), Some("Se quitó qwen."));
}

#[test]
fn local_models_are_named_the_way_opencode_names_them() {
    assert_eq!(
        model_ref("qwen2.5-coder:7b-ctx32k"),
        "ollama/qwen2.5-coder:7b-ctx32k"
    );
    assert_eq!(short("qwen2.5-coder:7b-ctx32k"), "qwen2.5-coder:7b");
    assert_eq!(short("llama3"), "llama3");
    let snapshot = Snapshot {
        configured: vec!["qwen2.5-coder:7b-ctx32k".into()],
        ..Snapshot::default()
    };
    assert_eq!(
        connected_name(&snapshot, "qwen2.5-coder:7b").as_deref(),
        Some("qwen2.5-coder:7b-ctx32k")
    );
    assert!(connected_name(&snapshot, "devstral:24b").is_none());
}

#[test]
fn a_terminal_with_a_local_model_is_its_own_agent_tab() {
    use crate::features::agents::{Agent, Provider};
    let agent = Agent {
        id: "opencode".into(),
        provider: Provider::OpenCode,
        command: vec!["opencode".into()],
        cwd: std::path::PathBuf::new(),
        executable: Some("/bin/opencode".into()),
        configured: false,
    };
    let local = with_model(&agent, "qwen2.5-coder:7b-ctx32k");
    assert_eq!(local.id, "opencode:local:qwen2.5-coder:7b-ctx32k");
    assert_eq!(
        local.command,
        ["opencode", "--model", "ollama/qwen2.5-coder:7b-ctx32k"]
    );
    assert_eq!(
        agent.command,
        ["opencode"],
        "the plain OpenCode stays as it was"
    );
}
