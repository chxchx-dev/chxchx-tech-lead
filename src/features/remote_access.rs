//! Remote access through the person's own Tailscale, as one set of steps shared by the command
//! line and Studio: look at how things are, preview what enabling would do, apply it, and undo it.
//! Nothing here changes anything until `enable`/`disable` is called with a preview the person has
//! seen; the node is never published on the Internet (the integration refuses Funnel outright).

use crate::features::{connection, node_state};
use crate::integrations::tailscale::{self as ts, Disable, Machine, Plan, Reach};
use node_state::RemoteConfig;
use std::path::Path;

/// How things are now.
#[derive(Clone, Debug)]
pub struct Overview {
    /// Where the Tailscale CLI is, or why it is not found.
    pub cli: Result<std::path::PathBuf, String>,
    /// This machine on the tailnet, or why it cannot be read (not signed in, not running…).
    pub machine: Result<Machine, String>,
    /// What is saved: the name and port the node answers to.
    pub remote: Option<RemoteConfig>,
    /// The port the node is running on, when it is running.
    pub node_port: Option<u16>,
    /// Whether what Tailscale serves reaches the node (only when remote access is on).
    pub reach: Option<Reach>,
    /// The saved port is published on the Internet (Funnel), which ChxChx never does.
    pub funnel: bool,
}

pub const NO_TAILSCALE: &str = "No encuentro Tailscale. Instálalo desde https://tailscale.com/download \
    e inicia sesión (`sudo tailscale up`), o pon su ruta en Conexión.";

pub fn locate() -> Result<std::path::PathBuf, String> {
    ts::locate().ok_or_else(|| NO_TAILSCALE.to_owned())
}

pub fn overview(dir: &Path) -> Overview {
    overview_with(dir, locate())
}

pub fn overview_with(dir: &Path, cli: Result<std::path::PathBuf, String>) -> Overview {
    let remote = node_state::read_remote(dir);
    let node_port = node_state::read_info(dir).map(|info| info.port);
    let (machine, reach, funnel) = match &cli {
        Err(problem) => (Err(problem.clone()), None, false),
        Ok(path) => {
            let machine = ts::machine(path);
            let config = remote
                .as_ref()
                .and_then(|remote| ts::serve_config(path).ok().map(|config| (remote, config)));
            let reach = config
                .as_ref()
                .map(|(remote, config)| ts::reach(config, remote.https_port, node_port));
            let funnel = config
                .as_ref()
                .is_some_and(|(remote, config)| config.funnel.contains(&remote.https_port));
            (machine, reach, funnel)
        }
    };
    Overview {
        cli,
        machine,
        remote,
        node_port,
        reach,
        funnel,
    }
}

/// What enabling on `https_port` would do. The node's port is the given one, the one it is running
/// on, or the one chosen in Conexión.
pub fn preview_enable(
    dir: &Path,
    tailscale: &Path,
    node_port: Option<u16>,
    https_port: u16,
) -> Result<Plan, String> {
    let port = node_port
        .or_else(|| node_state::read_info(dir).map(|info| info.port))
        .unwrap_or_else(connection::node_port);
    ts::plan_enable(tailscale, port, https_port)
}

/// Applies a previewed plan and remembers the name and port, so the node answers to them.
pub fn enable(dir: &Path, tailscale: &Path, plan: &Plan) -> Result<RemoteConfig, String> {
    ts::apply_enable(tailscale, plan)?;
    let config = RemoteConfig::new(&plan.host, plan.https_port)?;
    node_state::write_remote(dir, &config)?;
    Ok(config)
}

/// What disabling would do, or `None` when remote access is not on.
pub fn preview_disable(
    dir: &Path,
    tailscale: &Path,
) -> Result<Option<(RemoteConfig, Disable)>, String> {
    let Some(remote) = node_state::read_remote(dir) else {
        return Ok(None);
    };
    let config = ts::serve_config(tailscale)?;
    let plan = ts::plan_disable(&config, remote.https_port);
    Ok(Some((remote, plan)))
}

/// First stops answering to the name (so the safe state holds even if the next step fails), then
/// removes what ChxChx put in Tailscale.
pub fn disable(dir: &Path, tailscale: &Path, plan: Disable) -> Result<(), String> {
    node_state::remove_remote(dir);
    if let Disable::Remove { https_port } = plan {
        ts::apply_disable(tailscale, https_port)?;
    }
    Ok(())
}

