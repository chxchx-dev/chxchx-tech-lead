//! Starts the local node when the person logs in (ADR 0017, H5), as a service of *their own
//! account*: a systemd user unit, a macOS LaunchAgent or a Windows scheduled task. Never root or
//! administrator, never a system-wide file.
//!
//! The plan (files to write, commands to run) is computed by pure functions, so it can be shown
//! before anything changes and tested on any system. `apply` is idempotent: identical files are
//! left alone, a different one is saved as `.bak` first.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const NAME: &str = "chxchx-node";
const LABEL: &str = "tech.chxchx.node";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Macos,
    Windows,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// Everything the service depends on, as plain values.
#[derive(Clone, Debug)]
pub struct Setup {
    pub platform: Platform,
    pub exe: PathBuf,
    pub port: u16,
    /// The user's home directory.
    pub home: PathBuf,
    /// `CHXCHX_TECH_HOME` when it is set, so the service sees the same state as the person.
    pub tech_home: Option<PathBuf>,
    pub uid: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The file the service lives in, with its contents. Windows has none.
    pub file: Option<(PathBuf, String)>,
    pub install: Vec<Vec<String>>,
    /// Run before removing the file.
    pub uninstall: Vec<Vec<String>>,
    pub notes: Vec<String>,
}

fn words(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

fn systemd_quote(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    format!("\"{escaped}\"")
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn plan(setup: &Setup) -> Plan {
    let exe = setup.exe.display().to_string();
    let port = setup.port.to_string();
    match setup.platform {
        Platform::Linux => {
            let mut unit = format!(
                "[Unit]\nDescription=ChxChx Tech Lead: nodo local (solo 127.0.0.1)\n\n[Service]\n\
                 ExecStart={} node start --port {port}\nRestart=on-failure\nRestartSec=5\n\
                 NoNewPrivileges=true\n",
                systemd_quote(&exe)
            );
            if let Some(tech) = &setup.tech_home {
                unit.push_str(&format!(
                    "Environment={}\n",
                    systemd_quote(&format!("CHXCHX_TECH_HOME={}", tech.display()))
                ));
            }
            unit.push_str("\n[Install]\nWantedBy=default.target\n");
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| setup.home.join(".config"));
            Plan {
                file: Some((config.join(format!("systemd/user/{NAME}.service")), unit)),
                install: vec![
                    words(&["systemctl", "--user", "daemon-reload"]),
                    words(&["systemctl", "--user", "enable", "--now", NAME]),
                ],
                uninstall: vec![words(&["systemctl", "--user", "disable", "--now", NAME])],
                notes: vec![
                    "Arranca al iniciar tu sesión. Para que también corra sin sesión abierta puedes \
                     ejecutar tú mismo `loginctl enable-linger` (no se hace solo)."
                        .to_owned(),
                ],
            }
        }
        Platform::Macos => {
            let logs = setup.home.join("Library/Logs");
            let mut env = String::new();
            if let Some(tech) = &setup.tech_home {
                env = format!(
                    "  <key>EnvironmentVariables</key>\n  <dict>\n    <key>CHXCHX_TECH_HOME</key>\n    <string>{}</string>\n  </dict>\n",
                    xml(&tech.display().to_string())
                );
            }
            let plist = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key>\n  <string>{LABEL}</string>\n  <key>ProgramArguments</key>\n  <array>\n    <string>{}</string>\n    <string>node</string>\n    <string>start</string>\n    <string>--port</string>\n    <string>{port}</string>\n  </array>\n{env}  <key>RunAtLoad</key>\n  <true/>\n  <key>KeepAlive</key>\n  <dict>\n    <key>SuccessfulExit</key>\n    <false/>\n  </dict>\n  <key>StandardOutPath</key>\n  <string>{}</string>\n  <key>StandardErrorPath</key>\n  <string>{}</string>\n</dict>\n</plist>\n",
                xml(&exe),
                xml(&logs.join("chxchx-node.log").display().to_string()),
                xml(&logs.join("chxchx-node.log").display().to_string()),
            );
            let domain = format!("gui/{}", setup.uid.unwrap_or(501));
            let path = setup
                .home
                .join(format!("Library/LaunchAgents/{LABEL}.plist"));
            let file = path.display().to_string();
            Plan {
                file: Some((path, plist)),
                install: vec![vec![
                    "launchctl".into(),
                    "bootstrap".into(),
                    domain.clone(),
                    file,
                ]],
                uninstall: vec![vec![
                    "launchctl".into(),
                    "bootout".into(),
                    format!("{domain}/{LABEL}"),
                ]],
                notes: vec!["Sin verificar en macOS todavía.".to_owned()],
            }
        }
        Platform::Windows => Plan {
            file: None,
            install: vec![vec![
                "schtasks".into(),
                "/Create".into(),
                "/F".into(),
                "/TN".into(),
                NAME.into(),
                "/SC".into(),
                "ONLOGON".into(),
                "/TR".into(),
                format!(
                    "powershell -NoProfile -WindowStyle Hidden -Command \"& '{exe}' node start --port {port}\""
                ),
            ]],
            uninstall: vec![words(&["schtasks", "/Delete", "/F", "/TN", NAME])],
            notes: vec!["Sin verificar en Windows todavía.".to_owned()],
        },
    }
}

