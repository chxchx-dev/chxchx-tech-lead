//! Bounded project search by file name or content. Read-only; never follows links.
use crate::features::project_detection::IGNORED_DIRECTORIES;
use std::fs;
use std::io::Read;
use std::path::Path;

const MAX_FILES: usize = 20_000;
const MAX_HITS: usize = 200;
const MAX_DEPTH: usize = 24;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_PREVIEW: usize = 160;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    /// Path relative to the project root, with `/` separators.
    pub relative: String,
    /// 1-based line of a content match; `None` for a file-name match.
    pub line: Option<usize>,
    /// 0-based character column and length of the match inside the line.
    pub column: usize,
    pub length: usize,
    pub preview: String,
}

#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub files_scanned: usize,
    /// True when a limit stopped the search before the whole project was read.
    pub truncated: bool,
}

pub fn search(root: &Path, query: &str, in_content: bool) -> Result<SearchResults, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(SearchResults::default());
    }
    if query.len() > 256 {
        return Err("La búsqueda es demasiado larga".to_owned());
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("No se pudo resolver el proyecto: {error}"))?;
    let needle = query.to_lowercase();
    let mut results = SearchResults::default();
    walk(&root, &root, 0, &needle, in_content, &mut results);
    results.hits.sort_by(|left, right| {
        left.relative
            .cmp(&right.relative)
            .then(left.line.cmp(&right.line))
    });
    Ok(results)
}

fn walk(
    root: &Path,
    directory: &Path,
    depth: usize,
    needle: &str,
    in_content: bool,
    results: &mut SearchResults,
) {
    if depth > MAX_DEPTH || results.truncated {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if results.files_scanned >= MAX_FILES || results.hits.len() >= MAX_HITS {
            results.truncated = true;
            return;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        let path = entry.path();
        if kind.is_dir() {
            if !IGNORED_DIRECTORIES.contains(&name.as_str()) {
                walk(root, &path, depth + 1, needle, in_content, results);
            }
            continue;
        }
        if !kind.is_file() || name.starts_with(".env") {
            continue;
        }
        results.files_scanned += 1;
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if !in_content {
            if let Some(column) = relative.to_lowercase().find(needle) {
                results.hits.push(SearchHit {
                    column: relative[..column].chars().count(),
                    length: needle.chars().count(),
                    preview: relative.clone(),
                    relative,
                    line: None,
                });
            }
            continue;
        }
        search_content(&path, relative, needle, results);
    }
}

fn search_content(path: &Path, relative: String, needle: &str, results: &mut SearchResults) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.len() > MAX_FILE_BYTES {
        return;
    }
    let mut bytes = Vec::new();
    let Ok(file) = fs::File::open(path) else {
        return;
    };
    if file.take(MAX_FILE_BYTES).read_to_end(&mut bytes).is_err()
        || bytes[..bytes.len().min(8192)].contains(&0)
    {
        return;
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return;
    };
    for (index, line) in text.lines().enumerate() {
        if results.hits.len() >= MAX_HITS {
            results.truncated = true;
            return;
        }
        let lowered = line.to_lowercase();
        if let Some(position) = lowered.find(needle) {
            // Lowercasing can change byte offsets for some scripts; fall back to column 0.
            let column = if line.is_char_boundary(position) {
                line[..position].chars().count()
            } else {
                0
            };
            results.hits.push(SearchHit {
                relative: relative.clone(),
                line: Some(index + 1),
                column,
                length: needle.chars().count(),
                preview: line.trim().chars().take(MAX_PREVIEW).collect(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("chxchx-search-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        fs::write(
            dir.join("src/Main.rs"),
            "fn main() {\n    let Token = 1;\n}\n",
        )
        .unwrap();
        fs::write(dir.join("README.md"), "Usa el token con cuidado\n").unwrap();
        fs::write(dir.join(".env"), "TOKEN=secreto\n").unwrap();
        fs::write(dir.join("node_modules/pkg/index.js"), "token\n").unwrap();
        fs::write(dir.join("binary.bin"), [b't', 0, b'o', b'k', b'e', b'n']).unwrap();
        dir
    }

    #[test]
    fn finds_names_and_content_without_leaking_ignored_files() {
        let dir = project("basic");
        let names = search(&dir, "main", false).unwrap();
        assert_eq!(names.hits.len(), 1);
        assert_eq!(names.hits[0].relative, "src/Main.rs");
        assert_eq!(names.hits[0].line, None);

        let content = search(&dir, "TOKEN", true).unwrap();
        let found: Vec<_> = content
            .hits
            .iter()
            .map(|hit| (hit.relative.as_str(), hit.line))
            .collect();
        assert_eq!(found, [("README.md", Some(1)), ("src/Main.rs", Some(2))]);
        assert_eq!(content.hits[1].column, 8, "column of the match in the line");
        assert!(!content.truncated);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_and_oversized_queries_are_handled() {
        let dir = project("limits");
        assert!(search(&dir, "   ", true).unwrap().hits.is_empty());
        assert!(search(&dir, &"x".repeat(300), true).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn results_are_capped_and_flagged_as_truncated() {
        let dir = project("cap");
        let many = "coincidencia\n".repeat(MAX_HITS + 50);
        fs::write(dir.join("many.txt"), many).unwrap();
        let results = search(&dir, "coincidencia", true).unwrap();
        assert_eq!(results.hits.len(), MAX_HITS);
        assert!(results.truncated);
        let _ = fs::remove_dir_all(&dir);
    }
}
