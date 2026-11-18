//! Keep the project status files short by archiving older sections before a fresh chat.
//!
//! The rules match the former Python adapter: keep a contiguous window of recent sections,
//! preserve canonical and managed sections, and prepend archived sections newest first.

use crate::features::project_backup::backup_project;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_KEEP_SECTIONS: usize = 6;
pub const DEFAULT_BUDGET_CHARS: usize = 12_000;
const COMPACTED_FILES: [&str; 2] = ["CURRENT_STATE.md", "HANDOFF.md"];
const ARCHIVE_INTRO: &str =
    "Secciones archivadas automáticamente, de la más reciente a la más antigua.\n\n";
const PINNED_HEADINGS: [&str; 6] = [
    "estado actual",
    "en progreso",
    "bloqueos",
    "último cambio",
    "pendiente",
    "validación",
];
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompactionResult {
    pub files: Vec<CompactedFile>,
    pub backup: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactedFile {
    pub name: String,
    pub moved: Vec<String>,
    pub chars_before: usize,
    pub chars_after: usize,
}

impl CompactionResult {
    pub fn moved_count(&self) -> usize {
        self.files.iter().map(|file| file.moved.len()).sum()
    }

    pub fn changed(&self) -> bool {
        self.moved_count() > 0
    }
}

/// Projects opt out with `auto_compact_context = false` in `.ai/chxchx-tech.toml`.
/// Missing, malformed, or unreadable settings keep the default enabled behavior.
pub fn auto_compact_enabled(project_root: &Path) -> bool {
    let config = project_root.join(".ai/chxchx-tech.toml");
    let Ok(metadata) = fs::symlink_metadata(&config) else {
        return true;
    };
    if !metadata.file_type().is_file() {
        return true;
    }
    let Ok(contents) = fs::read_to_string(config) else {
        return true;
    };
    let Ok(parsed) = contents.parse::<toml::Table>() else {
        return true;
    };
    !matches!(
        parsed.get("auto_compact_context"),
        Some(toml::Value::Boolean(false))
    )
}

/// Compact `CURRENT_STATE.md` and `HANDOFF.md`, backing up the managed project files first.
pub fn compact_context(project_root: &Path) -> Result<CompactionResult, String> {
    compact_context_with_limits(project_root, DEFAULT_KEEP_SECTIONS, DEFAULT_BUDGET_CHARS)
}

fn compact_context_with_limits(
    project_root: &Path,
    keep: usize,
    budget: usize,
) -> Result<CompactionResult, String> {
    compact_context_with_backup(project_root, keep, budget, backup_project)
}

fn compact_context_with_backup(
    project_root: &Path,
    keep: usize,
    budget: usize,
    make_backup: impl FnOnce(&Path) -> Result<Option<PathBuf>, String>,
) -> Result<CompactionResult, String> {
    if !auto_compact_enabled(project_root) {
        return Ok(CompactionResult::default());
    }
    let root = project_root
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver la raíz del proyecto: {error}"))?;
    let ai = root.join(".ai");
    match fs::symlink_metadata(&ai) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CompactionResult::default());
        }
        Err(error) => return Err(format!("No se pudo inspeccionar .ai: {error}")),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(".ai es un enlace simbólico; no se compactó el contexto".to_owned());
        }
        Ok(metadata) if !metadata.is_dir() => return Ok(CompactionResult::default()),
        Ok(_) => {}
    }

    let mut plans = Vec::new();
    for name in COMPACTED_FILES {
        let path = ai.join(name);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "No se pudo inspeccionar {}: {error}",
                    path.display()
                ));
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let original = fs::read_to_string(&path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
        let (compacted, moved) = compact_text(&original, name, keep.max(1), budget);
        let file = CompactedFile {
            name: name.to_owned(),
            moved: moved
                .iter()
                .filter_map(|section| section.lines().next())
                .map(|heading| {
                    heading
                        .strip_prefix("## ")
                        .unwrap_or(heading)
                        .trim()
                        .to_owned()
                })
                .collect(),
            chars_before: original.chars().count(),
            chars_after: compacted.chars().count(),
        };
        plans.push(Plan {
            file,
            path,
            compacted,
            moved_text: moved.concat(),
        });
    }

    let mut result = CompactionResult {
        files: plans.iter().map(|plan| plan.file.clone()).collect(),
        backup: None,
    };
    if !result.changed() {
        return Ok(result);
    }

    let history = ai.join("history");
    ensure_safe_archive_directory(&history)?;
    let mut archive_plans = Vec::new();
    for plan in plans.iter().filter(|plan| !plan.file.moved.is_empty()) {
        let archive = history.join(&plan.file.name);
        let existing = read_archive(&archive)?;
        let contents = prepend_archive(
            existing.as_deref().unwrap_or_default(),
            &plan.file.name,
            &plan.moved_text,
        );
        archive_plans.push((archive, existing, contents, plan));
    }

    result.backup = make_backup(&root)?;
    fs::create_dir_all(&history)
        .map_err(|error| format!("No se pudo crear {}: {error}", history.display()))?;
    for (archive, previous_archive, archive_contents, plan) in archive_plans {
        atomic_write(&archive, &archive_contents)?;
        if let Err(error) = atomic_write(&plan.path, &plan.compacted) {
            let rollback = match previous_archive {
                Some(previous) => atomic_write(&archive, &previous),
                None => fs::remove_file(&archive).map_err(|error| error.to_string()),
            };
            if let Err(rollback_error) = rollback {
                return Err(format!(
                    "{error}; tampoco se pudo revertir el archivo histórico: {rollback_error}"
                ));
            }
            return Err(error);
        }
    }
    Ok(result)
}

