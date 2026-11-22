use super::*;
use std::fs;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-tools-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_bundle(root: &Path, id: &str) {
    let manifest = format!(
        "schema_version=1\n\
         bundle_id={id}\n\
         platform=linux-x86_64\n\
         uv=uv\n\
         python.basic-memory=python/python312\n\
         python.serena-agent=python/python313\n\
         version.basic-memory=1.0.0\n\
         lock.basic-memory=locks/basic_memory.txt\n\
         wheelhouse.basic-memory=wheels/basic_memory\n\
         version.serena-agent=2.1.0\n\
         lock.serena-agent=locks/serena_agent.txt\n\
         wheelhouse.serena-agent=wheels/serena_agent\n"
    );
    fs::write(root.join("bundle-manifest.txt"), manifest).unwrap();
    fs::create_dir_all(root.join("python")).unwrap();
    fs::create_dir_all(root.join("locks")).unwrap();
    for wheelhouse in ["wheels/basic_memory", "wheels/serena_agent"] {
        fs::create_dir_all(root.join(wheelhouse)).unwrap();
        fs::write(root.join(wheelhouse).join("fixture.whl"), b"fixture wheel").unwrap();
    }
    fs::write(root.join("uv"), b"uv fixture").unwrap();
    fs::write(root.join("python/python312"), b"python 3.12").unwrap();
    fs::write(root.join("python/python313"), b"python 3.13").unwrap();
    fs::write(
        root.join("locks/basic_memory.txt"),
        b"basic-memory==1.0.0\n",
    )
    .unwrap();
    fs::write(
        root.join("locks/serena_agent.txt"),
        b"serena-agent==2.1.0\n",
    )
    .unwrap();
}

