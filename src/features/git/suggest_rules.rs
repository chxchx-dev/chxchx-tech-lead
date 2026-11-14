//! The rules behind the suggestions: what a file is, and how to say it.

pub(super) const GENERIC_FOLDERS: &[&str] = &[
    "src",
    "lib",
    "app",
    "apps",
    "packages",
    "source",
    "sources",
    "crates",
    "internal",
    "pkg",
    "cmd",
    "migration",
];

use super::{ChangeInfo, CommitType};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Class {
    Source,
    Test,
    Docs,
    Build,
    Ci,
    Chore,
}

pub(super) fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub(super) fn classify(path: &str) -> Class {
    let lower = path.to_lowercase();
    let name = file_name(&lower);
    let extension = name.rsplit_once('.').map_or("", |(_, ext)| ext);
    let folders: Vec<&str> = lower.split('/').collect();
    let in_folder = |wanted: &[&str]| {
        folders[..folders.len() - 1]
            .iter()
            .any(|folder| wanted.contains(folder))
    };
    if lower.starts_with(".github/")
        || lower.starts_with(".circleci/")
        || name == ".gitlab-ci.yml"
        || name == "jenkinsfile"
        || name.starts_with("azure-pipelines")
    {
        return Class::Ci;
    }
    if in_folder(&["tests", "test", "__tests__", "spec", "specs"])
        || name.starts_with("test_")
        || name.contains("_test.")
        || name.contains(".test.")
        || name.contains(".spec.")
    {
        return Class::Test;
    }
    const BUILD_FILES: &[&str] = &[
        "cargo.toml",
        "cargo.lock",
        "build.rs",
        "package.json",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "pyproject.toml",
        "uv.lock",
        "poetry.lock",
        "pipfile",
        "pipfile.lock",
        "cmakelists.txt",
        "makefile",
        "dockerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        ".dockerignore",
        "setup.py",
        "setup.cfg",
        "go.mod",
        "go.sum",
        "gemfile",
        "gemfile.lock",
    ];
    if BUILD_FILES.contains(&name)
        || name.starts_with("requirements") && extension == "txt"
        || extension == "cmake"
    {
        return Class::Build;
    }
    if lower.starts_with("docs/")
        || lower.starts_with(".ai/")
        || matches!(extension, "md" | "rst" | "adoc")
        || ["readme", "changelog", "license", "contributing"]
            .iter()
            .any(|stem| name.starts_with(stem))
    {
        return Class::Docs;
    }
    if name.starts_with('.')
        || in_folder(&[".vscode", ".idea", ".claude", ".codex"])
        || matches!(
            extension,
            "png" | "jpg" | "jpeg" | "gif" | "ico" | "svg" | "webp" | "woff" | "woff2" | "ttf"
        )
        || matches!(name, "rustfmt.toml" | "clippy.toml" | "rust-toolchain.toml")
    {
        return Class::Chore;
    }
    Class::Source
}

/// The module a path belongs to: its first folder that is not a generic container.
pub(super) fn area(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    parts
        .into_iter()
        .find(|part| !GENERIC_FOLDERS.contains(part) && !part.starts_with('.'))
        .map(str::to_owned)
}

pub(super) fn stem(path: &str) -> String {
    let name = file_name(path);
    name.rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_owned()
}

/// "a", "a y b", "a, b y c", or "N archivos" when there are too many to name.
pub(super) fn names(paths: &[&ChangeInfo]) -> String {
    let mut stems: Vec<String> = paths.iter().map(|change| stem(&change.path)).collect();
    stems.sort();
    stems.dedup();
    match stems.as_slice() {
        [] => "archivos".to_owned(),
        [one] => one.clone(),
        [a, b] => format!("{a} y {b}"),
        [a, b, c] => format!("{a}, {b} y {c}"),
        [a, b, c, rest @ ..] => format!("{a}, {b}, {c} y {} más", rest.len()),
    }
}

