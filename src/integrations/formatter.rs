//! Code formatting through the project's own tools (Prettier, rustfmt, ruff, gofmt…), never a
//! built-in guess: the file goes in through stdin and the formatted text comes back. Nothing is
//! written here; the caller decides what to do with the result.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(15);
const MAX_OUTPUT: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formatter {
    pub name: &'static str,
    program: PathBuf,
    args: Vec<String>,
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| {
            let plain = dir.join(name);
            [
                plain.clone(),
                plain.with_extension("exe"),
                plain.with_extension("cmd"),
            ]
        })
        .find(|candidate| candidate.is_file())
}

/// Prettier of the project (`node_modules/.bin`) when it has one, else the one on the PATH.
fn prettier(root: &Path) -> Option<PathBuf> {
    ["prettier", "prettier.cmd"]
        .iter()
        .map(|name| root.join("node_modules/.bin").join(name))
        .find(|candidate| candidate.is_file())
        .or_else(|| find_in_path("prettier"))
}

/// The tool that formats `relative`, when it is installed.
pub fn formatter_for(root: &Path, relative: &Path) -> Option<Formatter> {
    let extension = relative
        .extension()
        .and_then(|e| e.to_str())?
        .to_ascii_lowercase();
    let file = relative.to_string_lossy().replace('\\', "/");
    match extension.as_str() {
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "json" | "jsonc" | "css" | "scss"
        | "less" | "html" | "vue" | "md" | "markdown" | "yml" | "yaml" | "graphql" => {
            Some(Formatter {
                name: "Prettier",
                program: prettier(root)?,
                args: vec!["--stdin-filepath".into(), file],
            })
        }
        "rs" => Some(Formatter {
            name: "rustfmt",
            program: find_in_path("rustfmt")?,
            args: vec!["--edition".into(), "2024".into()],
        }),
        "py" => Some(Formatter {
            name: "ruff",
            program: find_in_path("ruff")?,
            args: vec!["format".into(), "--stdin-filename".into(), file, "-".into()],
        }),
        "go" => Some(Formatter {
            name: "gofmt",
            program: find_in_path("gofmt")?,
            args: Vec::new(),
        }),
        _ => None,
    }
}

/// Formats `text` as `relative` with `formatter`, running in the project folder so the tool finds
/// its own configuration (`.prettierrc`, `rustfmt.toml`…).
pub fn format(root: &Path, formatter: &Formatter, text: &str) -> Result<String, String> {
    let mut child = Command::new(&formatter.program)
        .args(&formatter.args)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("No se pudo iniciar {}: {error}", formatter.name))?;
    let mut stdin = child.stdin.take().ok_or("sin entrada")?;
    let input = text.to_owned();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });
    let mut out = child.stdout.take().ok_or("sin salida")?;
    let mut err = child.stderr.take().ok_or("sin errores")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = (&mut out)
            .take(MAX_OUTPUT as u64 + 1)
            .read_to_end(&mut bytes);
        bytes
    });
    let errors = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = (&mut err).take(16 * 1024).read_to_end(&mut bytes);
        bytes
    });
    let deadline = Instant::now() + LIMIT;
    let status = loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{} tardó demasiado", formatter.name));
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let _ = writer.join();
    let stdout = reader.join().unwrap_or_default();
    let stderr = String::from_utf8_lossy(&errors.join().unwrap_or_default()).into_owned();
    if !status.success() {
        let reason = stderr
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("error");
        return Err(format!("{}: {}", formatter.name, reason.trim()));
    }
    if stdout.len() > MAX_OUTPUT {
        return Err("El resultado es demasiado grande".into());
    }
    String::from_utf8(stdout).map_err(|_| "El formateador no devolvió texto válido".into())
}

/// Whether the project asks to format every save (`"editor.formatOnSave": true` in
/// `.vscode/settings.json`). Without it, formatting stays something the person asks for.
pub fn format_on_save(root: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join(".vscode/settings.json")) else {
        return false;
    };
    mandatory(&text)
}

fn mandatory(settings: &str) -> bool {
    let Some(at) = settings.find("\"editor.formatOnSave\"") else {
        return false;
    };
    let rest = settings[at + "\"editor.formatOnSave\"".len()..].trim_start();
    rest.strip_prefix(':')
        .is_some_and(|value| value.trim_start().starts_with("true"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_explicit_true_makes_formatting_mandatory() {
        assert!(mandatory("{\n  \"editor.formatOnSave\": true,\n}"));
        assert!(mandatory("{\"editor.formatOnSave\" :true}"));
        assert!(!mandatory("{\"editor.formatOnSave\": false}"));
        assert!(!mandatory("{\"editor.tabSize\": 2}"));
    }

    #[test]
    fn files_without_a_known_tool_have_no_formatter() {
        let root = std::env::temp_dir();
        assert!(formatter_for(&root, Path::new("notas.txt")).is_none());
        assert!(formatter_for(&root, Path::new("Makefile")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn formats_through_stdin_and_reports_failures() {
        let root = std::env::temp_dir();
        let cat = Formatter {
            name: "cat",
            program: find_in_path("cat").unwrap(),
            args: Vec::new(),
        };
        assert_eq!(format(&root, &cat, "a  b\n").unwrap(), "a  b\n");
        let fail = Formatter {
            name: "false",
            program: find_in_path("false").unwrap(),
            args: Vec::new(),
        };
        assert!(format(&root, &fail, "x").is_err());
    }
}
