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
        "dockerfile" | "containerfile" => ("code", design::brand_blue(), "Dockerfile"),
        "makefile" | "gnumakefile" | "justfile" => ("terminal", design::tint_green(), "Makefile"),
        "cmakelists.txt" => ("braces", design::tint_green(), "CMake"),
        ".gitignore" | ".gitattributes" | ".dockerignore" => ("git-commit", design::muted(), "Git"),
        ".env" | ".env.example" | ".editorconfig" => ("braces", design::muted(), "Configuración"),
        "license" | "licence" | "copying" => ("file-text", design::muted(), "Licencia"),
        _ => ("", design::muted(), ""),
    };
    if !language.is_empty() {
        return FileType {
            icon,
            tint,
            language,
        };
    }
    let (icon, tint, language) = match extension.as_str() {
        "java" => ("code", design::tint_orange(), "Java"),
        "kt" | "kts" => ("code", design::tint_purple(), "Kotlin"),
        "swift" => ("code", design::tint_orange(), "Swift"),
        "rb" => ("code", design::tint_orange(), "Ruby"),
        "php" => ("code", design::tint_purple(), "PHP"),
        "sql" => ("braces", design::brand_blue(), "SQL"),
        "vue" | "svelte" => ("code", design::tint_green(), "Componente web"),
        "lua" => ("code", design::brand_blue(), "Lua"),
        "cmake" => ("braces", design::tint_green(), "CMake"),
        "ini" | "cfg" | "conf" => ("braces", design::muted(), "Configuración"),
        "mdx" => ("file-text", design::brand_cyan(), "MDX"),
        "csv" | "tsv" => ("file-text", design::tint_green(), "Datos"),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" => {
            ("file", design::tint_purple(), "Imagen")
        }
        "rs" => ("code", design::tint_orange(), "Rust"),
        "py" => ("code", design::tint_yellow(), "Python"),
        "c" | "h" => ("code", design::brand_blue(), "C"),
        "cc" | "cpp" | "cxx" | "hpp" => ("code", design::brand_blue(), "C++"),
        "js" | "jsx" | "mjs" => ("code", design::tint_yellow(), "JavaScript"),
        "ts" | "tsx" => ("code", design::brand_blue(), "TypeScript"),
        "go" => ("code", design::brand_cyan(), "Go"),
        "cs" => ("code", design::tint_purple(), "C#"),
        "sh" | "bash" | "ps1" | "bat" | "cmd" => ("terminal", design::tint_green(), "Shell"),
        "toml" => ("braces", design::tint_orange(), "TOML"),
        "json" => ("braces", design::tint_yellow(), "JSON"),
        "yml" | "yaml" => ("braces", design::tint_purple(), "YAML"),
        "html" | "xml" | "svg" | "qml" | "ui" => ("code", design::tint_orange(), "Marcado"),
        "css" | "scss" | "qss" => ("braces", design::brand_blue(), "Estilos"),
        "diff" | "patch" => ("git-commit", design::tint_purple(), "Diff"),
        "md" | "markdown" => ("file-text", design::brand_cyan(), "Markdown"),
        "txt" | "log" => ("file-text", design::muted(), "Texto"),
        "lock" => ("file", design::faint(), "Lockfile"),
        _ => ("file", design::muted(), "Texto plano"),
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
