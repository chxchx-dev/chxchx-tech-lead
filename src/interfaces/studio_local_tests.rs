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
