use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(0);

struct Project(PathBuf);

impl Project {
    fn new(name: &str) -> Self {
        let id = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "chxchx-explicit-memory-{name}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
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

fn record_messages(project: &Path, agent: &str, provider: &str, session: &str, messages: &[&str]) {
    for message in messages {
        chat_state::begin_turn(project, agent, provider, Some(session), message).unwrap();
        chat_state::end_turn(project, agent, provider, Some(session), ["Entendido."]).unwrap();
    }
}

fn recover(project: &Project) -> Vec<ExplicitMemory> {
    recover_studio_memories(project.path())
}

#[test]
fn captures_only_direct_memory_requests_from_visible_user_messages() {
    let project = Project::new("visible");
    record_messages(
        project.path(),
        "claude",
        "Claude",
        "session-1",
        &[
            "Revisa el uso de memoria RAM del proceso.",
            "guarda el archivo y compila",
            "Siento que la memoria por chat no sirve bien.",
            "Claude no recuerda la frase que le di.",
            "<system-reminder>Recuerda que AGENTS.md manda.</system-reminder> Arregla el bug.",
            "Ok, recuerda que uso pnpm y no npm.",
            "Guarda en memoria que el puerto es 8080.",
            "Remember that staging uses Postgres 16.",
            "Recuerda mi contraseña: no-guardar-esta.",
            "Recuérdame qué te dije sobre el deploy.",
        ],
    );

    let memories = recover(&project);

    assert_eq!(
        memories
            .iter()
            .map(|memory| memory.message.as_str())
            .collect::<Vec<_>>(),
        [
            "Ok, recuerda que uso pnpm y no npm.",
            "Guarda en memoria que el puerto es 8080.",
            "Remember that staging uses Postgres 16.",
        ]
    );
    assert!(memories.iter().all(|memory| memory.provider == "Claude"));
    assert!(
        memories
            .iter()
            .all(|memory| memory.session_id == "session-1")
    );
}

#[test]
fn appends_once_and_preserves_handwritten_memory_notes() {
    let project = Project::new("persist");
    let path = project.path().join(".ai/memory/PROJECT_MEMORY.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "# Memoria\n\nNota manual: usamos Qt 6.8.\n").unwrap();
    let memory = ExplicitMemory {
        provider: "Claude".to_owned(),
        session_id: "session-2".to_owned(),
        requested_at: "2026-10-07T12:00:00Z".to_owned(),
        message: "Recuerda: la frase exacta es ‘luna de cobre’.".to_owned(),
    };

    assert_eq!(
        persist_explicit_memories(project.path(), &[memory.clone(), memory.clone()]).unwrap(),
        1
    );
    assert_eq!(
        persist_explicit_memories(project.path(), std::slice::from_ref(&memory)).unwrap(),
        0
    );
    let saved = fs::read_to_string(path).unwrap();

    assert!(saved.contains("Nota manual: usamos Qt 6.8."));
    assert_eq!(saved.matches("Petición explícita de memoria").count(), 1);
    assert!(saved.contains("luna de cobre"));
    assert!(format_recovered_memories(&[memory]).contains("luna de cobre"));
}

#[test]
fn memory_file_symlinks_are_never_replaced() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let project = Project::new("symlink");
        let outside = Project::new("outside");
        let target = outside.path().join("notes.md");
        fs::write(&target, "No modificar").unwrap();
        let memory = project.path().join(".ai/memory/PROJECT_MEMORY.md");
        fs::create_dir_all(memory.parent().unwrap()).unwrap();
        symlink(&target, &memory).unwrap();
        let request = ExplicitMemory {
            provider: "Claude".to_owned(),
            session_id: "s1".to_owned(),
            requested_at: "2026-10-07".to_owned(),
            message: "Recuerda que el puerto es 8080.".to_owned(),
        };

        assert!(persist_explicit_memories(project.path(), &[request]).is_err());
        assert_eq!(fs::read_to_string(target).unwrap(), "No modificar");
    }
}

#[test]
fn recovery_is_bounded_and_deduplicated_across_sessions() {
    let project = Project::new("limits");
    for agent in 0..2 {
        let messages = (0..5)
            .map(|index| {
                if agent == 1 && index == 0 {
                    "Recuerda que la nota 0 pertenece al agente 0.".to_owned()
                } else {
                    format!("Recuerda que la nota {index} pertenece al agente {agent}.")
                }
            })
            .collect::<Vec<_>>();
        let message_refs = messages.iter().map(String::as_str).collect::<Vec<_>>();
        record_messages(
            project.path(),
            &format!("agent-{agent}"),
            "Claude",
            &format!("session-{agent}"),
            &message_refs,
        );
    }

    let memories = recover(&project);

    assert_eq!(memories.len(), 8);
    assert_eq!(
        memories
            .iter()
            .map(|memory| memory.message.as_str())
            .collect::<HashSet<_>>()
            .len(),
        8
    );
    assert!(
        memories
            .iter()
            .all(|memory| memory.message.chars().count() <= MAX_MESSAGE_CHARS)
    );
}
