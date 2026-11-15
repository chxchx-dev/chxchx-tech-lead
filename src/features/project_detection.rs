use crate::features::project_profiles::resolve_profile;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct ProjectDetection {
    pub stacks: Vec<String>,
    pub languages: Vec<String>,
    pub package_managers: Vec<String>,
    pub infrastructure: Vec<String>,
    pub profile: String,
    pub profile_rules: Vec<String>,
}

#[derive(Default)]
struct TreeScan {
    suffixes: BTreeSet<String>,
    csproj: Vec<PathBuf>,
    qt_candidates: Vec<PathBuf>,
}

pub(crate) const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    ".venv",
    "venv",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".turbo",
    ".nx",
    "coverage",
    "bin",
    "obj",
    ".idea",
    ".vscode",
    "target",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".tox",
    ".gradle",
    "Pods",
];

pub fn detect_project(root: &Path) -> ProjectDetection {
    let package = read_package_json(root);
    let scan = scan_tree(root);
    let dependencies = package_dependencies(&package);
    let mut stacks = Vec::new();
    let mut languages = Vec::new();

    if contains_any(
        &scan.suffixes,
        &[".cc", ".cpp", ".cxx", ".c++", ".hh", ".hpp", ".hxx", ".ipp"],
    ) {
        languages.push("cpp".to_owned());
        stacks.push("cpp".to_owned());
    }
    if detects_qt(&scan) {
        stacks.push("qt".to_owned());
    }
    let mut javascript_stacks = Vec::new();
    if dependencies.contains("next") {
        javascript_stacks.push("nextjs");
    }
    if dependencies.contains("react-native") {
        javascript_stacks.push("react-native");
    }
    if dependencies.contains("@nestjs/core") {
        javascript_stacks.push("nestjs");
    }
    if dependencies.contains("react") && !dependencies.contains("next") {
        javascript_stacks.push(if dependencies.contains("vite") {
            "react-vite"
        } else {
            "react"
        });
    }
    if dependencies.contains("prisma") || dependencies.contains("@prisma/client") {
        javascript_stacks.push("prisma");
    }
    if has_redis_package(&dependencies) {
        javascript_stacks.push("redis");
    }
    javascript_stacks.sort_unstable();
    stacks.extend(javascript_stacks.into_iter().map(str::to_owned));

    if root_has_extension(root, "sln") || !scan.csproj.is_empty() {
        stacks.push("dotnet".to_owned());
    }
    if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        stacks.push("python".to_owned());
    }
    if root.join("Cargo.toml").exists() {
        stacks.push("rust".to_owned());
    }
    if root.join("go.mod").exists() {
        stacks.push("go".to_owned());
    }
    if detects_postgres(root, &dependencies, &scan) {
        stacks.push("postgres".to_owned());
    }
    if detects_redis(root, &dependencies, &scan) && !stacks.iter().any(|stack| stack == "redis") {
        stacks.push("redis".to_owned());
    }
    if detects_docker(root) {
        stacks.push("docker".to_owned());
    }
    if detects_ai(root, &dependencies) {
        stacks.push("ai".to_owned());
    }

    let package_managers = [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("uv.lock", "uv"),
    ]
    .into_iter()
    .filter(|(file, _)| root.join(file).exists())
    .map(|(_, manager)| manager.to_owned())
    .collect();

    let language_suffixes = [
        (".ts", "typescript"),
        (".tsx", "typescript"),
        (".js", "javascript"),
        (".jsx", "javascript"),
        (".cs", "csharp"),
        (".py", "python"),
        (".kt", "kotlin"),
        (".swift", "swift"),
        (".sql", "sql"),
        (".rs", "rust"),
        (".go", "go"),
        (".cc", "cpp"),
        (".cpp", "cpp"),
        (".cxx", "cpp"),
        (".c++", "cpp"),
        (".hh", "cpp"),
        (".hpp", "cpp"),
        (".hxx", "cpp"),
        (".ipp", "cpp"),
    ];
    for (suffix, language) in language_suffixes {
        if scan.suffixes.contains(suffix) && !languages.iter().any(|item| item == language) {
            languages.push(language.to_owned());
        }
    }

    let infrastructure = if root.join(".github").join("workflows").exists() {
        vec!["github-actions".to_owned()]
    } else {
        Vec::new()
    };
    if stacks.is_empty() {
        stacks.push("generic".to_owned());
    }
    let profile = resolve_profile(&stacks);
    ProjectDetection {
        stacks,
        languages,
        package_managers,
        infrastructure,
        profile: profile.name,
        profile_rules: profile.rules,
    }
}

