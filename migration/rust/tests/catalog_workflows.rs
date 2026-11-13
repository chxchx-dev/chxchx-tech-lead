//! Skills, Tech Packs and Mods through the real CLI, in isolated temp projects.
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Sandbox {
    root: PathBuf,
    home: PathBuf,
    project: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "chxchx-catalog-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let (home, project) = (root.join("home"), root.join("project"));
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&project).unwrap();
        Self {
            root,
            home,
            project,
        }
    }

    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_chxchx-tech"))
            .args(args.iter().map(OsString::from))
            .arg("--path")
            .arg(&self.project)
            .env("CHXCHX_TECH_HOME", &self.home)
            .output()
            .expect("run the Rust CLI")
    }

    /// `list` commands take the project as a positional argument.
    fn list(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_chxchx-tech"))
            .args(args.iter().map(OsString::from))
            .arg(&self.project)
            .env("CHXCHX_TECH_HOME", &self.home)
            .output()
            .expect("run the Rust CLI")
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.project.join(relative)).unwrap_or_default()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn skills_preview_apply_disable_and_idempotence() {
    let sandbox = Sandbox::new();
    fs::write(
        sandbox.project.join("AGENTS.md"),
        "# Mi guía\n\nTexto manual.\n",
    )
    .unwrap();

    let preview = sandbox.cli(&["skills", "enable", "architecture", "testing"]);
    assert!(preview.status.success(), "{}", text(&preview));
    assert!(text(&preview).contains("habilitar la skill `architecture`"));
    assert!(
        !sandbox.project.join(".ai").exists(),
        "preview must not write"
    );

    let applied = sandbox.cli(&["skills", "enable", "architecture", "testing", "--apply"]);
    assert!(applied.status.success(), "{}", text(&applied));
    let selection = sandbox.read(".ai/chxchx-skills.toml");
    assert!(selection.contains("\"architecture\"") && selection.contains("\"testing\""));
    let context = sandbox.read(".ai/SKILLS.md");
    assert!(context.contains("## architecture") && context.contains("## testing"));
    let agents = sandbox.read("AGENTS.md");
    assert!(
        agents.contains("Texto manual."),
        "manual content must survive"
    );
    assert!(agents.contains("Skills habilitadas: architecture, testing."));
    assert!(text(&applied).contains("Backup:"), "{}", text(&applied));

    let again = sandbox.cli(&["skills", "enable", "architecture", "--apply"]);
    assert!(
        text(&again).contains("todo está actualizado"),
        "{}",
        text(&again)
    );

    let disabled = sandbox.cli(&["skills", "disable", "testing", "--apply"]);
    assert!(disabled.status.success(), "{}", text(&disabled));
    assert!(!sandbox.read(".ai/chxchx-skills.toml").contains("testing"));
    assert!(!sandbox.read(".ai/SKILLS.md").contains("## testing"));

    let unknown = sandbox.cli(&["skills", "enable", "does-not-exist"]);
    assert!(!unknown.status.success());
    assert!(text(&unknown).contains("Skill desconocida"));
}

#[test]
fn invalid_selection_is_reported_and_never_overwritten() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.project.join(".ai")).unwrap();
    let broken = "version = 2\nenabled = [\"architecture\"]\n";
    fs::write(sandbox.project.join(".ai/chxchx-skills.toml"), broken).unwrap();
    let result = sandbox.cli(&["skills", "enable", "testing", "--apply"]);
    assert!(!result.status.success());
    assert!(text(&result).contains("no compatible"), "{}", text(&result));
    assert_eq!(sandbox.read(".ai/chxchx-skills.toml"), broken);
}

#[test]
fn tech_pack_enables_all_of_its_skills() {
    let sandbox = Sandbox::new();
    let applied = sandbox.cli(&["packs", "apply", "python-backend", "--apply"]);
    assert!(applied.status.success(), "{}", text(&applied));
    let selection = sandbox.read(".ai/chxchx-skills.toml");
    for skill in [
        "python-engineering",
        "api-design",
        "testing",
        "security-review",
        "code-review",
    ] {
        assert!(selection.contains(skill), "missing {skill}");
    }
    let unknown = sandbox.cli(&["packs", "apply", "nope"]);
    assert!(text(&unknown).contains("Tech Pack desconocido"));
}

