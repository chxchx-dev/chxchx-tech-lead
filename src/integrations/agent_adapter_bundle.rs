//! The agent service, carried inside the binary (ADR 0017, H5).
//!
//! `services/agent-adapter` only uses Python's standard library, so its sources travel inside the
//! program and are unpacked on first use into the application's own folder, with a small launcher
//! that runs them with the user's Python (3.11 or newer). No `uv`, no checkout, no manual step.
//! The unpacked copy is named after a hash of its contents, so a new version never reuses an old one.

use crate::integrations::executable_search::{agent_path, find_in};
use std::path::{Path, PathBuf};
use std::process::Command;

const FILES: &[(&str, &str)] = &[
    (
        "__init__.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/__init__.py"),
    ),
    (
        "__main__.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/__main__.py"),
    ),
    (
        "cli.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/cli.py"),
    ),
    (
        "codex_live.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/codex_live.py"),
    ),
    (
        "contract.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/contract.py"),
    ),
    (
        "opencode_events.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/opencode_events.py"),
    ),
    (
        "opencode_live.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/opencode_live.py"),
    ),
    (
        "opencode_live2.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/opencode_live2.py"),
    ),
    (
        "opencode_stream.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/opencode_stream.py"),
    ),
    (
        "permissions.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/permissions.py"),
    ),
    (
        "process.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/process.py"),
    ),
    (
        "runner.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/runner.py"),
    ),
    (
        "usage.py",
        include_str!("../../services/agent-adapter/src/chxchx_agent_adapter/usage.py"),
    ),
];

fn digest() -> String {
    // FNV-1a: this only names a folder, it is not a security measure.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, text) in FILES {
        for byte in name.bytes().chain(text.bytes()) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Name of the folder this build unpacks into.
pub fn current_name() -> String {
    digest()
}

/// A Python 3.11+ on the search path, or why there is none.
pub fn python() -> Result<PathBuf, String> {
    let path = agent_path();
    let names: &[&str] = if cfg!(windows) {
        &["python", "python3"]
    } else {
        &["python3", "python"]
    };
    for name in names {
        let Some(found) = find_in(name, &path) else {
            continue;
        };
        let ok = Command::new(&found)
            .args([
                "-c",
                "import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)",
            ])
            .output()
            .is_ok_and(|output| output.status.success());
        if ok {
            return Ok(found);
        }
    }
    Err("Codex y OpenCode necesitan Python 3.11 o más nuevo, y no lo encuentro. Instálalo \
         (por ejemplo `sudo dnf install python3` o `sudo apt install python3`) y vuelve a intentarlo."
        .to_owned())
}

fn launcher_text(python: &Path, source: &Path) -> String {
    if cfg!(windows) {
        format!(
            "@echo off\r\nset PYTHONPATH={}\r\n\"{}\" -m chxchx_agent_adapter %*\r\n",
            source.display(),
            python.display()
        )
    } else {
        // Single quotes cannot be escaped inside single quotes: close, escape, reopen.
        let quote =
            |path: &Path| format!("'{}'", path.display().to_string().replace('\'', "'\\''"));
        format!(
            "#!/bin/sh\nPYTHONPATH={} exec {} -m chxchx_agent_adapter \"$@\"\n",
            quote(source),
            quote(python)
        )
    }
}

/// Unpacks the service under `base` and returns its launcher. Idempotent.
pub fn unpack(base: &Path, python: &Path) -> Result<PathBuf, String> {
    let root = base.join("agent-adapter").join(digest());
    let source = root.join("src");
    let package = source.join("chxchx_agent_adapter");
    std::fs::create_dir_all(&package).map_err(|error| format!("{}: {error}", package.display()))?;
    for (name, text) in FILES {
        let target = package.join(name);
        if std::fs::read_to_string(&target).ok().as_deref() != Some(text) {
            std::fs::write(&target, text)
                .map_err(|error| format!("{}: {error}", target.display()))?;
        }
    }
    let launcher = root.join(if cfg!(windows) {
        "chxchx-agent-adapter.cmd"
    } else {
        "chxchx-agent-adapter"
    });
    let text = launcher_text(python, &source);
    if std::fs::read_to_string(&launcher).ok().as_deref() != Some(text.as_str()) {
        std::fs::write(&launcher, text)
            .map_err(|error| format!("{}: {error}", launcher.display()))?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("{}: {error}", launcher.display()))?;
    }
    Ok(launcher)
}

/// The embedded service, ready to run: unpacked into the application's folder.
pub fn ensure() -> Result<PathBuf, String> {
    let base = crate::features::project_profiles::application_home()
        .ok_or("No se pudo resolver la carpeta de ChxChx.")?;
    unpack(&base, &python()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_python_file_of_the_service_is_embedded() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("services/agent-adapter/src/chxchx_agent_adapter");
        let mut on_disk: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name.ends_with(".py"))
            .collect();
        on_disk.sort();
        let embedded: Vec<&str> = FILES.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            on_disk, embedded,
            "a service file was added or removed: update FILES"
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_unpacked_service_answers_the_contract_and_unpacking_twice_is_harmless() {
        let Ok(python) = python() else {
            return; // no suitable Python on this machine
        };
        let base = std::env::temp_dir().join(format!("chxchx-bundle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let launcher = unpack(&base, &python).unwrap();
        assert_eq!(unpack(&base, &python).unwrap(), launcher);
        let described = crate::integrations::agent_adapter::describe(&launcher).unwrap();
        assert!(described.providers.iter().any(|p| p == "Codex"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_path_with_a_quote_cannot_break_the_launcher() {
        #[cfg(unix)]
        {
            let text = launcher_text(
                Path::new("/usr/bin/python3"),
                Path::new("/tmp/it's here/src"),
            );
            assert!(text.contains("'/tmp/it'\\''s here/src'"), "{text}");
        }
    }
}
