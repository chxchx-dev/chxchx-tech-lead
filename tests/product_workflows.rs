use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_SANDBOX: AtomicU64 = AtomicU64::new(0);

struct Sandbox {
    root: PathBuf,
    home: PathBuf,
    project: PathBuf,
    bin: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let root = loop {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must be after UNIX epoch")
                .as_nanos();
            let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
            let candidate = std::env::temp_dir().join(format!(
                "chxchx-rust-tests-{}-{timestamp}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("failed to create test sandbox: {error}"),
            }
        };

        let home = root.join("home");
        let project = root.join("project");
        let bin = root.join("bin");
        fs::create_dir(&home).expect("create isolated application home");
        fs::create_dir(&project).expect("create project fixture");
        fs::create_dir(&bin).expect("create isolated PATH");
        Self {
            root,
            home,
            project,
            bin,
        }
    }

    fn cli(&self, args: impl IntoIterator<Item = OsString>) -> Output {
        Command::new(env!("CARGO_BIN_EXE_chxchx-tech"))
            .args(args)
            .env("CHXCHX_TECH_HOME", &self.home)
            .env("PATH", &self.bin)
            .output()
            .expect("run the Rust CLI")
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.project.join(relative)
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

fn assert_success(output: &Output) -> String {
    assert!(output.status.success(), "{}", text(output));
    text(output)
}

fn command(parts: &[&str], project: &Path) -> Vec<OsString> {
    parts
        .iter()
        .map(OsString::from)
        .chain(std::iter::once(project.as_os_str().to_owned()))
        .collect()
}

#[test]
fn minimal_init_previews_without_writes_and_apply_is_idempotent() {
    let sandbox = Sandbox::new();
    let preview = sandbox.cli(command(&["init", "--minimal"], &sandbox.project));
    let preview_text = assert_success(&preview);
    assert!(
        !preview_text.contains("No se puede aplicar:"),
        "{preview_text}"
    );
    assert!(!sandbox.path(".ai").exists());
    assert!(!sandbox.home.join("projects.json").exists());

    let applied = sandbox.cli(command(&["init", "--minimal", "--apply"], &sandbox.project));
    assert_success(&applied);
    assert!(sandbox.path(".ai/chxchx-tech.toml").is_file());
    assert!(sandbox.path(".ai/memory").is_dir());
    let config_before = fs::read(sandbox.path(".ai/chxchx-tech.toml")).unwrap();
    let registry_before = fs::read(sandbox.home.join("projects.json")).unwrap();

    let repeated = sandbox.cli(command(&["init", "--minimal", "--apply"], &sandbox.project));
    assert_success(&repeated);
    assert_eq!(
        fs::read(sandbox.path(".ai/chxchx-tech.toml")).unwrap(),
        config_before
    );
    assert_eq!(
        fs::read(sandbox.home.join("projects.json")).unwrap(),
        registry_before
    );
}

#[test]
fn trust_requires_apply_and_untrust_removes_only_the_project_entry() {
    let sandbox = Sandbox::new();
    let trust_file = sandbox.home.join("trusted_projects.json");
    let preview = sandbox.cli(command(&["workspace", "trust"], &sandbox.project));
    assert_success(&preview);
    assert!(!trust_file.exists());

    let apply = sandbox.cli(command(
        &["workspace", "trust", "--apply"],
        &sandbox.project,
    ));
    assert_success(&apply);
    let trusted: serde_json::Value =
        serde_json::from_slice(&fs::read(&trust_file).unwrap()).unwrap();
    let entries = trusted.as_object().unwrap();
    assert_eq!(entries.len(), 1);
    let project_key = entries.keys().next().unwrap().clone();
    assert!(trusted[&project_key].is_string());

    let untrust_preview = sandbox.cli(command(&["workspace", "untrust"], &sandbox.project));
    assert_success(&untrust_preview);
    assert!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&trust_file).unwrap()).unwrap()
            [&project_key]
            .is_string()
    );

    let untrust = sandbox.cli(command(
        &["workspace", "untrust", "--apply"],
        &sandbox.project,
    ));
    assert_success(&untrust);
    let remaining: serde_json::Value =
        serde_json::from_slice(&fs::read(trust_file).unwrap()).unwrap();
    assert!(remaining.get(&project_key).is_none());
    assert!(sandbox.home.join("backups/global").is_dir());
}