#[cfg(unix)]
fn fake_uv(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("uv");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn both_tools_are_known_by_name() {
    assert_eq!(
        find_tool("basic-memory").unwrap().commands,
        ["basic-memory", "bm"]
    );
    assert_eq!(find_tool("serena").unwrap().commands, ["serena"]);
    assert!(find_tool("otra").is_none());
    assert!(install("otra", true).unwrap_err().contains("desconocida"));
}

#[test]
fn a_tool_already_on_disk_is_left_alone_whatever_its_command_is_called() {
    let bin = scratch("present");
    fs::write(bin.join("bm"), "").unwrap();
    assert_eq!(
        install_with(find_tool("basic-memory").unwrap(), None, &bin, false),
        Ok(Outcome::AlreadyInstalled)
    );
    fs::write(bin.join("serena.exe"), "").unwrap();
    assert_eq!(
        install_with(find_tool("serena").unwrap(), None, &bin, true),
        Ok(Outcome::AlreadyInstalled)
    );
}

#[test]
fn a_preview_shows_the_uv_command_and_changes_nothing() {
    let bin = scratch("preview");
    let uv = Path::new("/opt/uv");
    let outcome = install_with(find_tool("serena").unwrap(), Some(uv), &bin, true).unwrap();
    assert_eq!(
        outcome,
        Outcome::Preview(
            ["/opt/uv", "tool", "install", "-p", "3.13", "serena-agent"]
                .map(String::from)
                .to_vec()
        )
    );
    let without_uv = install_with(find_tool("basic-memory").unwrap(), None, &bin, true).unwrap();
    assert!(
        matches!(&without_uv, Outcome::Preview(command) if command[0] == "uv" && command.contains(&"--prerelease".to_owned()))
    );
    assert!(fs::read_dir(&bin).unwrap().next().is_none());
}

#[test]
fn without_uv_a_real_install_says_what_is_missing() {
    let bin = scratch("no-uv");
    let error = install_with(find_tool("serena").unwrap(), None, &bin, false).unwrap_err();
    assert!(error.contains("uv no está instalado"), "{error}");
}

#[cfg(unix)]
#[test]
fn uv_runs_with_the_tool_directory_and_a_failure_is_reported() {
    let dir = scratch("run");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let uv = fake_uv(
        &dir,
        &format!(
            "echo \"$*|$UV_TOOL_BIN_DIR\" > \"{}/call\"\ntouch \"$UV_TOOL_BIN_DIR/serena\"",
            dir.display()
        ),
    );

    assert_eq!(
        install_with(find_tool("serena").unwrap(), Some(&uv), &bin, false),
        Ok(Outcome::Installed)
    );
    assert!(is_installed(find_tool("serena").unwrap(), &bin));
    let call = fs::read_to_string(dir.join("call")).unwrap();
    assert_eq!(
        call.trim(),
        format!("tool install -p 3.13 serena-agent|{}", bin.display())
    );

    let failing = fake_uv(&scratch("fail"), "exit 7");
    let error = install_with(
        find_tool("basic-memory").unwrap(),
        Some(&failing),
        &bin,
        false,
    )
    .unwrap_err();
    assert!(
        error.contains("basic-memory") && error.contains('7'),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn a_bundle_update_replaces_old_shims_and_uses_only_persistent_offline_files() {
    use std::os::unix::fs::PermissionsExt;

    let root = scratch("bundle-update");
    let source = root.join("download");
    let data_root = root.join("user-data");
    let tool_bin = root.join("tools");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&tool_bin).unwrap();
    write_bundle(&source, "fixture-1234567890abcdef");

    let uv = root.join("uv");
    let captured = root.join("uv-arguments.txt");
    let script = format!(
        "#!/bin/sh\n\
         printf '%s\\n' \"$*\" > '{}'\n\
         package=''\n\
         for argument in \"$@\"; do package=\"$argument\"; done\n\
         case \"$package\" in\n\
           basic-memory==1.0.0) commands='basic-memory bm' ;;\n\
           serena-agent==2.1.0) commands='serena' ;;\n\
           *) exit 9 ;;\n\
         esac\n\
         for command in $commands; do\n\
           printf '#!/bin/sh\\necho updated\\n' > \"$UV_TOOL_BIN_DIR/$command\"\n\
           chmod +x \"$UV_TOOL_BIN_DIR/$command\"\n\
         done\n",
        captured.display()
    );
    fs::write(&uv, script).unwrap();
    fs::set_permissions(&uv, fs::Permissions::from_mode(0o755)).unwrap();
    fs::copy(&uv, source.join("uv")).unwrap();
    for command in ["basic-memory", "bm"] {
        fs::write(tool_bin.join(command), b"old shim").unwrap();
    }

    let outcome = install_from_bundle_at(
        find_tool("basic-memory").unwrap(),
        &source,
        &tool_bin,
        Some(&data_root),
        false,
    )
    .unwrap();

    assert_eq!(outcome, Outcome::Installed);
    let persisted = data_root.join("fixture-1234567890abcdef");
    assert!(persisted.join("bundle-manifest.txt").is_file());
    assert!(persisted.join("python/python312").is_file());
    assert_eq!(
        fs::read_to_string(tool_bin.join("bm")).unwrap(),
        "#!/bin/sh\necho updated\n"
    );
    let arguments = fs::read_to_string(captured).unwrap();
    for expected in [
        "--force".to_owned(),
        "--offline".to_owned(),
        "--no-index".to_owned(),
        "--no-python-downloads".to_owned(),
        persisted.join("wheels/basic_memory").display().to_string(),
        persisted
            .join("locks/basic_memory.txt")
            .display()
            .to_string(),
        persisted.join("python/python312").display().to_string(),
        "basic-memory==1.0.0".to_owned(),
    ] {
        assert!(
            arguments.contains(&expected),
            "missing {expected} in {arguments}"
        );
    }
    assert!(
        !arguments.contains("--prerelease"),
        "the pinned stable bundle must not enable prerelease resolution: {arguments}"
    );
}

#[test]
fn a_bundle_preview_is_offline_and_does_not_persist_the_bundle() {
    let root = scratch("bundle-preview");
    let source = root.join("download");
    let data_root = root.join("user-data");
    let tool_bin = root.join("tools");
    fs::create_dir_all(&source).unwrap();
    write_bundle(&source, "fixture-1234567890abcdef");

    let outcome =
        install_from_bundle_at(find_tool("serena").unwrap(), &source, &tool_bin, None, true)
            .unwrap();

    assert!(matches!(
        outcome,
        Outcome::Preview(command)
            if command.contains(&"--offline".to_owned())
                && command.contains(&"--force".to_owned())
                && command.contains(&"serena-agent==2.1.0".to_owned())
    ));
    assert!(!data_root.exists());
    assert!(!tool_bin.exists());
}
