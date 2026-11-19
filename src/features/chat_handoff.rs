//! Persisted context handoffs for Studio chats.
//!
//! Rust owns the threshold, storage, redaction, and consumption lifecycle. The provider adapter
//! only produces the text of a handoff when Rust asks it to run a hidden read-only turn.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const THRESHOLD: f64 = 0.90;
pub const MAX_HANDOFF_CHARS: usize = 24_000;
const MAX_HANDOFF_BYTES: u64 = (MAX_HANDOFF_CHARS * 4) as u64;
const DIRECTORY: &str = ".ai/chat-handoffs";
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub const PROMPT: &str = "Vamos a liberar tu ventana de contexto: esta conversación seguirá en una sesión nueva que solo conocerá lo que escribas ahora. No uses herramientas ni modifiques archivos. Redacta un traspaso completo y fiel, en Markdown, con estas secciones:\n\n## Objetivo\nQué quiere lograr la persona y en qué contexto.\n\n## Decisiones y preferencias\nDecisiones tomadas, estilo y restricciones, lo que pidió o rechazó (con sus palabras cuando importen).\n\n## Lo hecho\nArchivos creados o modificados (ruta y qué cambió), comandos relevantes con su resultado y pruebas ejecutadas.\n\n## Estado actual\nQué funciona, qué falla y las hipótesis vigentes.\n\n## Pendiente\nLo que falta y el siguiente paso.\n\n## Datos que no hay que volver a buscar\nRutas, nombres, identificadores y mensajes de error exactos.\n\nSi al empezar esta conversación recibiste un traspaso anterior, incorpora lo que siga vigente: un traspaso sobre otro no debe perder nada. Máximo 1500 palabras. No incluyas secretos ni tokens. Responde solo con el traspaso.";

/// Whether the provider reported a usable context window at or above 90%.
pub fn needs_release(tokens: Option<i64>, window: Option<i64>) -> bool {
    let (Some(tokens), Some(window)) = (tokens, window) else {
        return false;
    };
    tokens > 0 && window > 0 && tokens as f64 / window as f64 >= THRESHOLD
}

/// Reads a handoff written by this version or the previous Python adapter.
pub fn load(project: &Path, provider: &str, session_id: &str) -> Option<String> {
    let path = existing_handoff_path(project, provider, session_id).ok()??;
    let metadata = fs::symlink_metadata(&path).ok()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_HANDOFF_BYTES {
        return None;
    }
    let mut file = fs::File::open(path).ok()?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(MAX_HANDOFF_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_HANDOFF_BYTES {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    let safe = redact_and_clip(text.trim());
    let safe = safe.trim();
    (!safe.is_empty()).then(|| safe.to_owned())
}

/// Redacts secret-like lines and atomically creates the handoff file.
pub fn save(project: &Path, provider: &str, session_id: &str, text: &str) -> Result<(), String> {
    let path = handoff_path(project, provider, session_id)?;
    let content = redact_and_clip(text);
    if content.trim().is_empty() {
        return Err("El agente no produjo un traspaso".to_owned());
    }
    match fs::symlink_metadata(&path) {
        Ok(_) => return Err("Ya existe un traspaso pendiente para esta sesión".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let parent = path
        .parent()
        .ok_or("La ruta del traspaso no tiene carpeta")?;
    let temp = loop {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".handoff-{}-{sequence}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("No se pudo crear el traspaso temporal: {error}")),
        }
    };
    let (temporary, mut file) = temp;
    let result = (|| {
        file.write_all(content.as_bytes())
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        fs::rename(&temporary, &path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Removes the persisted handoff after a successful fresh session accepted it.
pub fn consume(project: &Path, provider: &str, session_id: &str) -> Result<(), String> {
    let Some(path) = existing_handoff_path(project, provider, session_id)? else {
        return Ok(());
    };
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            fs::remove_file(path).map_err(|error| error.to_string())
        }
        Ok(_) => Err("La ruta del traspaso no es un archivo regular".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn validate_identity(provider: &str, session_id: &str) -> Result<(), String> {
    if !matches!(provider, "Codex" | "Claude" | "OpenCode") {
        return Err("Proveedor de chat no soportado".to_owned());
    }
    if session_id.is_empty()
        || session_id.len() > 160
        || !session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err("ID de sesión no válido para un traspaso".to_owned());
    }
    Ok(())
}

fn project_root(project: &Path) -> Result<PathBuf, String> {
    project
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver el proyecto: {error}"))
}

fn handoff_path(project: &Path, provider: &str, session_id: &str) -> Result<PathBuf, String> {
    validate_identity(provider, session_id)?;
    let root = project_root(project)?;
    let ai = root.join(".ai");
    ensure_directory(&ai)?;
    let directory = root.join(DIRECTORY);
    ensure_directory(&directory)?;
    Ok(directory.join(format!("{}-{session_id}.md", provider.to_lowercase())))
}

fn existing_handoff_path(
    project: &Path,
    provider: &str,
    session_id: &str,
) -> Result<Option<PathBuf>, String> {
    validate_identity(provider, session_id)?;
    let root = project_root(project)?;
    let ai = root.join(".ai");
    if !existing_directory(&ai)? {
        return Ok(None);
    }
    let directory = root.join(DIRECTORY);
    if !existing_directory(&directory)? {
        return Ok(None);
    }
    Ok(Some(directory.join(format!(
        "{}-{session_id}.md",
        provider.to_lowercase()
    ))))
}

fn existing_directory(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(true),
        Ok(_) => Err(format!(
            "La carpeta de traspasos no es un directorio seguro: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(format!(
            "La carpeta de traspasos no es un directorio seguro: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|error| error.to_string())?;
            let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
            if metadata.file_type().is_dir() {
                Ok(())
            } else {
                Err(format!(
                    "La ruta creada no es un directorio: {}",
                    path.display()
                ))
            }
        }
        Err(error) => Err(error.to_string()),
    }
}

fn redact_and_clip(text: &str) -> String {
    let mut result = String::new();
    let mut first = true;
    for line in text.lines() {
        if !first {
            result.push('\n');
        }
        first = false;
        let safe = if crate::features::chat_state::contains_secret(line) {
            "(línea omitida: posible secreto)"
        } else {
            line
        };
        let remaining = MAX_HANDOFF_CHARS
            .saturating_sub(1)
            .saturating_sub(result.chars().count());
        if remaining == 0 {
            break;
        }
        result.extend(safe.chars().take(remaining));
        if result.chars().count() >= MAX_HANDOFF_CHARS.saturating_sub(1) {
            break;
        }
    }
    result.push('\n');
    result
}

#[cfg(test)]
#[path = "chat_handoff_tests.rs"]
mod tests;