#[test]
fn mods_install_remove_and_protect_foreign_files() {
    let sandbox = Sandbox::new();
    let preview = sandbox.cli(&["mods", "apply", "chxchx-review-before-merge"]);
    assert!(preview.status.success(), "{}", text(&preview));
    assert!(
        !sandbox.project.join(".claude").exists(),
        "preview must not write"
    );

    let applied = sandbox.cli(&["mods", "apply", "chxchx-review-before-merge", "--apply"]);
    assert!(applied.status.success(), "{}", text(&applied));
    let installed = sandbox.read(".claude/skills/chxchx-review-before-merge/SKILL.md");
    assert!(installed.contains("<!-- chxchx-mod:chxchx-review-before-merge -->"));
    assert!(installed.starts_with("---\nname: chxchx-review-before-merge\n"));
    assert!(
        !sandbox.project.join(".agents").exists(),
        "Codex root only when Codex is used"
    );

    let again = sandbox.cli(&["mods", "apply", "chxchx-review-before-merge", "--apply"]);
    assert!(
        text(&again).contains("todo está actualizado"),
        "{}",
        text(&again)
    );

    let removed = sandbox.cli(&["mods", "remove", "chxchx-review-before-merge", "--apply"]);
    assert!(removed.status.success(), "{}", text(&removed));
    assert!(
        text(&removed).contains("Backup recuperable:"),
        "{}",
        text(&removed)
    );
    assert!(
        !sandbox
            .project
            .join(".claude/skills/chxchx-review-before-merge")
            .exists()
    );

    let foreign = sandbox
        .project
        .join(".claude/skills/chxchx-debug-with-evidence");
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("SKILL.md"), "mi propia skill\n").unwrap();
    let blocked = sandbox.cli(&["mods", "apply", "chxchx-debug-with-evidence", "--apply"]);
    assert!(!blocked.status.success(), "{}", text(&blocked));
    assert!(text(&blocked).contains("no administra ChxChx"));
    let kept = sandbox.cli(&["mods", "remove", "chxchx-debug-with-evidence", "--apply"]);
    assert!(!kept.status.success());
    assert_eq!(
        sandbox.read(".claude/skills/chxchx-debug-with-evidence/SKILL.md"),
        "mi propia skill\n"
    );
}

#[test]
fn codex_projects_get_mods_under_agents_skills() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.project.join(".agents")).unwrap();
    let applied = sandbox.cli(&["mods", "apply", "chxchx-safe-refactor", "--apply"]);
    assert!(applied.status.success(), "{}", text(&applied));
    assert!(
        sandbox
            .read(".agents/skills/chxchx-safe-refactor/SKILL.md")
            .contains("chxchx-mod:chxchx-safe-refactor")
    );
    assert!(
        sandbox
            .read(".claude/skills/chxchx-safe-refactor/SKILL.md")
            .contains("chxchx-mod:chxchx-safe-refactor")
    );
    let removed = sandbox.cli(&["mods", "remove", "chxchx-safe-refactor", "--apply"]);
    assert!(removed.status.success(), "{}", text(&removed));
    assert!(
        !sandbox
            .project
            .join(".agents/skills/chxchx-safe-refactor")
            .exists()
    );
}

#[test]
fn lists_mark_recommendations_for_the_detected_stack() {
    let sandbox = Sandbox::new();
    fs::write(
        sandbox.project.join("pyproject.toml"),
        "[project]\nname = \"demo\"\n",
    )
    .unwrap();
    fs::write(sandbox.project.join("main.py"), "print('hi')\n").unwrap();
    let packs = sandbox.list(&["packs", "list"]);
    assert!(packs.status.success(), "{}", text(&packs));
    assert!(
        text(&packs).contains("★ python-backend"),
        "{}",
        text(&packs)
    );
    let skills = sandbox.list(&["skills", "list"]);
    assert!(
        text(&skills).contains("★ architecture"),
        "{}",
        text(&skills)
    );
}
