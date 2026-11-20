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
fn rust_cli_previews_and_prunes_automatic_memory_with_a_recoverable_backup() {
    let sandbox = Sandbox::new();
    let memory_path = sandbox.path(".ai/memory/PROJECT_MEMORY.md");
    fs::create_dir_all(memory_path.parent().unwrap()).unwrap();
    let entry = |text: &str| {
        format!(
            "### Petición explícita de memoria — 2026-11-18\n\n<!-- explicit-memory:Claude:s1:0123456789ab -->\nOrigen: Claude, conversación `s1`.\n\n> {text}\n"
        )
    };
    let original = format!(
        "# Memoria\n\n## Notas propias\n\nUsamos Qt 6.8.\n\n{}\n{}\n{}\n{}\n## Más notas\n\nNo tocar.\n",
        entry("Revisa el uso de memoria RAM."),
        entry("Recuerda que el deploy es los viernes."),
        entry("recuerda que el deploy es los viernes."),
        entry("Recuerda mi contraseña: no-guardar-esta.")
    );
    fs::write(&memory_path, &original).unwrap();

    let mut preview_args = command(&["workspace", "memory-prune"], &sandbox.project);
    preview_args.push("--dry-run".into());
    let preview = sandbox.cli(preview_args);
    let preview_text = assert_success(&preview);
    assert!(
        preview_text.contains("se retirarían 3 entrada(s)"),
        "{preview_text}"
    );
    assert!(preview_text.contains("memoria RAM"));
    assert_eq!(fs::read_to_string(&memory_path).unwrap(), original);
    assert!(!sandbox.home.join("backups").exists());

    let applied = sandbox.cli(command(&["workspace", "memory-prune"], &sandbox.project));
    let applied_text = assert_success(&applied);
    assert!(
        applied_text.contains("Retiradas 3 entrada(s)"),
        "{applied_text}"
    );
    assert!(applied_text.contains("Backup:"), "{applied_text}");
    let pruned = fs::read_to_string(&memory_path).unwrap();
    assert!(pruned.contains("Usamos Qt 6.8."));
    assert!(pruned.contains("No tocar."));
    assert_eq!(pruned.matches("Recuerda que el deploy").count(), 1);
    assert!(!pruned.contains("memoria RAM"));
    assert!(!pruned.contains("no-guardar-esta"));

    let project_backup = fs::read_dir(sandbox.home.join("backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let backup_dir = fs::read_dir(project_backup.join("memory"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let backup_file = backup_dir.join("PROJECT_MEMORY.md");
    assert_eq!(fs::read_to_string(&backup_file).unwrap(), original);

    let repeated = sandbox.cli(command(&["workspace", "memory-prune"], &sandbox.project));
    assert!(assert_success(&repeated).contains("ya está limpia"));
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

#[cfg(unix)]
#[test]
fn configured_processes_require_trust_and_support_dry_run_start_stop_and_listing() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.path(".ai")).unwrap();
    fs::write(
        sandbox.path(".ai/chxchx-tech.toml"),
        "version=2\n[workspace]\nname='fixture'\n[[workspace.processes]]\nid='worker'\nlabel='Worker'\ncommand=['/bin/sleep','30']\ncwd='.'\n",
    ).unwrap();

    let denied = sandbox.cli(
        ["process", "start", "worker", "--path"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert!(!denied.status.success());
    assert!(
        text(&denied).contains("no es confiable"),
        "{}",
        text(&denied)
    );

    let trust = sandbox.cli(
        ["workspace", "trust", "--apply"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert_success(&trust);
    let dry = sandbox.cli(
        ["process", "start", "worker", "--path"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned(), "--dry-run".into()]),
    );
    assert!(assert_success(&dry).contains("DRY RUN"));
    assert!(!sandbox.home.join("workspace-state.json").exists());

    let start = sandbox.cli(
        ["process", "start", "worker", "--path"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    let started = assert_success(&start);
    assert!(started.contains("Iniciado `worker`"), "{started}");
    let duplicate = sandbox.cli(
        ["process", "start", "worker", "--path"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    let duplicate_text = assert_success(&duplicate);
    assert!(
        duplicate_text.contains("ya está ejecutándose"),
        "{duplicate_text}\nstate: {}",
        fs::read_to_string(sandbox.home.join("workspace-state.json")).unwrap_or_default()
    );
    let listed = sandbox.cli(
        ["process", "list"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert!(assert_success(&listed).contains("worker\tRunning"));
    let stopped = sandbox.cli(
        ["process", "stop", "worker", "--path"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert!(assert_success(&stopped).contains("Detenido `worker`"));
}

#[cfg(unix)]
#[test]
fn short_lived_configured_process_records_its_exit_and_exposes_a_bounded_log_tail() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.path(".ai")).unwrap();
    fs::write(
        sandbox.path(".ai/chxchx-tech.toml"),
        r#"version=2
[workspace]
name='fixture'
[[workspace.processes]]
id='worker'
label='Worker'
command=["/bin/sh","-c","printf 'one\\ntwo\\nthree\\n'"]
cwd='.'
"#,
    )
    .unwrap();
    let trust = sandbox.cli(
        ["workspace", "trust", "--apply"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert_success(&trust);

    let started = sandbox.cli(
        ["process", "start", "worker", "--path"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert!(assert_success(&started).contains("terminó durante el inicio"));

    let log = sandbox.cli(
        ["process", "log", "worker", "--path"]
            .into_iter()
            .map(OsString::from)
            .chain([
                sandbox.project.as_os_str().to_owned(),
                "--lines".into(),
                "2".into(),
            ]),
    );
    assert_eq!(assert_success(&log).trim(), "two\nthree");

    let listed = sandbox.cli(
        ["process", "list"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert!(assert_success(&listed).contains("worker\tExited"));
}

#[test]
fn resources_overview_matches_registered_projects_without_writing_workspace_state() {
    let sandbox = Sandbox::new();
    let other = sandbox.root.join("other-project");
    fs::create_dir(&other).unwrap();
    fs::write(
        sandbox.home.join("projects.json"),
        serde_json::json!({
            "projects": [{"alias": "other-app", "name": "Other", "path": other}]
        })
        .to_string(),
    )
    .unwrap();

    let output = sandbox.cli(command(&["resources", "--json"], &sandbox.project));
    let payload: serde_json::Value =
        serde_json::from_str(&assert_success(&output)).expect("resources prints JSON");

    assert_eq!(payload["schema"], "chxchx.resources-overview");
    assert_eq!(payload["schema_version"], 1);
    assert!(payload["system"]["memory_total_bytes"].as_u64().unwrap() > 0);
    let projects = payload["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    assert!(projects.iter().any(|project| {
        project["alias"] == "other-app" && project["path"] == other.display().to_string()
    }));
    assert!(projects.iter().any(|project| {
        project["path"] == sandbox.project.display().to_string() && project["max_agents"] == 2
    }));
    assert!(
        !sandbox.home.join("workspace-state.json").exists(),
        "resource inspection must stay read-only"
    );
}

#[cfg(unix)]
#[test]
fn workspace_lifecycle_previews_auto_start_and_preserves_active_suspended_and_stopped_states() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.path(".ai")).unwrap();
    fs::write(
        sandbox.path(".ai/chxchx-tech.toml"),
        "version=2\n[workspace]\nname='Lifecycle fixture'\nauto_start=true\n[[workspace.processes]]\nid='worker'\nlabel='Worker'\ncommand=['/bin/sleep','30']\ncwd='.'\nauto_start=true\n",
    )
    .unwrap();
    let trust = sandbox.cli(
        ["workspace", "trust", "--apply"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert_success(&trust);

    let preview = sandbox.cli(
        ["workspace", "start", "--dry-run"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert!(assert_success(&preview).contains("DRY RUN: iniciar workspace"));
    assert!(!sandbox.home.join("workspace-state.json").exists());

    let start = sandbox.cli(
        ["workspace", "start"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert!(assert_success(&start).contains("Iniciado `worker`"));
    let state_path = sandbox.home.join("workspace-state.json");
    let state: serde_json::Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    let key = sandbox
        .project
        .canonicalize()
        .unwrap()
        .display()
        .to_string();
    assert_eq!(state["workspaces"][&key]["status"], "ACTIVE");
    assert_eq!(state["workspaces"][&key]["project_path"], key);
    assert_eq!(
        state["workspaces"][&key]["session_name"],
        "Lifecycle-fixture"
    );

    let suspend = sandbox.cli(
        ["workspace", "suspend"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert_success(&suspend);
    let suspended: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    assert_eq!(suspended["workspaces"][&key]["status"], "SUSPENDED");

    let resume = sandbox.cli(
        ["workspace", "resume"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert_success(&resume);
    let resumed: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    assert_eq!(resumed["workspaces"][&key]["status"], "ACTIVE");

    let stop = sandbox.cli(
        ["workspace", "stop"]
            .into_iter()
            .map(OsString::from)
            .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert_success(&stop);
    let stopped: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    assert_eq!(stopped["workspaces"][&key]["status"], "STOPPED");
    assert_eq!(
        stopped["workspaces"][&key]["process_ids"],
        serde_json::json!([])
    );
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

fn project(sandbox: &Sandbox) -> String {
    sandbox.project.display().to_string()
}

impl Sandbox {
    /// Runs the CLI as a terminal would, with extra environment variables. The color variables of
    /// the machine running the tests are cleared first so the result does not depend on them.
    fn cli_with(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chxchx-tech"));
        command
            .args(args)
            .env_remove("NO_COLOR")
            .env_remove("CLICOLOR_FORCE")
            .env_remove("FORCE_COLOR")
            .env("TERM", "xterm-256color")
            .env("CHXCHX_TECH_HOME", &self.home)
            .env("PATH", &self.bin);
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().expect("run the Rust CLI")
    }
}

#[test]
fn output_is_plain_when_piped_and_colored_only_when_a_terminal_asks_for_it() {
    let sandbox = Sandbox::new();
    assert_success(&sandbox.cli(command(&["init", "--apply"], &sandbox.project)));

    // Piped (what scripts and these tests see): no escape codes, the lines they always read.
    let plain = text(&sandbox.cli_with(&["status", &project(&sandbox)], &[]));
    assert!(!plain.contains('\x1b'), "{plain:?}");
    assert!(
        plain.contains("Proyecto: ") && plain.contains("Confianza local: no"),
        "{plain}"
    );

    // A terminal that wants color gets the richer layout with the same facts.
    let rich = text(&sandbox.cli_with(&["status", &project(&sandbox)], &[("CLICOLOR_FORCE", "1")]));
    assert!(rich.contains('\x1b'), "{rich:?}");
    for fact in ["Tecnología", "Confianza local", "Perfil", "Siguiente paso"] {
        assert!(rich.contains(fact), "{fact:?} missing from {rich}");
    }

    // NO_COLOR always wins, and JSON is never decorated.
    let refused = text(&sandbox.cli_with(
        &["status", &project(&sandbox)],
        &[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")],
    ));
    assert!(!refused.contains('\x1b'));
    let json = text(&sandbox.cli_with(
        &["status", "--json", &project(&sandbox)],
        &[("CLICOLOR_FORCE", "1")],
    ));
    assert!(!json.contains('\x1b') && json.contains("chxchx.project-overview"));
}

#[test]
fn previews_and_failures_keep_their_words_with_color() {
    let sandbox = Sandbox::new();
    let preview =
        text(&sandbox.cli_with(&["init", &project(&sandbox)], &[("CLICOLOR_FORCE", "1")]));
    assert!(
        preview.contains("Inicialización completa") && preview.contains("Vista previa: usa `init"),
        "{preview}"
    );
    assert!(
        !sandbox.path(".ai").exists(),
        "a colored preview still writes nothing"
    );

    // The same command with `--apply` repeats the plan and then reports what it did.
    let applied = text(&sandbox.cli_with(
        &["init", "--apply", &project(&sandbox)],
        &[("CLICOLOR_FORCE", "1")],
    ));
    assert!(applied.contains("Cambios aplicados"), "{applied}");

    let version = text(
        &sandbox
            .cli_with(&["version"], &[("CLICOLOR_FORCE", "1")])
            .clone(),
    );
    assert!(
        version.contains("Tech ") && version.contains('╗'),
        "the banner: {version}"
    );
    let plain_version = text(&sandbox.cli_with(&["version"], &[]));
    assert_eq!(
        plain_version.trim(),
        format!("ChxChx Tech {}", env!("CARGO_PKG_VERSION"))
    );

    // Errors are colored on stderr but still start with the word scripts look for.
    let bad = sandbox.cli_with(
        &[
            "skills",
            "enable",
            "no-existe",
            "--path",
            &project(&sandbox),
        ],
        &[("CLICOLOR_FORCE", "1")],
    );
    assert!(!bad.status.success());
    assert!(text(&bad).contains("Error:"), "{}", text(&bad));
}

#[cfg(unix)]
#[test]
fn a_closed_pipe_ends_the_command_quietly_instead_of_panicking() {
    use std::process::Stdio;
    let sandbox = Sandbox::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_chxchx-tech"))
        .args(["skills", "list"])
        .arg(&sandbox.project)
        .env("CHXCHX_TECH_HOME", &sandbox.home)
        .env("PATH", &sandbox.bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start the CLI");
    // `... | head -0`: nobody reads what the command prints.
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("wait for the CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
}

/// A terminal-less run on a machine with no desktop: nothing may open a window in a test.
fn headless(sandbox: &Sandbox, program: &str, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command
        .args(args)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("SSH_CONNECTION", "test")
        .env("CHXCHX_TECH_HOME", &sandbox.home)
        .env("PATH", &sandbox.bin);
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("run the CLI")
}

#[test]
fn one_command_serves_everything_and_a_pipe_gets_the_help() {
    let sandbox = Sandbox::new();
    let tech = env!("CARGO_BIN_EXE_chxchx-tech");
    let help = text(&headless(&sandbox, tech, &[], &[]));
    for word in [
        "Con un solo comando",
        "chxchx-tech ~/mi-proyecto",
        "Atajos: st",
    ] {
        assert!(help.contains(word), "{word:?} missing from {help}");
    }
    // The short name is the same program.
    let short = text(&headless(
        &sandbox,
        env!("CARGO_BIN_EXE_chxchx"),
        &["--version"],
        &[],
    ));
    assert!(short.contains(env!("CARGO_PKG_VERSION")), "{short}");
    // Short names and the singular spellings of the old CLI.
    let project = sandbox.project.display().to_string();
    for words in [
        vec!["st", &project],
        vec!["skill", "list", &project],
        vec!["pack", "list", &project],
    ] {
        let out = headless(&sandbox, tech, &words, &[]);
        assert!(out.status.success(), "{words:?}: {}", text(&out));
    }
}

#[test]
fn commands_of_the_earlier_cli_fail_with_the_place_their_function_moved_to() {
    let sandbox = Sandbox::new();
    for (words, expected) in [
        (vec!["ci", "run"], "CI/CD"),
        (vec!["deploy", "status"], "CI/CD"),
        (vec!["sync"], "projects sync"),
        (vec!["workspace", "terminal"], "Studio"),
        (vec!["projects", "prepare"], "init"),
        (vec!["agent", "chat", "claude", "--live"], "chat-state"),
        (vec!["agent"], "tui"),
    ] {
        let out = sandbox.cli(words.iter().map(OsString::from));
        assert_eq!(out.status.code(), Some(1), "{words:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("se retiró") && stderr.contains(expected),
            "{words:?}: {stderr}"
        );
    }
    let unknown = sandbox.cli(["quizas"].map(OsString::from));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("no es un comando de ChxChx"));
}

#[test]
fn chat_state_restore_and_reset_are_served_by_rust_without_a_python_cli() {
    let sandbox = Sandbox::new();
    let state_dir = sandbox.path(".ai/chat-state");
    fs::create_dir_all(&state_dir).unwrap();
    fs::write(
        state_dir.join("codex.json"),
        r#"{"schema":"chxchx.chat-state","schema_version":1,"agent":"codex","provider":"Codex","session_id":"thread-1","updated":"2026-11-17T00:00:00Z","verified":true,"messages":[{"role":"user","text":"¿En qué íbamos?"},{"role":"assistant","text":"Migrábamos el CLI."},{"role":"tool","text":"se descarta"}]}"#,
    )
    .unwrap();

    let restore = sandbox.cli([
        "agent".into(),
        "chat-state".into(),
        "codex".into(),
        "--path".into(),
        sandbox.project.as_os_str().to_owned(),
        "--restore".into(),
    ]);
    assert_success(&restore);
    let payload: serde_json::Value = serde_json::from_slice(&restore.stdout).unwrap();
    assert_eq!(payload["schema"], "chxchx.chat-state");
    assert_eq!(payload["session_id"], "thread-1");
    assert_eq!(payload["messages"].as_array().unwrap().len(), 2);
    assert!(
        payload.get("verified").is_none(),
        "verification is internal"
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(state_dir.join("codex.json")).unwrap()).unwrap();
    assert_eq!(saved["verified"], false);

    let reset = sandbox.cli([
        "agent".into(),
        "chat-state".into(),
        "codex".into(),
        "--path".into(),
        sandbox.project.as_os_str().to_owned(),
        "--reset".into(),
    ]);
    assert_success(&reset);
    let cleared: serde_json::Value = serde_json::from_slice(&reset.stdout).unwrap();
    assert_eq!(cleared["messages"], serde_json::json!([]));
    assert!(!state_dir.join("codex.json").exists());
}

#[test]
fn connectors_are_previewed_then_written_for_every_agent_and_removed_without_touching_the_rest() {
    let sandbox = Sandbox::new();
    let project = sandbox.project.display().to_string();
    let run = |words: &[&str]| {
        let mut args: Vec<OsString> = words.iter().map(OsString::from).collect();
        args.push("--path".into());
        args.push(project.clone().into());
        sandbox.cli(args)
    };

    // The catalog lists the apps people ask for.
    let listed = assert_success(&sandbox.cli([
        "connectors".into(),
        "list".into(),
        sandbox.project.clone().into_os_string(),
    ]));
    for word in ["excel", "powerbi", "postgres", "Office", "Bases de datos"] {
        assert!(listed.contains(word), "{word:?} missing from {listed}");
    }

    // A project that already has its own server in .mcp.json.
    fs::write(
        sandbox.path(".mcp.json"),
        r#"{"mcpServers": {"mio": {"command": "propio", "args": []}}}"#,
    )
    .unwrap();

    // Preview: says what would change and writes nothing.
    let preview = assert_success(&run(&[
        "connectors",
        "add",
        "excel",
        "word",
        "--client",
        "all",
    ]));
    assert!(preview.contains("añadir `excel` a .mcp.json"), "{preview}");
    assert!(preview.contains("Vista previa"), "{preview}");
    assert!(!sandbox.path("opencode.json").exists() && !sandbox.path(".codex").exists());

    // Apply: the three files, valid, with the person's own server still there and a backup of it.
    let applied = assert_success(&run(&[
        "connectors",
        "add",
        "excel",
        "word",
        "--client",
        "all",
        "--apply",
    ]));
    assert!(applied.contains("Respaldo"), "{applied}");
    // Windows runs `npx` through `cmd /c`, so the program differs by platform.
    let launcher = if cfg!(windows) { "cmd" } else { "npx" };
    let claude: serde_json::Value =
        serde_json::from_str(&read(&sandbox.path(".mcp.json"))).unwrap();
    assert!(
        claude["mcpServers"]["mio"].is_object()
            && claude["mcpServers"]["excel"]["command"] == launcher
    );
    let opencode: serde_json::Value =
        serde_json::from_str(&read(&sandbox.path("opencode.json"))).unwrap();
    assert_eq!(opencode["mcp"]["word"]["type"], "local");
    let codex: toml::Value = toml::from_str(&read(&sandbox.path(".codex/config.toml"))).unwrap();
    assert_eq!(
        codex["mcp_servers"]["excel"]["command"].as_str(),
        Some(launcher)
    );
    let backups = sandbox.home.join("backups-mcp");
    assert!(
        backups.is_dir(),
        "the file that existed was saved before it changed"
    );
    assert!(
        !sandbox.home.join("backups").exists(),
        "rollback's own backups are not touched"
    );

    // Idempotent.
    let again = assert_success(&run(&[
        "connectors",
        "add",
        "excel",
        "word",
        "--client",
        "all",
        "--apply",
    ]));
    assert!(again.contains("todo está actualizado"), "{again}");

    // Removing takes only what ChxChx added.
    assert_success(&run(&[
        "connectors",
        "remove",
        "excel",
        "--client",
        "all",
        "--apply",
    ]));
    let claude: serde_json::Value =
        serde_json::from_str(&read(&sandbox.path(".mcp.json"))).unwrap();
    assert!(
        claude["mcpServers"].get("excel").is_none() && claude["mcpServers"]["word"].is_object()
    );
    assert!(
        claude["mcpServers"]["mio"].is_object(),
        "the person's own server stays"
    );
}

#[test]
fn a_secret_never_reaches_the_project_and_custom_servers_and_mistakes_are_handled() {
    let sandbox = Sandbox::new();
    let project = sandbox.project.display().to_string();
    let run = |words: &[&str], env: &[(&str, &str)]| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_chxchx-tech"));
        command.args(words).arg("--path").arg(&project);
        command
            .env("CHXCHX_TECH_HOME", &sandbox.home)
            .env("PATH", &sandbox.bin);
        for (k, v) in env {
            command.env(k, v);
        }
        command.output().expect("run the CLI")
    };
    let out = run(
        &[
            "connectors",
            "add",
            "postgres",
            "--client",
            "claude",
            "--apply",
        ],
        &[("DATABASE_URI", "postgresql://u:SUPERSECRETA@h/db")],
    );
    assert_success(&out);
    let written = read(&sandbox.path(".mcp.json"));
    assert!(
        written.contains("${DATABASE_URI}") && !written.contains("SUPERSECRETA"),
        "{written}"
    );
    assert!(
        !text(&out).contains("SUPERSECRETA"),
        "the value is not echoed either"
    );

    // A parameter that is missing, an unknown connector and a bad custom server say why.
    let missing = run(&["connectors", "add", "sqlite", "--client", "claude"], &[]);
    assert!(
        !missing.status.success() && text(&missing).contains("db_path"),
        "{}",
        text(&missing)
    );
    let unknown = run(
        &["connectors", "add", "no-existe", "--client", "claude"],
        &[],
    );
    assert!(text(&unknown).contains("desconocido"), "{}", text(&unknown));
    let with_value = run(
        &[
            "connectors",
            "add",
            "sqlite",
            "--client",
            "opencode",
            "--param",
            "db_path=./datos.db",
            "--apply",
        ],
        &[],
    );
    assert_success(&with_value);
    assert!(read(&sandbox.path("opencode.json")).contains("./datos.db"));

    let custom = run(
        &[
            "connectors",
            "custom",
            "mi-api",
            "--command",
            "npx",
            "--arg",
            "-y",
            "--arg",
            "mi-paquete",
            "--env",
            "MI_TOKEN",
            "--client",
            "claude",
            "--apply",
        ],
        &[],
    );
    assert_success(&custom);
    let claude: serde_json::Value =
        serde_json::from_str(&read(&sandbox.path(".mcp.json"))).unwrap();
    assert_eq!(
        claude["mcpServers"]["mi-api"]["args"],
        serde_json::json!(["-y", "mi-paquete"])
    );
    assert_eq!(
        claude["mcpServers"]["mi-api"]["env"]["MI_TOKEN"],
        "${MI_TOKEN}"
    );

    // A broken file is reported, never overwritten.
    fs::write(sandbox.path("opencode.json"), "{ roto").unwrap();
    let broken = run(
        &[
            "connectors",
            "add",
            "excel",
            "--client",
            "opencode",
            "--apply",
        ],
        &[],
    );
    assert!(!broken.status.success());
    assert_eq!(read(&sandbox.path("opencode.json")), "{ roto");
}

/// Registers a project in the sandbox's catalog next to the ones already there.
#[cfg(unix)]
fn register_project(sandbox: &Sandbox, alias: &str, folder: &Path) {
    let file = sandbox.home.join("projects.json");
    let mut document: serde_json::Value = fs::read(&file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| serde_json::json!({"projects": []}));
    document["projects"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "alias": alias, "name": alias, "path": folder, "profile": "generic", "stacks": []
        }));
    fs::write(file, document.to_string()).unwrap();
}

#[cfg(unix)]
fn worker_project(sandbox: &Sandbox, name: &str) -> PathBuf {
    let folder = sandbox.root.join(name);
    fs::create_dir_all(folder.join(".ai")).unwrap();
    fs::write(
        folder.join(".ai/chxchx-tech.toml"),
        format!("version=2\n[workspace]\nname='{name}'\nauto_start=true\n[[workspace.processes]]\nid='worker'\nlabel='Worker'\ncommand=['/bin/sleep','30']\ncwd='.'\nauto_start=true\n"),
    )
    .unwrap();
    let folder = folder.canonicalize().unwrap();
    let trust = sandbox.cli(command(&["workspace", "trust", "--apply"], &folder));
    assert_success(&trust);
    folder
}

#[cfg(unix)]
#[test]
fn switching_projects_suspends_the_active_one_starts_the_target_and_remembers_it() {
    let sandbox = Sandbox::new();
    let (first, second) = (
        worker_project(&sandbox, "uno"),
        worker_project(&sandbox, "dos"),
    );
    register_project(&sandbox, "uno", &first);
    register_project(&sandbox, "dos", &second);
    let status = |folder: &Path| -> String {
        let state: serde_json::Value =
            serde_json::from_slice(&fs::read(sandbox.home.join("workspace-state.json")).unwrap())
                .unwrap();
        state["workspaces"][folder.display().to_string()]["status"]
            .as_str()
            .unwrap_or("")
            .to_owned()
    };

    assert_success(&sandbox.cli(["projects", "switch", "uno"].map(OsString::from)));
    assert_eq!(status(&first), "ACTIVE");

    let preview = sandbox.cli(["projects", "switch", "dos", "--dry-run"].map(OsString::from));
    let text = assert_success(&preview);
    assert!(text.contains("DRY RUN: workspace suspendido"), "{text}");
    assert_eq!(status(&first), "ACTIVE", "a preview changes nothing");
    let registry: serde_json::Value =
        serde_json::from_slice(&fs::read(sandbox.home.join("projects.json")).unwrap()).unwrap();
    assert_eq!(registry["last_project"], first.display().to_string());

    let switched = assert_success(&sandbox.cli(["projects", "switch", "dos"].map(OsString::from)));
    assert!(
        switched.contains("proyecto activo: dos (dos)"),
        "{switched}"
    );
    assert_eq!(status(&first), "SUSPENDED");
    assert_eq!(status(&second), "ACTIVE");
    let registry: serde_json::Value =
        serde_json::from_slice(&fs::read(sandbox.home.join("projects.json")).unwrap()).unwrap();
    assert_eq!(registry["last_project"], second.display().to_string());
    assert_eq!(
        registry["projects"].as_array().unwrap().len(),
        2,
        "the catalog is kept"
    );
    assert_success(
        &sandbox.cli(
            ["workspace", "stop"]
                .into_iter()
                .map(OsString::from)
                .chain([second.as_os_str().to_owned()]),
        ),
    );
}

#[cfg(unix)]
#[test]
fn a_switch_that_cannot_start_the_target_leaves_the_others_alone() {
    let sandbox = Sandbox::new();
    let active = worker_project(&sandbox, "activo");
    register_project(&sandbox, "activo", &active);
    let untrusted = sandbox.root.join("sin-confianza");
    fs::create_dir_all(untrusted.join(".ai")).unwrap();
    fs::write(
        untrusted.join(".ai/chxchx-tech.toml"),
        "version=2\n[workspace]\nname='x'\n",
    )
    .unwrap();
    register_project(
        &sandbox,
        "sin-confianza",
        &untrusted.canonicalize().unwrap(),
    );
    assert_success(&sandbox.cli(["projects", "switch", "activo"].map(OsString::from)));

    let refused = sandbox.cli(["projects", "switch", "sin-confianza"].map(OsString::from));

    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("no es confiable"));
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(sandbox.home.join("workspace-state.json")).unwrap())
            .unwrap();
    assert_eq!(
        state["workspaces"][active.display().to_string()]["status"],
        "ACTIVE"
    );
    let unknown = sandbox.cli(["projects", "switch", "nadie"].map(OsString::from));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("alias"));
    assert_success(
        &sandbox.cli(
            ["workspace", "stop"]
                .into_iter()
                .map(OsString::from)
                .chain([active.as_os_str().to_owned()]),
        ),
    );
}

#[test]
fn reported_errors_are_listed_redacted_and_scoped_to_their_project() {
    let sandbox = Sandbox::new();
    let report = sandbox.cli(
        [
            "workspace",
            "report-error",
            "--operation",
            "login",
            "--message",
            "falló con token=abc123 al entrar",
        ]
        .into_iter()
        .map(OsString::from)
        .chain([sandbox.project.as_os_str().to_owned()]),
    );
    assert_success(&report);

    let all = assert_success(&sandbox.cli(["workspace", "errors"].map(OsString::from)));
    assert!(
        all.contains("login") && all.contains("token=[REDACTED]"),
        "{all}"
    );
    assert!(!all.contains("abc123"), "{all}");
    let scoped = assert_success(&sandbox.cli(command(&["workspace", "errors"], &sandbox.project)));
    assert!(scoped.contains("login"), "{scoped}");
    let other = sandbox.root.join("otro");
    fs::create_dir_all(&other).unwrap();
    let none = assert_success(&sandbox.cli(command(&["workspace", "errors"], &other)));
    assert!(none.contains("No hay errores guardados"), "{none}");
}

#[test]
fn a_failing_workspace_command_leaves_its_error_in_the_list() {
    let sandbox = Sandbox::new();
    let refused = sandbox.cli(command(&["workspace", "start"], &sandbox.project));
    assert!(
        !refused.status.success(),
        "an untrusted project cannot start"
    );

    let listed = assert_success(&sandbox.cli(["workspace", "errors"].map(OsString::from)));

    assert!(
        listed.contains("workspace start") && listed.contains("no es confiable"),
        "{listed}"
    );
}
