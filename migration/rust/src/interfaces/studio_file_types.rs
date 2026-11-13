//! Maps file names to the icon, tint and language label shown in the IDE.
use super::design;
use iced::Color;
use std::path::Path;

pub(super) struct FileType {
    pub icon: &'static str,
    pub tint: Color,
    pub language: &'static str,
}

pub(super) fn of(path: &Path) -> FileType {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (icon, tint, language) = match extension.as_str() {
        "rs" => ("code", design::TINT_ORANGE, "Rust"),
        "py" => ("code", design::TINT_YELLOW, "Python"),
        "c" | "h" => ("code", design::BRAND_BLUE, "C"),
        "cc" | "cpp" | "cxx" | "hpp" => ("code", design::BRAND_BLUE, "C++"),
        "js" | "jsx" | "mjs" => ("code", design::TINT_YELLOW, "JavaScript"),
        "ts" | "tsx" => ("code", design::BRAND_BLUE, "TypeScript"),
        "go" => ("code", design::BRAND_CYAN, "Go"),
        "cs" => ("code", design::TINT_PURPLE, "C#"),
        "sh" | "bash" | "ps1" | "bat" | "cmd" => ("terminal", design::TINT_GREEN, "Shell"),
        "toml" => ("braces", design::TINT_ORANGE, "TOML"),
        "json" => ("braces", design::TINT_YELLOW, "JSON"),
        "yml" | "yaml" => ("braces", design::TINT_PURPLE, "YAML"),
        "html" | "xml" | "svg" | "qml" | "ui" => ("code", design::TINT_ORANGE, "Marcado"),
        "css" | "scss" | "qss" => ("braces", design::BRAND_BLUE, "Estilos"),
        "diff" | "patch" => ("git-commit", design::TINT_PURPLE, "Diff"),
        "md" | "markdown" => ("file-text", design::BRAND_CYAN, "Markdown"),
        "txt" | "log" => ("file-text", design::MUTED, "Texto"),
        "lock" => ("file", design::FAINT, "Lockfile"),
        _ => ("file", design::MUTED, "Texto plano"),
    };
    FileType {
        icon,
        tint,
        language,
    }
}
