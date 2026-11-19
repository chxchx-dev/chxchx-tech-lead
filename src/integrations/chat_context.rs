//! Build the project context included in a fresh visual-chat session.
//!
//! Reads are bounded and limited to the known project context files. Symlinks are skipped so a
//! project cannot make Studio inline arbitrary files outside its root by linking an `.ai` file.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const CHAT_PANEL_REQUEST_MARKER: &str =
    "Primera solicitud del usuario en el panel de chat de ChxChx Studio:";

const CONTEXT_FILES: [&str; 5] = [
    "AGENTS.md",
    ".ai/PROJECT.md",
    ".ai/CURRENT_STATE.md",
    ".ai/HANDOFF.md",
    ".ai/memory/PROJECT_MEMORY.md",
];
const MAX_FILE_CHARS: usize = 12_000;
const MAX_CONTEXT_CHARS: usize = 36_000;
const MAX_FILE_READ_BYTES: usize = MAX_FILE_CHARS * 4 + 4;

const NEW_CHAT_PROMPT: &str = "Inicia una conversación nueva y económica en contexto. Antes de responder, lee AGENTS.md si existe, .ai/PROJECT.md, .ai/CURRENT_STATE.md y .ai/HANDOFF.md. No leas .ai/history/ salvo que la tarea necesite contexto anterior; ahí se archivan secciones antiguas y resúmenes de sesiones. Lee también .ai/memory/PROJECT_MEMORY.md si existe y recupera sus instrucciones explícitas de memoria, preservando literalmente frases y nombres que el usuario pidió recordar. Cuando el usuario pida explícitamente recordar algo durante esta conversación, añádelo de inmediato como entrada fechada en ese archivo, sin reemplazar otras entradas, y confirma la ruta; no esperes al cierre del trabajo. No guardes contraseñas, tokens, claves ni datos personales sensibles. Usa primero el archivo local como fuente de frases explícitas. No busques una frase con actividad o búsqueda global de Basic Memory. Recupera de Basic Memory únicamente decisiones relevantes del proyecto, las decisiones y notas relevantes para el estado y la tarea pendiente; si el usuario pregunta por una memoria explícita que no está en el archivo, busca solo esa petición en los chats recientes del mismo proyecto y guárdala si aparece; si no aparece, dilo con claridad. No cargues ni reproduzcas conversaciones completas. Resume brevemente qué contexto persistido encontraste y continúa desde el pendiente. No supongas que información no guardada en archivos de contexto o memoria está disponible. Antes de responder al terminar trabajo sustancial, guarda sin pedir acción manual: actualiza estado y handoff y usa Basic Memory solo con el ámbito exacto del proyecto indicado abajo para conocimiento duradero. No guardes saludos, preguntas triviales, secretos ni transcripciones completas.";

/// Prompt policy with the Basic Memory project pinned to this project's configuration.
pub fn new_chat_prompt(project_root: &Path) -> String {
    let memory_project = memory_project(project_root);
    let scope = match memory_project {
        Some(project) => format!(
            "El proyecto Basic Memory autorizado para este chat es `{project}` (campo `memory_project` de `.ai/chxchx-tech.toml`). Si llamas herramientas Basic Memory, especifica exactamente ese nombre en `project`; no reutilices un `project_id` tomado de otro proyecto, del historial de herramientas o del proyecto predeterminado. Si no puedes fijar ese ámbito, no llames la herramienta."
        ),
        None => "Este proyecto no declara `memory_project` en `.ai/chxchx-tech.toml`; no consultes ni escribas Basic Memory. Usa únicamente archivos locales del proyecto.".to_owned(),
    };
    format!("{NEW_CHAT_PROMPT}\n\nÁmbito de memoria obligatorio: {scope}")
}

fn memory_project(project_root: &Path) -> Option<String> {
    let config = fs::read_to_string(project_root.join(".ai/chxchx-tech.toml")).ok()?;
    let parsed = config.parse::<toml::Table>().ok()?;
    parsed
        .get("memory_project")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|project| !project.is_empty())
        .map(str::to_owned)
}