pub(super) fn kind_for_source(files: &[&ChangeInfo]) -> CommitType {
    let new = files.iter().filter(|file| file.badge == 'N').count();
    let deleted = files.iter().filter(|file| file.badge == 'D').count();
    let additions: u32 = files.iter().map(|file| file.additions).sum();
    let deletions: u32 = files.iter().map(|file| file.deletions).sum();
    let hints: u32 = files.iter().map(|file| file.fix_hints).sum();
    if hints > 0 && new == 0 {
        CommitType::Fix
    } else if new > 0 && new * 2 >= files.len() {
        CommitType::Feat
    } else if deleted == files.len() || deletions > additions.saturating_mul(2) {
        CommitType::Refactor
    } else if additions + deletions <= 30 && new == 0 {
        CommitType::Fix
    } else if additions > deletions.saturating_mul(2) {
        CommitType::Feat
    } else {
        CommitType::Refactor
    }
}

pub(super) fn describe(kind: CommitType, class: Class, files: &[&ChangeInfo]) -> String {
    let what = names(files);
    let new = files.iter().filter(|file| file.badge == 'N').count();
    let deleted_only = files.iter().all(|file| file.badge == 'D');
    match (kind, class) {
        (_, Class::Docs) => {
            if files.len() == 1 {
                format!("actualiza la documentación ({what})")
            } else {
                "actualiza la documentación".to_owned()
            }
        }
        (_, Class::Ci) => "ajusta los workflows de CI".to_owned(),
        (_, Class::Build) => {
            if files
                .iter()
                .all(|file| file_name(&file.path).contains("lock"))
            {
                "actualiza dependencias".to_owned()
            } else {
                "actualiza dependencias y compilación".to_owned()
            }
        }
        (_, Class::Chore) => "ajusta configuración y archivos del proyecto".to_owned(),
        (_, Class::Test) => {
            if new > 0 {
                format!("añade pruebas de {what}")
            } else {
                format!("actualiza pruebas de {what}")
            }
        }
        (CommitType::Feat, _) => {
            if new * 2 >= files.len() {
                format!("añade {what}")
            } else {
                format!("mejora {what}")
            }
        }
        (CommitType::Fix, _) => format!("corrige {what}"),
        (CommitType::Refactor, _) => {
            if deleted_only {
                format!("elimina {what}")
            } else {
                format!("reorganiza {what}")
            }
        }
        (CommitType::Perf, _) => format!("acelera {what}"),
        (CommitType::Style, _) => format!("da formato a {what}"),
        _ => format!("actualiza {what}"),
    }
}

pub(super) fn body_for(files: &[&ChangeInfo]) -> Vec<String> {
    let mut lines: Vec<String> = files
        .iter()
        .take(10)
        .map(|file| match file.badge {
            'N' => format!("- {} (nuevo, +{})", file.path, file.additions),
            'D' => format!("- {} (eliminado, −{})", file.path, file.deletions),
            _ => format!("- {} (+{}/−{})", file.path, file.additions, file.deletions),
        })
        .collect();
    if files.len() > 10 {
        lines.push(format!("- … y {} archivos más", files.len() - 10));
    }
    lines
}

/// Folders below `area`, ignoring generic containers: `features/git` for
/// `rust/src/features/git/x.rs`.
pub(super) fn sub_area(path: &str, area: &str) -> Option<String> {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    let rest: Vec<&str> = parts
        .into_iter()
        .skip_while(|part| *part != area)
        .skip(1)
        .filter(|part| !GENERIC_FOLDERS.contains(part))
        .take(2)
        .collect();
    (!rest.is_empty()).then(|| rest.join("/"))
}

/// First one or two words of a file name: `studio_git_view.rs` -> `studio_git`.
pub(super) fn name_prefix(path: &str) -> String {
    let stem = stem(path);
    let words: Vec<&str> = stem.split(['_', '-']).collect();
    if words.len() >= 2 {
        words[..2].join("_")
    } else {
        words[0].to_owned()
    }
}
