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
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // Files known by their whole name rather than an extension.
    let (icon, tint, language) = match name.as_str() {
        "dockerfile" | "containerfile" => ("code", design::BRAND_BLUE, "Dockerfile"),
        "makefile" | "gnumakefile" | "justfile" => ("terminal", design::TINT_GREEN, "Makefile"),
        "cmakelists.txt" => ("braces", design::TINT_GREEN, "CMake"),
        ".gitignore" | ".gitattributes" | ".dockerignore" => ("git-commit", design::MUTED, "Git"),
        ".env" | ".env.example" | ".editorconfig" => ("braces", design::MUTED, "Configuración"),
        "license" | "licence" | "copying" => ("file-text", design::MUTED, "Licencia"),
        _ => ("", design::MUTED, ""),
    };
    if !language.is_empty() {
        return FileType {
            icon,
            tint,
            language,
        };
    }
    let (icon, tint, language) = match extension.as_str() {
        "java" => ("code", design::TINT_ORANGE, "Java"),
        "kt" | "kts" => ("code", design::TINT_PURPLE, "Kotlin"),
        "swift" => ("code", design::TINT_ORANGE, "Swift"),
        "rb" => ("code", design::TINT_ORANGE, "Ruby"),
        "php" => ("code", design::TINT_PURPLE, "PHP"),
        "sql" => ("braces", design::BRAND_BLUE, "SQL"),
        "vue" | "svelte" => ("code", design::TINT_GREEN, "Componente web"),
        "lua" => ("code", design::BRAND_BLUE, "Lua"),
        "cmake" => ("braces", design::TINT_GREEN, "CMake"),
        "ini" | "cfg" | "conf" => ("braces", design::MUTED, "Configuración"),
        "mdx" => ("file-text", design::BRAND_CYAN, "MDX"),
        "csv" | "tsv" => ("file-text", design::TINT_GREEN, "Datos"),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" => {
            ("file", design::TINT_PURPLE, "Imagen")
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn language(path: &str) -> &'static str {
        of(Path::new(path)).language
    }

    #[test]
    fn the_type_comes_from_the_extension_in_any_case() {
        assert_eq!(language("src/main.rs"), "Rust");
        assert_eq!(language("LEEME.MD"), "Markdown");
        assert_eq!(language("a/b/c.tsx"), "TypeScript");
        assert_eq!(language("datos.csv"), "Datos");
        assert_eq!(language("sin-extension"), "Texto plano");
    }

    #[test]
    fn some_files_are_known_by_their_whole_name() {
        assert_eq!(language("Dockerfile"), "Dockerfile");
        assert_eq!(language("sub/Makefile"), "Makefile");
        assert_eq!(language(".gitignore"), "Git");
        assert_eq!(language("CMakeLists.txt"), "CMake");
        assert_eq!(language("notas.txt"), "Texto");
    }
}