/// Inline project notes for a new chat. Codex reads `AGENTS.md` itself, so it is not duplicated.
pub fn inline_project_context(project_root: &Path, provider: &str) -> String {
    let mut parts = Vec::new();
    let mut budget = MAX_CONTEXT_CHARS;
    for name in CONTEXT_FILES {
        if provider == "Codex" && name == "AGENTS.md" {
            continue;
        }
        if budget == 0 {
            break;
        }
        let path = project_root.join(name);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.file_type().is_file() {
            continue;
        }
        let Some((source, source_truncated)) = read_prefix(&path, MAX_FILE_READ_BYTES) else {
            continue;
        };
        let text = source.trim();
        if text.is_empty() && !source_truncated {
            continue;
        }
        let limit = MAX_FILE_CHARS.min(budget);
        let clipped = clipped(text, limit, source_truncated);
        budget = budget.saturating_sub(text.chars().count().min(limit));
        parts.push(format!(
            "<archivo-de-contexto ruta=\"{name}\">\n{clipped}\n</archivo-de-contexto>"
        ));
    }
    if parts.is_empty() {
        return "\n\nEste proyecto aún no tiene archivos de contexto (AGENTS.md ni .ai/): no los busques ni ejecutes comandos para encontrarlos; empieza directamente con la solicitud.".to_owned();
    }
    format!(
        "\n\nStudio ya leyó por ti los archivos de contexto de arriba y los incluye aquí. No ejecutes comandos para leerlos de nuevo; el chat no puede pedirte permiso para eso.\n{}",
        parts.join("\n")
    )
}

/// Context prepared by Rust for a fresh Studio chat, after context compaction has run.
pub fn initial_context(project_root: &Path, provider: &str) -> String {
    let recovered = crate::features::explicit_memory::prepare_new_chat(project_root);
    let mut context = new_chat_prompt(project_root);
    context.push_str(&inline_project_context(project_root, provider));
    context.push_str(&recovered);
    context.push_str(&crate::features::chat_journal::recent_sessions_prompt(
        project_root,
    ));
    context
}

fn read_prefix(path: &Path, max_bytes: usize) -> Option<(String, bool)> {
    let mut file = fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    let capacity = usize::try_from(length).unwrap_or(max_bytes).min(max_bytes);
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref()
        .take(max_bytes as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    let truncated = length > bytes.len() as u64;
    Some((String::from_utf8_lossy(&bytes).into_owned(), truncated))
}

fn clipped(text: &str, limit: usize, source_truncated: bool) -> String {
    let mut result = text.chars().take(limit).collect::<String>();
    if text.chars().count() > limit {
        result.push_str(&format!(
            "\n[…recortado a {limit} caracteres; lee el archivo si necesitas el resto]"
        ));
    } else if source_truncated {
        result.push_str("\n[…contenido restante omitido por el límite de lectura]");
    }
    result
}

/// Build the first request sent to a fresh provider session.
///
/// `recovered` and `recap` are supplied by the context recovery layer. A handoff takes precedence
/// over a transcript recap, matching the existing adapter behavior.
pub fn first_message(
    project_root: &Path,
    provider: &str,
    request: &str,
    recovered: &str,
    handoff: &str,
    recap: &str,
) -> String {
    let mut context = initial_context(project_root, provider);
    context.push_str(recovered);
    if !handoff.is_empty() {
        context.push_str(&handoff_block(handoff));
    } else {
        context.push_str(recap);
    }
    format!("{context}\n\n{CHAT_PANEL_REQUEST_MARKER}\n{request}")
}

pub fn handoff_block(text: &str) -> String {
    format!(
        "\n\nEsta conversación continúa de una sesión anterior cuyo contexto se liberó automáticamente para ahorrar consumo. Retómala desde el traspaso de abajo como si nada hubiera cambiado: no menciones el traspaso ni que el contexto se liberó, no pidas confirmación ni repitas lo ya hecho. Si la persona pregunta por algo que está en el traspaso, responde con naturalidad, como si lo recordaras de la conversación.\n<archivo-de-contexto ruta=\"traspaso de la conversación\">\n{text}\n</archivo-de-contexto>"
    )
}

/// Existing regular context files for callers that must pass files to a provider by name.
pub fn context_paths(project_root: &Path) -> Vec<PathBuf> {
    CONTEXT_FILES
        .iter()
        .filter_map(|name| {
            let path = project_root.join(name);
            fs::symlink_metadata(&path)
                .ok()
                .filter(|metadata| metadata.file_type().is_file())
                .map(|_| path)
        })
        .collect()
}

#[cfg(test)]
#[path = "chat_context_tests.rs"]
mod tests;
