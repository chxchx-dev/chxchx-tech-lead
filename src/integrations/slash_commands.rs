//! Slash commands an agent finds in a project: its skills and custom commands, read from the
//! folders each agent already uses (`.claude/skills`, `.claude/commands`, `.agents/skills`,
//! `.opencode/command`). Read-only; nothing is run or written here.
use std::fs;
use std::path::Path;

const MAX_COMMANDS: usize = 200;
const MAX_FILE: u64 = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub name: String,
    pub description: String,
    /// Where it came from: `skill` or `comando`.
    pub kind: &'static str,
}

/// `description:` of a Markdown file's front matter, folded to one line.
pub fn description(markdown: &str) -> String {
    let mut lines = markdown.lines();
    if lines.next().map(str::trim) != Some("---") {
        return String::new();
    }
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        if let Some(value) = line.strip_prefix("description:") {
            return value
                .trim()
                .trim_matches(['"', '\'', '>', '|'])
                .trim()
                .to_owned();
        }
    }
    String::new()
}

fn valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
}

fn read(path: &Path) -> Option<String> {
    let meta = fs::symlink_metadata(path).ok()?;
    (meta.is_file() && meta.len() <= MAX_FILE)
        .then(|| fs::read_to_string(path).ok())
        .flatten()
}

fn skills(dir: &Path, found: &mut Vec<Found>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(text) = read(&entry.path().join("SKILL.md"))
            && valid(&name)
        {
            found.push(Found {
                name,
                description: description(&text),
                kind: "skill",
            });
        }
    }
}

fn commands(dir: &Path, found: &mut Vec<Found>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = file.strip_suffix(".md") else {
            continue;
        };
        if let Some(text) = read(&entry.path())
            && valid(name)
        {
            found.push(Found {
                name: name.to_owned(),
                description: description(&text),
                kind: "comando",
            });
        }
    }
}

/// Skills and commands of `project` for `agent`, sorted by name, without duplicates.
pub fn discover(project: &Path, agent: &str) -> Vec<Found> {
    let mut found = Vec::new();
    match agent {
        "claude" => {
            commands(&project.join(".claude/commands"), &mut found);
            skills(&project.join(".claude/skills"), &mut found);
        }
        "opencode" => {
            commands(&project.join(".opencode/command"), &mut found);
            commands(&project.join(".opencode/commands"), &mut found);
            skills(&project.join(".agents/skills"), &mut found);
        }
        _ => skills(&project.join(".agents/skills"), &mut found),
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found.dedup_by(|a, b| a.name == b.name);
    found.truncate(MAX_COMMANDS);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_description_of_the_front_matter() {
        assert_eq!(
            description("---\nname: x\ndescription: \"Hace algo\"\n---\ncuerpo"),
            "Hace algo"
        );
        assert_eq!(description("sin front matter"), "");
    }

    #[test]
    fn finds_skills_and_commands_of_the_agent() {
        let root = std::env::temp_dir().join(format!("chxchx-slash-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".claude/skills/revisar")).unwrap();
        fs::create_dir_all(root.join(".claude/commands")).unwrap();
        fs::write(
            root.join(".claude/skills/revisar/SKILL.md"),
            "---\ndescription: Revisa cambios\n---\n",
        )
        .unwrap();
        fs::write(root.join(".claude/commands/desplegar.md"), "Despliega").unwrap();
        fs::write(root.join(".claude/commands/mal nombre.md"), "x").unwrap();
        let names: Vec<_> = discover(&root, "claude")
            .into_iter()
            .map(|f| (f.name, f.kind))
            .collect();
        assert_eq!(
            names,
            vec![("desplegar".into(), "comando"), ("revisar".into(), "skill")]
        );
        assert!(discover(&root, "codex").is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
