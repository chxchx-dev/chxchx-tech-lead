use crate::features::project_detection::ProjectDetection;
use serde::Serialize;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct ProjectConfig<'a> {
    version: u8,
    profile: &'a str,
    memory_project: String,
    memory_path: &'static str,
    workspace: Workspace<'a>,
}

#[derive(Serialize)]
struct Workspace<'a> {
    name: &'a str,
    adapter: &'static str,
    editor: &'static str,
    auto_open_editor: bool,
    auto_attach: bool,
    auto_start: bool,
    header: Header,
    layout: Layout,
    resources: Resources,
    processes: Vec<Process>,
    agents: Vec<Agent>,
    presets: std::collections::BTreeMap<&'static str, Vec<&'static str>>,
}

#[derive(Serialize)]
struct Header {
    enabled: bool,
    label: &'static str,
    logo: &'static str,
    template: &'static str,
}

#[derive(Serialize)]
struct Layout {
    orientation: &'static str,
}

#[derive(Serialize)]
struct Resources {
    warn_memory_percent: u8,
    critical_memory_percent: u8,
    warn_swap_percent: u8,
    max_agents: u8,
}

#[derive(Serialize)]
struct Process {
    id: &'static str,
    label: String,
    command: Vec<String>,
    cwd: String,
    auto_start: bool,
}

#[derive(Serialize)]
struct Agent {
    id: &'static str,
    command: Vec<&'static str>,
    cwd: &'static str,
    auto_start: bool,
}

pub fn render_default_config(
    root: &Path,
    name: &str,
    profile: &str,
    detection: &ProjectDetection,
) -> Result<String, String> {
    let config = ProjectConfig {
        version: 2,
        profile,
        memory_project: memory_project_name(root, name),
        memory_path: ".ai/memory",
        workspace: Workspace {
            name,
            adapter: "zellij",
            editor: "studio",
            auto_open_editor: false,
            auto_attach: true,
            auto_start: false,
            header: Header {
                enabled: true,
                label: "chxchx-tech",
                logo: "",
                template: "{logo} {label} | {project} | {profile}",
            },
            layout: Layout {
                orientation: "horizontal",
            },
            resources: Resources {
                warn_memory_percent: 70,
                critical_memory_percent: 85,
                warn_swap_percent: 40,
                max_agents: 2,
            },
            processes: suggested_processes(root, detection),
            agents: vec![
                Agent {
                    id: "codex",
                    command: vec!["codex"],
                    cwd: ".",
                    auto_start: false,
                },
                Agent {
                    id: "claude",
                    command: vec!["claude"],
                    cwd: ".",
                    auto_start: false,
                },
            ],
            presets: [("default", vec!["codex", "claude"])].into(),
        },
    };
    toml::to_string_pretty(&config)
        .map_err(|error| format!("No se pudo renderizar la configuración inicial: {error}"))
}

pub fn memory_project_name(root: &Path, name: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for character in name.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            separator = false;
            slug.push(character);
        } else {
            separator = true;
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "project" } else { slug };
    let mut digest = Sha1::digest(root.display().to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    digest.truncate(6);
    format!("{slug}-{digest}")
}

fn suggested_processes(root: &Path, detection: &ProjectDetection) -> Vec<Process> {
    let package = read_package_json(root);
    let scripts = package
        .get("scripts")
        .and_then(Value::as_object)
        .map(|scripts| scripts.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let dependencies = ["dependencies", "devDependencies"]
        .into_iter()
        .filter_map(|section| package.get(section).and_then(Value::as_object))
        .flat_map(|dependencies| dependencies.keys().cloned())
        .collect::<BTreeSet<_>>();
    let manager = ["pnpm", "yarn", "npm"]
        .into_iter()
        .find(|manager| {
            detection
                .package_managers
                .iter()
                .any(|item| item == manager)
        })
        .unwrap_or("npm");
    if let Some(script) = suggested_node_script(detection, &scripts, &dependencies) {
        let command = if script == "start" {
            vec![manager.to_owned(), "start".to_owned()]
        } else {
            vec![manager.to_owned(), "run".to_owned(), script.to_owned()]
        };
        let primary = ["react-native", "nextjs", "react-vite", "nestjs"]
            .into_iter()
            .find(|stack| detection.stacks.iter().any(|item| item == stack))
            .unwrap_or("");
        let label = if dependencies.contains("expo") {
            "Expo / React Native".to_owned()
        } else {
            match primary {
                "react-native" => "React Native / Metro",
                "nextjs" => "Next.js",
                "react-vite" => "Vite",
                "nestjs" => "NestJS",
                _ => "Node.js",
            }
            .to_owned()
        };
        return vec![Process {
            id: "dev-server",
            label,
            command,
            cwd: ".".to_owned(),
            auto_start: true,
        }];
    }
    if let Some(project) = find_csproj(root) {
        return vec![Process {
            id: "dotnet-watch",
            label: ".NET watch".to_owned(),
            command: vec!["dotnet".to_owned(), "watch".to_owned(), "run".to_owned()],
            cwd: project
                .parent()
                .and_then(|parent| parent.strip_prefix(root).ok())
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .filter(|path| !path.is_empty())
                .unwrap_or_else(|| ".".to_owned()),
            auto_start: true,
        }];
    }
    if root.join("manage.py").is_file() {
        let command = if root.join("uv.lock").exists() {
            vec!["uv", "run", "python", "manage.py", "runserver"]
        } else {
            vec!["python", "manage.py", "runserver"]
        };
        return vec![Process {
            id: "django",
            label: "Django".to_owned(),
            command: command.into_iter().map(str::to_owned).collect(),
            cwd: ".".to_owned(),
            auto_start: true,
        }];
    }
    if root.join("Cargo.toml").is_file() {
        return one_process("cargo-run", "Rust", ["cargo", "run"]);
    }
    if root.join("go.mod").is_file() && root.join("main.go").is_file() {
        return one_process("go-run", "Go", ["go", "run", "."]);
    }
    Vec::new()
}

fn suggested_node_script(
    detection: &ProjectDetection,
    scripts: &BTreeSet<String>,
    dependencies: &BTreeSet<String>,
) -> Option<&'static str> {
    let candidates: &[&str] = if detection.stacks.iter().any(|stack| stack == "react-native")
        || dependencies.contains("expo")
    {
        &["start", "dev"]
    } else if detection.stacks.iter().any(|stack| stack == "nestjs") {
        &["start:dev", "dev", "start"]
    } else {
        &["dev", "start"]
    };
    candidates
        .iter()
        .copied()
        .find(|candidate| scripts.contains(*candidate))
}

fn one_process<const N: usize>(id: &'static str, label: &str, command: [&str; N]) -> Vec<Process> {
    vec![Process {
        id,
        label: label.to_owned(),
        command: command.into_iter().map(str::to_owned).collect(),
        cwd: ".".to_owned(),
        auto_start: true,
    }]
}

fn read_package_json(root: &Path) -> Value {
    fs::read(root.join("package.json"))
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or(Value::Null)
}

fn find_csproj(root: &Path) -> Option<PathBuf> {
    let ignored = [".git", "node_modules", "bin", "obj", ".venv", "venv"];
    let mut pending = vec![root.to_owned()];
    let mut found = Vec::new();
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
                if !ignored.iter().any(|name| entry.file_name() == *name) {
                    pending.push(path);
                }
            } else if kind.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("csproj"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    (found.len() == 1).then(|| found.remove(0))
}