fn scan_tree(root: &Path) -> TreeScan {
    let mut scan = TreeScan::default();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if !IGNORED_DIRECTORIES
                    .iter()
                    .any(|ignored| entry.file_name() == *ignored)
                {
                    pending.push(path);
                }
                continue;
            }
            let is_file = kind.is_file()
                || (kind.is_symlink()
                    && fs::metadata(&path).is_ok_and(|metadata| metadata.is_file()));
            if !is_file {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if let Some(extension) = Path::new(&name).extension() {
                scan.suffixes
                    .insert(format!(".{}", extension.to_string_lossy()));
            }
            if name.ends_with(".csproj") {
                scan.csproj.push(path.clone());
            }
            if name == "cmakelists.txt"
                || name == "meson.build"
                || [".cmake", ".pro", ".pri"]
                    .iter()
                    .any(|suffix| name.ends_with(suffix))
            {
                scan.qt_candidates.push(path);
            }
        }
    }
    scan
}

fn read_package_json(root: &Path) -> Value {
    fs::read(root.join("package.json"))
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or(Value::Null)
}

fn package_dependencies(package: &Value) -> BTreeSet<String> {
    ["dependencies", "devDependencies"]
        .into_iter()
        .filter_map(|section| package.get(section).and_then(Value::as_object))
        .flat_map(|dependencies| dependencies.keys().cloned())
        .collect()
}

fn detects_qt(scan: &TreeScan) -> bool {
    scan.qt_candidates.iter().any(|path| {
        let Ok(bytes) = fs::read(path) else {
            return false;
        };
        let contents =
            String::from_utf8_lossy(&bytes[..bytes.len().min(512 * 1024)]).to_lowercase();
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_lowercase());
        contents.contains("qt6")
            || contents.contains("qt5")
            || (matches!(extension.as_deref(), Some("pro" | "pri")) && contents.contains("qt +="))
    })
}

fn detects_postgres(root: &Path, dependencies: &BTreeSet<String>, scan: &TreeScan) -> bool {
    dependencies
        .iter()
        .any(|dependency| matches!(dependency.as_str(), "pg" | "postgres" | "postgresql"))
        || csproj_mentions(scan, "npgsql")
        || requirements_contain(root, &["psycopg", "asyncpg"])
}

fn detects_redis(root: &Path, dependencies: &BTreeSet<String>, scan: &TreeScan) -> bool {
    has_redis_package(dependencies)
        || read_dependency_files(root)
            .iter()
            .any(|contents| contains_redis_dependency(contents))
        || csproj_mentions(scan, "stackexchange.redis")
}

fn detects_ai(root: &Path, dependencies: &BTreeSet<String>) -> bool {
    let markers = [
        "openai",
        "anthropic",
        "langchain",
        "llama-index",
        "transformers",
    ];
    dependencies
        .iter()
        .any(|dependency| markers.contains(&dependency.as_str()))
        || requirements_contain(root, &markers)
}

fn detects_docker(root: &Path) -> bool {
    [
        "Dockerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ]
    .iter()
    .any(|file| root.join(file).is_file())
}

fn requirements_contain(root: &Path, markers: &[&str]) -> bool {
    read_dependency_files(root).iter().any(|contents| {
        let contents = contents.to_lowercase();
        markers.iter().any(|marker| contents.contains(marker))
    })
}

fn read_dependency_files(root: &Path) -> Vec<String> {
    ["pyproject.toml", "requirements.txt", "requirements-dev.txt"]
        .into_iter()
        .filter_map(|file| fs::read_to_string(root.join(file)).ok())
        .collect()
}

fn contains_redis_dependency(contents: &str) -> bool {
    let lowered = contents.to_lowercase();
    lowered.match_indices("redis").any(|(position, _)| {
        let line_start = lowered[..position].rfind('\n').map_or(0, |index| index + 1);
        let before = &lowered[line_start..position];
        let prefix_matches = before.trim().is_empty()
            || before
                .trim_end()
                .chars()
                .last()
                .is_some_and(|character| "\"', [".contains(character));
        let after = lowered[position + "redis".len()..].chars().next();
        prefix_matches
            && after.is_some_and(|character| {
                character.is_whitespace() || "<>=!~;[\"'".contains(character)
            })
    })
}

fn has_redis_package(dependencies: &BTreeSet<String>) -> bool {
    dependencies
        .iter()
        .any(|dependency| matches!(dependency.as_str(), "redis" | "ioredis" | "@redis/client"))
}

fn csproj_mentions(scan: &TreeScan, marker: &str) -> bool {
    scan.csproj.iter().any(|path| {
        fs::read_to_string(path).is_ok_and(|contents| contents.to_lowercase().contains(marker))
    })
}

fn root_has_extension(root: &Path, extension: &str) -> bool {
    fs::read_dir(root).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|value| value == extension)
        })
    })
}

fn contains_any(values: &BTreeSet<String>, expected: &[&str]) -> bool {
    expected.iter().any(|value| values.contains(*value))
}
