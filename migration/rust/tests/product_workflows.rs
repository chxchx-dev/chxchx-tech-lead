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