#[test]
fn project_catalog_treats_missing_as_empty_and_preserves_invalid_json() {
    let sandbox = Sandbox::new();
    let empty = sandbox.cli([OsString::from("projects"), OsString::from("list")]);
    assert_success(&empty);
    assert!(!sandbox.home.join("projects.json").exists());

    let registry = sandbox.home.join("projects.json");
    let malformed = b"{invalid";
    fs::write(&registry, malformed).unwrap();
    let result = sandbox.cli([OsString::from("projects"), OsString::from("list")]);
    assert!(!result.status.success());
    assert!(text(&result).contains("JSON inválido"));
    assert_eq!(fs::read(registry).unwrap(), malformed);
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn status_json(sandbox: &Sandbox) -> serde_json::Value {
    let output = sandbox.cli(command(&["status", "--json"], &sandbox.project));
    serde_json::from_str(&assert_success(&output)).expect("status --json prints JSON")
}

/// A person's whole first hour with a project: look, prepare, edit, enable skills, break the
/// managed block, and get back to the previous state, with a preview before every change.
#[test]
fn a_project_goes_from_empty_to_prepared_and_back_with_previews_and_backups() {
    let sandbox = Sandbox::new();

    // An empty folder: nothing configured, nothing trusted, nothing written by looking.
    let overview = status_json(&sandbox);
    assert_eq!(overview["schema"], "chxchx.project-overview");
    assert_eq!(overview["workspace"]["config_exists"], false);
    assert_eq!(overview["workspace"]["trusted"], false);
    assert!(!sandbox.path(".ai").exists());

    // Preparing it previews first, then writes the guides and the managed config.
    assert_success(&sandbox.cli(command(&["init"], &sandbox.project)));
    assert!(
        !sandbox.path("AGENTS.md").exists(),
        "a preview writes nothing"
    );
    assert_success(&sandbox.cli(command(&["init", "--apply"], &sandbox.project)));
    for file in [
        "AGENTS.md",
        "CLAUDE.md",
        ".ai/PROJECT.md",
        ".ai/chxchx-tech.toml",
    ] {
        assert!(sandbox.path(file).is_file(), "{file} was not created");
    }
    let overview = status_json(&sandbox);
    assert_eq!(overview["workspace"]["config_exists"], true);
    assert_eq!(overview["workspace"]["config_supported"], true);

    // What the person wrote next to the managed block survives a new init, which then changes nothing.
    let agents = sandbox.path("AGENTS.md");
    fs::write(&agents, format!("{}\n# Mi nota propia\n", read(&agents))).unwrap();
    let again = assert_success(&sandbox.cli(command(&["init", "--apply"], &sandbox.project)));
    assert!(read(&agents).contains("# Mi nota propia"), "{again}");
    assert!(again.contains("sin cambios locales"), "{again}");

    // Enabling a skill previews, then applies with a backup and regenerates the context.
    let enable = |apply: bool| {
        let mut args: Vec<OsString> = ["skills", "enable", "architecture", "--path"]
            .into_iter()
            .map(OsString::from)
            .collect();
        args.push(sandbox.project.clone().into_os_string());
        if apply {
            args.push("--apply".into());
        }
        sandbox.cli(args)
    };
    assert_success(&enable(false));
    assert!(!sandbox.path(".ai/SKILLS.md").exists());
    let applied = assert_success(&enable(true));
    assert!(applied.contains("Backup:"), "{applied}");
    assert!(read(&sandbox.path(".ai/SKILLS.md")).contains("architecture"));

    // Someone damages the managed block: init repairs it (keeping a backup)...
    let healthy = read(&agents);
    fs::write(
        &agents,
        healthy.replace("Perfil detectado:", "Perfil TAMPERED:"),
    )
    .unwrap();
    assert!(read(&agents).contains("TAMPERED"));
    assert_success(&sandbox.cli(command(&["init", "--apply"], &sandbox.project)));
    assert!(
        !read(&agents).contains("TAMPERED"),
        "init must repair the managed block"
    );

    // ...and rollback goes back to the previous saved state, previewing before it touches anything.
    let preview = assert_success(&sandbox.cli(command(&["rollback"], &sandbox.project)));
    assert!(preview.contains("restaurar AGENTS.md"), "{preview}");
    assert!(
        !read(&agents).contains("TAMPERED"),
        "a preview must not restore"
    );
    let restored =
        assert_success(&sandbox.cli(command(&["rollback", "--apply"], &sandbox.project)));
    assert!(restored.contains("Backup de seguridad"), "{restored}");
    assert!(
        read(&agents).contains("TAMPERED"),
        "the backup held the damaged state"
    );

    // Trust is its own explicit decision.
    assert_eq!(status_json(&sandbox)["workspace"]["trusted"], false);
    assert_success(&sandbox.cli(command(
        &["workspace", "trust", "--apply"],
        &sandbox.project,
    )));
    assert_eq!(status_json(&sandbox)["workspace"]["trusted"], true);
}
