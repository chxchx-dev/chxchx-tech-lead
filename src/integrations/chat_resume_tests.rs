use super::*;
use crate::features::chat_state;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("chxchx-chat-resume-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn saved_chat(project: &Path, session: &str) {
    chat_state::begin_turn(
        project,
        "codex",
        "Codex",
        Some(session),
        "Migrar el instalador",
    )
    .unwrap();
    chat_state::end_turn(
        project,
        "codex",
        "Codex",
        Some(session),
        ["Empecemos por Windows."],
    )
    .unwrap();
    chat_state::mark_unverified(project, "codex").unwrap();
}

#[test]
fn a_missing_provider_session_rebuilds_a_bounded_recap_from_saved_chat() {
    let project = Scratch::new();
    saved_chat(&project.0, "session-1");

    let (session, recap) = resolve_resume_with(
        &project.0,
        "Codex",
        "codex",
        Some("session-1"),
        |_, _, _| Some(false),
    );

    assert!(session.is_none());
    assert!(recap.contains("Persona: Migrar el instalador"));
    assert!(recap.contains("Agente: Empecemos por Windows."));
    assert!(recap.find("Persona:").unwrap() < recap.find("Agente:").unwrap());
    assert!(recap.ends_with("</archivo-de-contexto>"));
}

#[test]
fn known_unverifiable_or_unrelated_sessions_are_kept() {
    let project = Scratch::new();
    saved_chat(&project.0, "session-1");

    for answer in [Some(true), None] {
        assert_eq!(
            resolve_resume_with(
                &project.0,
                "Codex",
                "codex",
                Some("session-1"),
                |_, _, _| answer,
            ),
            (Some("session-1".into()), String::new())
        );
    }
    assert_eq!(
        resolve_resume_with(
            &project.0,
            "Codex",
            "codex",
            Some("another-session"),
            |_, _, _| Some(false),
        ),
        (Some("another-session".into()), String::new())
    );
    assert_eq!(
        resolve_resume_with(&project.0, "Codex", "codex", None, |_, _, _| Some(false)),
        (None, String::new())
    );
}

#[test]
fn verified_state_does_not_trigger_a_provider_scan() {
    let project = Scratch::new();
    chat_state::end_turn(
        &project.0,
        "codex",
        "Codex",
        Some("session-1"),
        ["respuesta"],
    )
    .unwrap();
    let mut checked = false;

    let result = resolve_resume_with(
        &project.0,
        "Codex",
        "codex",
        Some("session-1"),
        |_, _, _| {
            checked = true;
            Some(false)
        },
    );

    assert_eq!(result, (Some("session-1".into()), String::new()));
    assert!(!checked);
}