/// Runs a command of the plan. `Err` carries what the system said.
fn run(command: &[String]) -> Result<(), String> {
    let (program, args) = command.split_first().ok_or("comando vacío")?;
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("No se pudo ejecutar `{}`: {error}", command.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "`{}` falló: {}",
        command.join(" "),
        said.trim().lines().last().unwrap_or("sin detalle")
    ))
}

#[derive(Debug, PartialEq, Eq)]
pub enum Written {
    Created,
    Unchanged,
    /// The previous contents were saved here.
    Replaced(PathBuf),
}

/// Writes `contents` at `path`, saving a different existing file first.
pub fn write_file(path: &Path, contents: &str) -> Result<Written, String> {
    let existing = std::fs::read_to_string(path).ok();
    if existing.as_deref() == Some(contents) {
        return Ok(Written::Unchanged);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let backup = existing.is_some().then(|| {
        let mut name = path.as_os_str().to_owned();
        name.push(".bak");
        PathBuf::from(name)
    });
    if let Some(backup) = &backup {
        std::fs::copy(path, backup).map_err(|error| format!("{}: {error}", backup.display()))?;
    }
    std::fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(backup.map_or(Written::Created, Written::Replaced))
}

/// Writes the service and starts it. Returns what was done, one line each.
pub fn install(plan: &Plan) -> Result<Vec<String>, String> {
    let mut done = Vec::new();
    if let Some((path, contents)) = &plan.file {
        done.push(match write_file(path, contents)? {
            Written::Created => format!("Creado {}", path.display()),
            Written::Unchanged => format!("Sin cambios en {}", path.display()),
            Written::Replaced(backup) => format!(
                "Actualizado {} (copia anterior en {})",
                path.display(),
                backup.display()
            ),
        });
    }
    for command in &plan.install {
        // Loading a service that is already loaded is not an error worth stopping for.
        match run(command) {
            Ok(()) => done.push(format!("Ejecutado: {}", command.join(" "))),
            Err(error) if command.iter().any(|word| word == "bootstrap") => {
                done.push(format!("Aviso (¿ya estaba cargado?): {error}"));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(done)
}

/// Stops the service and removes what `install` created. Missing things are not errors.
pub fn uninstall(plan: &Plan) -> Result<Vec<String>, String> {
    let mut done = Vec::new();
    for command in &plan.uninstall {
        match run(command) {
            Ok(()) => done.push(format!("Ejecutado: {}", command.join(" "))),
            Err(error) => done.push(format!("Aviso (quizá no estaba instalado): {error}")),
        }
    }
    if let Some((path, _)) = &plan.file {
        match std::fs::remove_file(path) {
            Ok(()) => done.push(format!("Eliminado {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                done.push(format!("{} no existía", path.display()));
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    if plan.file.is_some() && plan.platform_reload() {
        let _ = run(&words(&["systemctl", "--user", "daemon-reload"]));
    }
    Ok(done)
}

impl Plan {
    fn platform_reload(&self) -> bool {
        self.install.first().is_some_and(|command| {
            command
                .first()
                .is_some_and(|program| program == "systemctl")
        })
    }
}

/// Whether the service is installed, and what its manager says about it.
pub fn status(plan: &Plan) -> String {
    let installed = match &plan.file {
        Some((path, _)) => path.exists(),
        None => run(&words(&["schtasks", "/Query", "/TN", NAME])).is_ok(),
    };
    if !installed {
        return "No instalado.".to_owned();
    }
    if plan.platform_reload() {
        let active = Command::new("systemctl")
            .args(["--user", "is-active", NAME])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default();
        return format!(
            "Instalado; estado según systemd: {}.",
            if active.is_empty() {
                "desconocido"
            } else {
                &active
            }
        );
    }
    "Instalado.".to_owned()
}

/// The setup of this machine and user.
pub fn current_setup(port: u16) -> Result<Setup, String> {
    let exe = super::node_process::locate()
        .ok_or("No encuentro el ejecutable `chxchx-tech` para el servicio.")?;
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("No se pudo resolver tu carpeta personal.")?;
    Ok(Setup {
        platform: Platform::current(),
        exe,
        port,
        home,
        tech_home: std::env::var_os("CHXCHX_TECH_HOME").map(PathBuf::from),
        uid: current_uid(),
    })
}

#[cfg(unix)]
fn current_uid() -> Option<u32> {
    let output = Command::new("id").arg("-u").output().ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

#[cfg(not(unix))]
fn current_uid() -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(platform: Platform) -> Setup {
        Setup {
            platform,
            exe: PathBuf::from("/opt/chx tech/chxchx-tech"),
            port: 47821,
            home: PathBuf::from("/home/ana"),
            tech_home: Some(PathBuf::from("/home/ana/.chx")),
            uid: Some(1000),
        }
    }

    #[test]
    fn the_linux_unit_is_a_user_unit_that_never_asks_for_more_privileges() {
        let plan = plan(&setup(Platform::Linux));
        let (path, unit) = plan.file.unwrap();
        assert!(path.ends_with("systemd/user/chxchx-node.service"));
        assert!(unit.contains("ExecStart=\"/opt/chx tech/chxchx-tech\" node start --port 47821"));
        assert!(unit.contains("NoNewPrivileges=true"));
        assert!(unit.contains("Environment=\"CHXCHX_TECH_HOME=/home/ana/.chx\""));
        assert!(unit.contains("WantedBy=default.target"));
        assert!(
            plan.install
                .iter()
                .all(|c| c.contains(&"--user".to_owned()))
        );
        assert!(!unit.contains("User=") && !unit.contains("sudo"));
    }

    #[test]
    fn quotes_and_percent_signs_in_a_path_cannot_break_the_unit() {
        assert_eq!(systemd_quote("a\"b%c"), "\"a\\\"b%%c\"");
    }

    #[test]
    fn the_macos_agent_restarts_only_after_a_failure() {
        let plan = plan(&setup(Platform::Macos));
        let (path, plist) = plan.file.unwrap();
        assert!(path.ends_with("Library/LaunchAgents/tech.chxchx.node.plist"));
        assert!(plist.contains("<key>SuccessfulExit</key>\n    <false/>"));
        assert_eq!(plan.install[0][..3], ["launchctl", "bootstrap", "gui/1000"]);
    }

    #[test]
    fn windows_uses_a_task_of_the_logged_in_user() {
        let plan = plan(&setup(Platform::Windows));
        assert!(plan.file.is_none());
        let command = plan.install[0].join(" ");
        assert!(command.contains("/SC ONLOGON") && !command.contains("SYSTEM"));
    }

    #[test]
    fn writing_twice_changes_nothing_and_a_different_file_is_kept_aside() {
        let dir = std::env::temp_dir().join(format!("chxchx-svc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("a/b.service");
        assert_eq!(write_file(&path, "uno").unwrap(), Written::Created);
        assert_eq!(write_file(&path, "uno").unwrap(), Written::Unchanged);
        let Written::Replaced(backup) = write_file(&path, "dos").unwrap() else {
            panic!("expected a backup");
        };
        assert_eq!(std::fs::read_to_string(backup).unwrap(), "uno");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "dos");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
