use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("chxchx-chat-context-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn block_text<'a>(context: &'a str, path: &str) -> &'a str {
    let start = format!("<archivo-de-contexto ruta=\"{path}\">\n");
    let body = context.split_once(&start).unwrap().1;
    body.split_once("\n</archivo-de-contexto>").unwrap().0
}

fn content_chars(block: &str) -> usize {
    block
        .split_once("\n[…recortado a ")
        .map_or(block, |(content, _)| content)
        .chars()
        .count()
}

#[test]
fn inline_context_is_bounded_per_file_and_in_total() {
    let project = Scratch::new();
    for path in [
        "AGENTS.md",
        ".ai/PROJECT.md",
        ".ai/CURRENT_STATE.md",
        ".ai/HANDOFF.md",
        ".ai/memory/PROJECT_MEMORY.md",
    ] {
        project.write(path, &"x".repeat(20_000));
    }

    let context = inline_project_context(&project.0, "Claude");
    let payloads = [
        ("AGENTS.md", block_text(&context, "AGENTS.md")),
        (".ai/PROJECT.md", block_text(&context, ".ai/PROJECT.md")),
        (
            ".ai/CURRENT_STATE.md",
            block_text(&context, ".ai/CURRENT_STATE.md"),
        ),
    ];

    assert!(
        payloads
            .iter()
            .all(|(_, text)| content_chars(text) <= 12_000)
    );
    assert!(
        payloads
            .iter()
            .map(|(_, text)| content_chars(text))
            .sum::<usize>()
            <= 36_000
    );
    assert!(context.contains("recortado a 12000 caracteres"));
    assert!(!context.contains(".ai/HANDOFF.md"));
}

#[test]
fn context_file_reads_stop_at_the_byte_limit() {
    let project = Scratch::new();
    project.write("context.md", &"x".repeat(100_000));

    let (read, truncated) = read_prefix(&project.0.join("context.md"), 128).unwrap();

    assert_eq!(read.len(), 128);
    assert!(truncated);
}

#[test]
fn truncated_whitespace_context_still_reports_omitted_content() {
    let project = Scratch::new();
    project.write(
        "AGENTS.md",
        &format!("{}contenido importante", " ".repeat(100_000)),
    );

    let context = inline_project_context(&project.0, "Claude");

    assert!(context.contains("contenido restante omitido por el límite de lectura"));
}

#[test]
fn codex_context_skips_agents_and_missing_context_has_direct_instruction() {
    let project = Scratch::new();
    project.write("AGENTS.md", "No debe duplicarse");
    project.write(".ai/PROJECT.md", "Arquitectura del proyecto");

    let context = inline_project_context(&project.0, "Codex");
    assert!(!context.contains("No debe duplicarse"));
    assert!(context.contains("Arquitectura del proyecto"));
    assert!(!context.contains("ruta=\"AGENTS.md\""));

    let empty = Scratch::new();
    let missing = inline_project_context(&empty.0, "Claude");
    assert!(missing.contains("no los busques ni ejecutes comandos"));
}

#[cfg(unix)]
#[test]
fn inline_context_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    let project = Scratch::new();
    let outside = Scratch::new();
    outside.write("private.md", "contenido fuera del proyecto");
    symlink(outside.0.join("private.md"), project.0.join("AGENTS.md")).unwrap();
    project.write(".ai/PROJECT.md", "nota permitida");

    let context = inline_project_context(&project.0, "Claude");
    assert!(!context.contains("contenido fuera del proyecto"));
    assert!(context.contains("nota permitida"));
    assert!(!context_paths(&project.0).contains(&project.0.join("AGENTS.md")));
}

#[test]
fn first_message_pins_memory_scope_and_handoff_precedes_recap() {
    let project = Scratch::new();
    project.write(
        ".ai/chxchx-tech.toml",
        "memory_project = \"chxchx-tech-lead-a2bf5e\"\n",
    );
    project.write(".ai/PROJECT.md", "Arquitectura vigente");

    let prompt = first_message(
        &project.0,
        "Claude",
        "Continúa el trabajo",
        "memoria recuperada",
        "estado de handoff",
        "recap anterior",
    );

    assert!(
        prompt.contains("`chxchx-tech-lead-a2bf5e`"),
        "memory scope missing from prompt: {prompt}"
    );
    assert!(prompt.contains("memory_project"));
    assert!(prompt.contains("memoria recuperada"));
    assert!(prompt.contains("estado de handoff"));
    assert!(!prompt.contains("recap anterior"));
    assert!(prompt.ends_with(&format!("{CHAT_PANEL_REQUEST_MARKER}\nContinúa el trabajo")));
}

#[test]
fn absent_memory_scope_disables_basic_memory() {
    let project = Scratch::new();
    project.write(".ai/chxchx-tech.toml", "auto_compact_context = true\n");

    let prompt = new_chat_prompt(&project.0);
    assert!(prompt.contains("no consultes ni escribas Basic Memory"));
}

#[test]
fn context_paths_are_project_relative_and_limited_to_the_known_set() {
    let project = Scratch::new();
    project.write("AGENTS.md", "instrucciones");
    project.write(".ai/PROJECT.md", "arquitectura");
    let paths = context_paths(&project.0);

    assert_eq!(
        paths,
        [
            project.0.join("AGENTS.md"),
            project.0.join(".ai/PROJECT.md"),
        ]
    );
}