struct Plan {
    file: CompactedFile,
    path: PathBuf,
    compacted: String,
    moved_text: String,
}

fn compact_text(text: &str, name: &str, keep: usize, budget: usize) -> (String, Vec<String>) {
    let normalized = text.replace("\r\n", "\n");
    let (mut head, sections) = split_sections(&normalized);
    let mut size = head.chars().count();
    let mut kept = Vec::new();
    let mut moved = Vec::new();
    let mut recent = 0;
    for section in sections {
        if is_pinned(&section) {
            size += section.chars().count();
            kept.push(section);
            continue;
        }
        let section_size = section.chars().count();
        let fits = recent < keep && size + section_size <= budget;
        if moved.is_empty() && (recent == 0 || fits) {
            size += section_size;
            kept.push(section);
            recent += 1;
        } else {
            moved.push(with_trailing_blank_line(&section));
        }
    }
    if moved.is_empty() {
        return (text.to_owned(), moved);
    }
    let pointer =
        format!("> Historial anterior: `.ai/history/{name}` (no se lee al iniciar un chat).\n");
    if !head.contains(&pointer) {
        head = with_trailing_blank_line(&format!("{}\n\n{pointer}", head.trim_end_matches('\n')));
    }
    let compacted = format!("{}{}", head, kept.concat())
        .trim_end_matches('\n')
        .to_owned()
        + "\n";
    (compacted, moved)
}

fn split_sections(text: &str) -> (String, Vec<String>) {
    let mut head = String::new();
    let mut sections: Vec<String> = Vec::new();
    let mut in_fence = false;
    let mut in_managed = false;
    for line in text.split_inclusive('\n') {
        let stripped = line.trim();
        let starts_section = !in_fence
            && !in_managed
            && (line.starts_with("## ") || stripped.starts_with("<!-- chxchx-tech:start"));
        if starts_section {
            sections.push(line.to_owned());
        } else if let Some(section) = sections.last_mut() {
            section.push_str(line);
        } else {
            head.push_str(line);
        }
        if stripped.starts_with("```") {
            in_fence = !in_fence;
        } else if stripped.starts_with("<!-- chxchx-tech:start") {
            in_managed = true;
        } else if stripped.starts_with("<!-- chxchx-tech:end") {
            in_managed = false;
        }
    }
    (head, sections)
}

fn is_pinned(section: &str) -> bool {
    let first = section.lines().next().unwrap_or_default();
    let heading = first
        .strip_prefix("## ")
        .unwrap_or(first)
        .trim()
        .to_lowercase();
    PINNED_HEADINGS.contains(&heading.as_str()) || section.contains("<!-- chxchx-tech:start")
}

fn with_trailing_blank_line(text: &str) -> String {
    format!("{}\n\n", text.trim_end_matches('\n'))
}

fn prepend_archive(existing: &str, name: &str, moved_text: &str) -> String {
    let title = format!(
        "# Historial de {}\n\n",
        name.strip_suffix(".md").unwrap_or(name)
    );
    let body = existing
        .strip_prefix(&title)
        .unwrap_or(existing)
        .strip_prefix(ARCHIVE_INTRO)
        .unwrap_or_else(|| existing.strip_prefix(&title).unwrap_or(existing));
    format!("{title}{ARCHIVE_INTRO}{moved_text}{body}")
        .trim_end_matches('\n')
        .to_owned()
        + "\n"
}

fn ensure_safe_archive_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(".ai/history es un enlace simbólico; no se compactó el contexto".to_owned())
        }
        Ok(metadata) if !metadata.is_dir() => {
            Err(".ai/history no es un directorio; no se compactó el contexto".to_owned())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "No se pudo inspeccionar {}: {error}",
            path.display()
        )),
    }
}

fn read_archive(path: &Path) -> Result<Option<String>, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(format!(
            "{} no es un archivo regular; no se compactó",
            path.display()
        )),
        Ok(_) => fs::read_to_string(path)
            .map(Some)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "No se pudo inspeccionar {}: {error}",
            path.display()
        )),
    }
}

fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "No se pudo resolver la carpeta del archivo".to_owned())?;
    let mut temporary = None;
    for _ in 0..10 {
        let counter = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".chxchx-context-{}-{counter}.tmp",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                if let Err(error) = write_sync(&mut file, contents) {
                    let _ = fs::remove_file(&candidate);
                    return Err(format!("No se pudo escribir {}: {error}", path.display()));
                }
                temporary = Some(candidate);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("No se pudo crear un temporal: {error}")),
        }
    }
    let temporary =
        temporary.ok_or_else(|| "No se pudo reservar un archivo temporal".to_owned())?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(temporary);
        return Err(format!("No se pudo reemplazar {}: {error}", path.display()));
    }
    Ok(())
}

fn write_sync(file: &mut File, contents: &str) -> std::io::Result<()> {
    file.write_all(contents.as_bytes())?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "chxchx-context-compact-{}-{id}",
                std::process::id()
            ));
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

    #[test]
    fn keeps_recent_and_pinned_sections_and_archives_the_older_window() {
        let text = "# Estado\n\n## Cambio 4\n\n- cuatro\n\n## Cambio 3\n\n- tres\n\n## Cambio 2\n\n- dos\n\n## Estado actual\n\n- Canónico.\n";
        let (compacted, moved) = compact_text(text, "CURRENT_STATE.md", 2, 12_000);

        assert!(compacted.contains("## Cambio 4"));
        assert!(compacted.contains("## Cambio 3"));
        assert!(compacted.contains("## Estado actual"));
        assert!(compacted.contains("`.ai/history/CURRENT_STATE.md`"));
        assert_eq!(moved.join(""), "## Cambio 2\n\n- dos\n\n");
    }

    #[test]
    fn section_markers_inside_fences_and_managed_blocks_are_not_split() {
        let text = "```md\n## Código\n```\n<!-- chxchx-tech:start demo -->\n## Administrado\n<!-- chxchx-tech:end demo -->\n## Cambio viejo\ntexto\n";
        let (compacted, moved) = compact_text(text, "HANDOFF.md", 1, 12_000);

        assert!(compacted.contains("## Código"));
        assert!(compacted.contains("## Administrado"));
        assert_eq!(moved.len(), 0);
    }

    #[test]
    fn config_can_opt_out_and_no_context_is_a_noop() {
        let project = Scratch::new();
        fs::create_dir_all(project.0.join(".ai")).unwrap();
        fs::write(
            project.0.join(".ai/chxchx-tech.toml"),
            "auto_compact_context = false\n",
        )
        .unwrap();

        assert!(!auto_compact_enabled(&project.0));
        assert_eq!(
            compact_context(&project.0).unwrap(),
            CompactionResult::default()
        );
    }

    #[test]
    fn archives_are_prepended_without_duplicate_headers() {
        let old = "# Historial de CURRENT_STATE\n\nSecciones archivadas automáticamente, de la más reciente a la más antigua.\n\n## Viejo\n\n- detalle\n";
        let new = prepend_archive(old, "CURRENT_STATE.md", "## Nuevo\n\n- actual\n\n");

        assert_eq!(new.matches("# Historial de CURRENT_STATE").count(), 1);
        assert!(new.find("## Nuevo").unwrap() < new.find("## Viejo").unwrap());
    }

    #[test]
    fn compaction_writes_archives_and_runs_the_backup_before_changes() {
        let project = Scratch::new();
        let state = project.0.join(".ai/CURRENT_STATE.md");
        fs::create_dir_all(state.parent().unwrap()).unwrap();
        fs::write(
            &state,
            "# Estado\n\n## Cambio 4\n\n- cuatro\n\n## Cambio 3\n\n- tres\n\n## Cambio 2\n\n- dos\n\n## Cambio 1\n\n- uno\n",
        )
        .unwrap();
        let mut backup_called = false;

        let result = compact_context_with_backup(&project.0, 2, 12_000, |root| {
            backup_called = true;
            assert_eq!(root, project.0.canonicalize().unwrap());
            Ok(None)
        })
        .unwrap();

        assert!(backup_called);
        assert_eq!(result.moved_count(), 2);
        let current = fs::read_to_string(&state).unwrap();
        assert!(current.contains("## Cambio 4") && current.contains("## Cambio 3"));
        assert!(!current.contains("## Cambio 2"));
        let archive = fs::read_to_string(project.0.join(".ai/history/CURRENT_STATE.md")).unwrap();
        assert!(archive.find("## Cambio 2").unwrap() < archive.find("## Cambio 1").unwrap());

        let unchanged = compact_context_with_backup(&project.0, 2, 12_000, |_| {
            panic!("an unchanged context must not create another backup")
        })
        .unwrap();
        assert!(!unchanged.changed());
    }
}
