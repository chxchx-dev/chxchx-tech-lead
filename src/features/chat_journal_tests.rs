use super::JOURNAL;
use crate::features::chat_state;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("chxchx-chat-journal-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn add_conversation(project: &Path, agent: &str, session: &str, user: &str, answer: &str) {
    chat_state::begin_turn(project, agent, "Claude", Some(session), user).unwrap();
    chat_state::end_turn(project, agent, "Claude", Some(session), [answer]).unwrap();
}

#[test]
fn records_visible_conversation_and_includes_it_in_new_chat_context() {
    let project = Project::new();
    add_conversation(
        project.path(),
        "claude",
        "session-1",
        "Migrar el diario de sesiones",
        "El diario ya se escribe desde Rust.",
    );
    chat_state::begin_turn(
        project.path(),
        "claude",
        "Claude",
        Some("session-1"),
        "Añade pruebas unitarias",
    )
    .unwrap();
    chat_state::end_turn(
        project.path(),
        "claude",
        "Claude",
        Some("session-1"),
        ["Las pruebas cubren el contrato."],
    )
    .unwrap();

    let saved = super::record_latest_chat(project.path(), "claude")
        .unwrap()
        .unwrap();
    let journal = fs::read_to_string(&saved).unwrap();
    let prompt = super::recent_sessions_prompt(project.path());
    let context = crate::integrations::chat_context::initial_context(project.path(), "Claude");

    assert!(journal.contains("Migrar el diario de sesiones"));
    assert!(journal.contains("Añade pruebas unitarias"));
    assert!(journal.contains("Las pruebas cubren el contrato."));
    assert!(prompt.contains("Migrar el diario de sesiones"));
    assert!(!prompt.contains("<!-- session:"));
    assert!(context.contains("Resumen automático de las últimas sesiones"));
    assert!(context.contains("Añade pruebas unitarias"));
    assert_eq!(saved, project.path().join(JOURNAL));
}

#[test]
fn updating_a_provider_session_replaces_its_entry_instead_of_duplicating_it() {
    let project = Project::new();
    add_conversation(
        project.path(),
        "claude",
        "session-1",
        "Primera tarea",
        "Primer cierre.",
    );
    super::record_latest_chat(project.path(), "claude").unwrap();
    chat_state::begin_turn(
        project.path(),
        "claude",
        "Claude",
        Some("session-1"),
        "Segunda tarea",
    )
    .unwrap();
    chat_state::end_turn(
        project.path(),
        "claude",
        "Claude",
        Some("session-1"),
        ["Segundo cierre."],
    )
    .unwrap();

    super::record_latest_chat(project.path(), "claude").unwrap();
    let journal = fs::read_to_string(project.path().join(JOURNAL)).unwrap();

    assert_eq!(journal.matches("## ").count(), 1);
    assert_eq!(journal.matches("<!-- session:").count(), 1);
    assert!(journal.contains("Segunda tarea"));
    assert!(journal.contains("Segundo cierre."));
}

#[test]
fn journal_keeps_only_the_thirty_most_recent_sessions() {
    let project = Project::new();
    for index in 0..31 {
        let agent = format!("agent-{index}");
        let session = format!("session-{index}");
        let request = format!("Solicitud {index}");
        let answer = format!("Cierre {index}");
        add_conversation(project.path(), &agent, &session, &request, &answer);
        super::record_latest_chat(project.path(), &agent).unwrap();
    }

    let journal = fs::read_to_string(project.path().join(JOURNAL)).unwrap();

    assert_eq!(journal.matches("## ").count(), 30);
    assert!(journal.contains("Solicitud 30"));
    assert!(!journal.contains("Solicitud 0"));
}

#[test]
fn missing_or_sessionless_chat_does_not_create_a_journal() {
    let project = Project::new();
    assert_eq!(
        super::record_latest_chat(project.path(), "missing").unwrap(),
        None
    );

    chat_state::begin_turn(project.path(), "claude", "Claude", None, "Pregunta").unwrap();
    assert_eq!(
        super::record_latest_chat(project.path(), "claude").unwrap(),
        None
    );
    assert!(!project.path().join(JOURNAL).exists());
}

#[test]
fn a_turn_without_an_answer_does_not_reuse_an_older_closure() {
    let project = Project::new();
    add_conversation(
        project.path(),
        "claude",
        "session-1",
        "Primera pregunta",
        "Respuesta anterior",
    );
    chat_state::begin_turn(
        project.path(),
        "claude",
        "Claude",
        Some("session-1"),
        "Pregunta interrumpida",
    )
    .unwrap();

    super::record_latest_chat(project.path(), "claude").unwrap();
    let journal = fs::read_to_string(project.path().join(JOURNAL)).unwrap();

    assert!(journal.contains("Pregunta interrumpida"));
    assert!(!journal.contains("Respuesta anterior"));
}

#[test]
fn writing_a_new_entry_never_exceeds_the_journal_read_limit() {
    let project = Project::new();
    let history = project.path().join(JOURNAL);
    fs::create_dir_all(history.parent().unwrap()).unwrap();
    let headings = (0..29)
        .map(|index| format!("## 2026-11-18 · Claude · `old-{index}`"))
        .collect::<Vec<_>>();
    let overhead = super::TITLE.len()
        + headings
            .iter()
            .map(|heading| heading.len() + 3)
            .sum::<usize>();
    let target_size = super::MAX_JOURNAL_BYTES as usize - 20;
    let body_chars = (target_size - overhead) / headings.len();
    let remainder = (target_size - overhead) % headings.len();
    let mut prior = String::from(super::TITLE);
    for (index, heading) in headings.iter().enumerate() {
        prior.push_str(heading);
        prior.push('\n');
        prior.push_str(&"x".repeat(body_chars + usize::from(index < remainder)));
        prior.push_str("\n\n");
    }
    assert!(prior.len() < super::MAX_JOURNAL_BYTES as usize);
    fs::write(&history, prior).unwrap();
    add_conversation(
        project.path(),
        "claude",
        "session-new",
        "Nueva pregunta",
        "Nuevo cierre",
    );

    super::record_latest_chat(project.path(), "claude").unwrap();
    let journal = fs::read_to_string(history).unwrap();

    assert!(journal.len() <= 256 * 1024);
    assert!(journal.contains("Nueva pregunta"));
    assert_eq!(journal.matches("## ").count(), 29);
}

#[cfg(unix)]
#[test]
fn refuses_to_replace_a_journal_symlink() {
    use std::os::unix::fs::symlink;

    let project = Project::new();
    let outside = Project::new();
    add_conversation(
        project.path(),
        "claude",
        "session-1",
        "Pregunta",
        "Respuesta",
    );
    let journal = project.path().join(JOURNAL);
    fs::create_dir_all(journal.parent().unwrap()).unwrap();
    let target = outside.path().join("untouched.md");
    fs::write(&target, "Contenido ajeno").unwrap();
    symlink(&target, &journal).unwrap();

    assert!(super::record_latest_chat(project.path(), "claude").is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "Contenido ajeno");
}