/// The state in words, one line each, for a person looking at it (Studio).
pub fn describe(overview: &Overview) -> Vec<String> {
    let mut lines = Vec::new();
    match &overview.cli {
        Err(problem) => {
            lines.push(problem.clone());
            return lines;
        }
        Ok(path) => lines.push(format!("Tailscale: {}", path.display())),
    }
    match &overview.machine {
        Err(problem) => {
            lines.push(format!("Conexión: {problem}"));
            return lines;
        }
        Ok(machine) => lines.push(format!(
            "Esta máquina: {} · HTTPS {}",
            machine.host,
            if machine.https_ready {
                "disponible"
            } else {
                "sin activar en la tailnet"
            }
        )),
    }
    let Some(remote) = &overview.remote else {
        lines.push("Acceso desde fuera: apagado.".into());
        return lines;
    };
    lines.push(format!("Acceso desde fuera: activado → {}", remote.url()));
    if overview.funnel {
        lines.push(format!(
            "⚠ El puerto {} está publicado en Internet (Funnel). Desactívalo: `tailscale funnel --https={} off`.",
            remote.https_port, remote.https_port
        ));
    }
    lines.push(match &overview.reach {
        Some(Reach::Serving(_)) => "Tailscale sirve el nodo ✓".to_owned(),
        Some(Reach::WrongPort { proxy, node_port }) => format!(
            "⚠ Tailscale apunta a {proxy}, pero el nodo está en el puerto {node_port}: el celular vería un 502. Vuelve a activar."
        ),
        Some(Reach::NodeDown(proxy)) => format!("Tailscale apunta a {proxy}; el nodo no está en marcha."),
        Some(Reach::Other) => format!("⚠ El puerto {} sirve otra cosa.", remote.https_port),
        Some(Reach::NotServed) | None => {
            "⚠ Tailscale ya no sirve ese puerto: vuelve a activar.".to_owned()
        }
    });
    lines
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A shell script standing in for `tailscale` that keeps the serve table in a file.
    fn fake(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("chxchx-remote-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("tailscale");
        std::fs::write(
            &script,
            format!(
                r#"#!/bin/sh
state="{dir}/serve.json"
case "$1 $2" in
  "status --json") echo '{{"BackendState":"Running","Self":{{"DNSName":"fedora.tail33e808.ts.net."}},"CertDomains":["fedora.tail33e808.ts.net"]}}' ;;
  "serve status") [ -f "$state" ] && cat "$state" || echo '{{}}' ;;
  "serve --bg")
     port="${{3#--https=}}"
     echo "{{\"Web\":{{\"fedora.tail33e808.ts.net:$port\":{{\"Handlers\":{{\"/\":{{\"Proxy\":\"$4\"}}}}}}}}}}" > "$state" ;;
  "serve --https="*) rm -f "$state" ;;
esac
"#,
                dir = dir.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, script)
    }

    #[test]
    fn previewing_changes_nothing_and_enabling_then_disabling_goes_there_and_back() {
        let (dir, script) = fake("cycle");
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();

        let plan = preview_enable(&home, &script, Some(47821), 8443).unwrap();
        assert_eq!(plan.url(), "https://fedora.tail33e808.ts.net:8443/");
        assert!(
            node_state::read_remote(&home).is_none(),
            "la vista previa no guarda nada"
        );
        assert!(!dir.join("serve.json").exists(), "ni toca Tailscale");

        let remote = enable(&home, &script, &plan).unwrap();
        assert_eq!(remote.https_port, 8443);
        assert_eq!(node_state::read_remote(&home).unwrap(), remote);

        let (saved, undo) = preview_disable(&home, &script).unwrap().unwrap();
        assert_eq!(saved, remote);
        assert_eq!(undo, Disable::Remove { https_port: 8443 });
        disable(&home, &script, undo).unwrap();
        assert!(node_state::read_remote(&home).is_none());
        assert!(!dir.join("serve.json").exists());
        assert!(
            preview_disable(&home, &script).unwrap().is_none(),
            "ya apagado"
        );
    }

    #[test]
    fn the_description_says_what_is_wrong_in_words() {
        let machine = Ok(Machine {
            host: "fedora.tail33e808.ts.net".into(),
            https_ready: true,
        });
        let remote = RemoteConfig::new("fedora.tail33e808.ts.net", 443).unwrap();
        let base = Overview {
            cli: Ok("/usr/bin/tailscale".into()),
            machine,
            remote: Some(remote),
            node_port: Some(47821),
            reach: Some(Reach::WrongPort {
                proxy: "http://127.0.0.1:1".into(),
                node_port: 47821,
            }),
            funnel: false,
        };
        let text = describe(&base).join("\n");
        assert!(text.contains("502") && text.contains("activado"), "{text}");
        let off = Overview {
            remote: None,
            reach: None,
            ..base.clone()
        };
        assert!(describe(&off).join("\n").contains("apagado"));
        let missing = Overview {
            cli: Err(NO_TAILSCALE.into()),
            ..base
        };
        assert_eq!(describe(&missing), vec![NO_TAILSCALE.to_owned()]);
    }
}
