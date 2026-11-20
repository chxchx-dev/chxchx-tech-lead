use super::*;
use std::fs;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chxchx-tools-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
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
