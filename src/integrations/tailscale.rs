//! The adapter of the official `tailscale` CLI (ADR 0017): it lets the node be reached from the
//! person's own devices, over their private tailnet, without opening a port on the router.
//!
//! What this module does and does not do:
//! - It only runs `tailscale status` and `tailscale serve`. It never runs `tailscale funnel`
//!   (that would publish the node on the Internet), never touches the firewall, and never
//!   changes Tailscale's own settings.
//! - `serve` is configured only after a preview, and never over something else: if the HTTPS port
//!   already serves a different target, it is left alone and the person is told.
//! - Whatever Tailscale forwards about the account (`Tailscale-User-*`) is not read here.

use crate::integrations::executable_search::{agent_path, find_in};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const LIMIT: Duration = Duration::from_secs(20);
/// Where `serve` sends requests: the node listens only here.
pub fn target(node_port: u16) -> String {
    format!("http://127.0.0.1:{node_port}")
}

/// The CLI, from the `PATH` and the places each system installs it.
pub fn locate() -> Option<PathBuf> {
    if let Some(chosen) = super::executable_search::chosen("tailscale") {
        return Some(chosen);
    }
    if let Some(found) = find_in(
        if cfg!(windows) {
            "tailscale.exe"
        } else {
            "tailscale"
        },
        &agent_path(),
    ) {
        return Some(found);
    }
    [
        "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
        "/usr/local/bin/tailscale",
        "/opt/homebrew/bin/tailscale",
        r"C:\Program Files\Tailscale\tailscale.exe",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|path| path.is_file())
}

/// This machine on the tailnet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    /// MagicDNS name, without the final dot.
    pub host: String,
    /// Tailscale can issue a certificate for it (HTTPS is enabled for the tailnet).
    pub https_ready: bool,
}

/// What a port of `tailscale serve` is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Serving {
    Proxy(String),
    /// A file, text or anything else that is not a proxy.
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ServeConfig {
    pub ports: BTreeMap<u16, Serving>,
    /// Ports that `tailscale funnel` publishes on the Internet.
    pub funnel: Vec<u16>,
}

/// Whether what `tailscale serve` shows on the HTTPS port actually reaches the node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reach {
    /// Served, and to the port the node is on.
    Serving(String),
    /// Served to another port than the node's: the phone gets a 502.
    WrongPort { proxy: String, node_port: u16 },
    /// Served, but there is no node to compare with (it is not running).
    NodeDown(String),
    /// The port serves something that is not a proxy.
    Other,
    /// The port is no longer served.
    NotServed,
}

pub fn reach(config: &ServeConfig, https_port: u16, node_port: Option<u16>) -> Reach {
    match (config.ports.get(&https_port), node_port) {
        (Some(Serving::Proxy(proxy)), Some(port)) if *proxy == target(port) => {
            Reach::Serving(proxy.clone())
        }
        (Some(Serving::Proxy(proxy)), Some(port)) => Reach::WrongPort {
            proxy: proxy.clone(),
            node_port: port,
        },
        (Some(Serving::Proxy(proxy)), None) => Reach::NodeDown(proxy.clone()),
        (Some(Serving::Other), _) => Reach::Other,
        (None, _) => Reach::NotServed,
    }
}

pub fn parse_machine(json: &str) -> Result<Machine, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("Respuesta de Tailscale ilegible: {error}"))?;
    let state = value["BackendState"].as_str().unwrap_or_default();
    if state != "Running" {
        return Err(match state {
            "NeedsLogin" | "NeedsMachineAuth" => {
                "Tailscale está instalado pero falta iniciar sesión: ejecuta `sudo tailscale up`."
                    .to_owned()
            }
            "Stopped" => "Tailscale está detenido: ejecuta `sudo tailscale up`.".to_owned(),
            other => format!("Tailscale no está conectado (estado: {other:?})."),
        });
    }
    let host = value["Self"]["DNSName"]
        .as_str()
        .map(|name| name.trim_end_matches('.').to_owned())
        .filter(|name| !name.is_empty())
        .ok_or("Tailscale no dio un nombre para esta máquina (¿MagicDNS desactivado?).")?;
    let https_ready = value["CertDomains"].as_array().is_some_and(|domains| {
        domains
            .iter()
            .any(|domain| domain.as_str() == Some(host.as_str()))
    });
    Ok(Machine { host, https_ready })
}

pub fn parse_serve(json: &str) -> Result<ServeConfig, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("Respuesta de Tailscale ilegible: {error}"))?;
    let port_of = |key: &str| {
        key.rsplit(':')
            .next()
            .and_then(|port| port.parse::<u16>().ok())
    };
    let mut config = ServeConfig::default();
    for (key, site) in value["Web"].as_object().into_iter().flatten() {
        let Some(port) = port_of(key) else { continue };
        let root = &site["Handlers"]["/"];
        let serving = match root["Proxy"].as_str() {
            Some(proxy) => Serving::Proxy(proxy.to_owned()),
            None => Serving::Other,
        };
        config.ports.insert(port, serving);
    }
    // A port that forwards raw TCP (no `Web` entry) is also taken.
    for (port, _) in value["TCP"].as_object().into_iter().flatten() {
        if let Ok(port) = port.parse::<u16>() {
            config.ports.entry(port).or_insert(Serving::Other);
        }
    }
    for (key, on) in value["AllowFunnel"].as_object().into_iter().flatten() {
        if on.as_bool() == Some(true)
            && let Some(port) = port_of(key)
        {
            config.funnel.push(port);
        }
    }
    Ok(config)
}

fn run(tailscale: &Path, args: &[&str]) -> Result<String, String> {
    // The one place that starts the CLI, so the rule is checked once: never `funnel`.
    if args.iter().any(|arg| arg.eq_ignore_ascii_case("funnel")) {
        return Err("ChxChx nunca publica el nodo en Internet (Tailscale Funnel)".to_owned());
    }
    let mut process = Command::new(tailscale);
    process.args(args).env("PATH", agent_path());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        process.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let (code, stdout, stderr) =
        capture(process, LIMIT).map_err(|error| format!("No pude ejecutar tailscale: {error}"))?;
    if code == 0 {
        Ok(stdout)
    } else {
        // Tailscale explains its failures on stderr.
        let why = if stderr.trim().is_empty() {
            &stdout
        } else {
            &stderr
        };
        Err(format!(
            "tailscale {} falló ({code}): {}",
            args.first().copied().unwrap_or(""),
            why.trim()
        ))
    }
}

/// Runs the command and returns its exit code, stdout and stderr, giving up after `limit`.
fn capture(mut process: Command, limit: Duration) -> Result<(i32, String, String), String> {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::Instant;
    let mut child = process
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    // Each pipe is drained on its own thread: a full pipe would otherwise stop the child.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            String::from_utf8_lossy(&bytes).into_owned()
        })
    };
    let out = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let deadline = Instant::now() + limit;
    let code = loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => break status.code().unwrap_or(-1),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("tailscale tardó demasiado en responder".to_owned());
            }
            None => std::thread::sleep(Duration::from_millis(15)),
        }
    };
    Ok((
        code,
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    ))
}

pub fn machine(tailscale: &Path) -> Result<Machine, String> {
    parse_machine(&run(tailscale, &["status", "--json"])?)
}

pub fn serve_config(tailscale: &Path) -> Result<ServeConfig, String> {
    parse_serve(&run(tailscale, &["serve", "status", "--json"])?)
}

/// What enabling remote access would do; nothing has been changed yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub host: String,
    pub https_port: u16,
    pub target: String,
    /// The exact command that `apply` runs.
    pub command: Vec<String>,
    /// The same proxy is already in place: applying changes nothing in Tailscale.
    pub already_in_place: bool,
}

impl Plan {
    /// The address a device opens.
    pub fn url(&self) -> String {
        if self.https_port == 443 {
            format!("https://{}/", self.host)
        } else {
            format!("https://{}:{}/", self.host, self.https_port)
        }
    }
}

pub fn plan_enable(tailscale: &Path, node_port: u16, https_port: u16) -> Result<Plan, String> {
    let machine = machine(tailscale)?;
    if !machine.https_ready {
        return Err(format!(
            "Esta tailnet no tiene HTTPS activado para {}. Actívalo en el panel de Tailscale \
             (DNS → HTTPS Certificates) y vuelve a intentarlo.",
            machine.host
        ));
    }
    plan_with(
        &machine,
        &serve_config(tailscale)?,
        tailscale,
        node_port,
        https_port,
    )
}

pub fn plan_with(
    machine: &Machine,
    config: &ServeConfig,
    tailscale: &Path,
    node_port: u16,
    https_port: u16,
) -> Result<Plan, String> {
    let target = target(node_port);
    if config.funnel.contains(&https_port) {
        return Err(format!(
            "El puerto {https_port} de Tailscale está publicado en Internet (Funnel). \
             Desactívalo con `tailscale funnel --https={https_port} off` o elige otro puerto con --https-port."
        ));
    }
    let already_in_place = match config.ports.get(&https_port) {
        None => false,
        Some(Serving::Proxy(current)) if *current == target => true,
        Some(Serving::Proxy(other)) => {
            return Err(format!(
                "El puerto HTTPS {https_port} de Tailscale ya sirve {other}. No lo cambio: \
                 elige otro con --https-port (p. ej. 8443) o libéralo tú."
            ));
        }
        Some(Serving::Other) => {
            return Err(format!(
                "El puerto HTTPS {https_port} de Tailscale ya está en uso. No lo cambio: \
                 elige otro con --https-port (p. ej. 8443)."
            ));
        }
    };
    Ok(Plan {
        host: machine.host.clone(),
        https_port,
        target: target.clone(),
        command: vec![
            tailscale.display().to_string(),
            "serve".into(),
            "--bg".into(),
            format!("--https={https_port}"),
            target,
        ],
        already_in_place,
    })
}

/// Configures `tailscale serve` as planned, and checks that it took.
pub fn apply_enable(tailscale: &Path, plan: &Plan) -> Result<(), String> {
    if !plan.already_in_place {
        let https = format!("--https={}", plan.https_port);
        run(tailscale, &["serve", "--bg", &https, &plan.target])?;
    }
    match serve_config(tailscale)?.ports.get(&plan.https_port) {
        Some(Serving::Proxy(current)) if *current == plan.target => Ok(()),
        other => Err(format!(
            "Tailscale no quedó sirviendo {} en el puerto {} (ahora: {other:?})",
            plan.target, plan.https_port
        )),
    }
}

/// How far a disable would go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disable {
    /// Our proxy is there: `serve --https=N off` removes it.
    Remove { https_port: u16 },
    /// Nothing of ours is served any more.
    NothingToRemove,
    /// The port serves something that is not a local proxy: not ours to remove.
    NotOurs { https_port: u16 },
}

pub fn plan_disable(config: &ServeConfig, https_port: u16) -> Disable {
    match config.ports.get(&https_port) {
        None => Disable::NothingToRemove,
        // What ChxChx configures is always a proxy to its own loopback port.
        Some(Serving::Proxy(proxy)) if proxy.starts_with("http://127.0.0.1:") => {
            Disable::Remove { https_port }
        }
        Some(_) => Disable::NotOurs { https_port },
    }
}

pub fn apply_disable(tailscale: &Path, https_port: u16) -> Result<(), String> {
    run(
        tailscale,
        &["serve", &format!("--https={https_port}"), "off"],
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from Tailscale 1.104.7.
    const SERVE_BOTH: &str = r#"{"TCP":{"443":{"HTTPS":true},"8443":{"HTTPS":true}},
        "Web":{"fedora.tail33e808.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:47991"}}},
               "fedora.tail33e808.ts.net:8443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:47990"}}}}}"#;

    fn machine() -> Machine {
        Machine {
            host: "fedora.tail33e808.ts.net".into(),
            https_ready: true,
        }
    }

    #[test]
    fn the_machine_is_read_only_when_tailscale_is_connected() {
        let running = r#"{"BackendState":"Running","Self":{"DNSName":"fedora.tail33e808.ts.net."},
            "CertDomains":["fedora.tail33e808.ts.net"]}"#;
        assert_eq!(parse_machine(running).unwrap(), machine());
        let without_https =
            r#"{"BackendState":"Running","Self":{"DNSName":"a.t.ts.net."},"CertDomains":null}"#;
        assert!(!parse_machine(without_https).unwrap().https_ready);
        for (state, expected) in [
            ("NeedsLogin", "iniciar sesión"),
            ("Stopped", "detenido"),
            ("Starting", "Starting"),
        ] {
            let error = parse_machine(&format!(r#"{{"BackendState":"{state}"}}"#)).unwrap_err();
            assert!(error.contains(expected), "{state}: {error}");
        }
        assert!(parse_machine("not json").is_err());
    }

    #[test]
    fn serving_a_different_port_than_the_nodes_is_told_apart_from_serving_it() {
        let mut config = ServeConfig::default();
        config
            .ports
            .insert(443, Serving::Proxy("http://127.0.0.1:47821".into()));
        assert_eq!(
            reach(&config, 443, Some(47821)),
            Reach::Serving("http://127.0.0.1:47821".into())
        );
        // The node was started on another port: the phone would get a 502.
        assert_eq!(
            reach(&config, 443, Some(47900)),
            Reach::WrongPort {
                proxy: "http://127.0.0.1:47821".into(),
                node_port: 47900
            }
        );
        assert_eq!(
            reach(&config, 443, None),
            Reach::NodeDown("http://127.0.0.1:47821".into())
        );
        assert_eq!(reach(&config, 8443, Some(47821)), Reach::NotServed);
        config.ports.insert(8443, Serving::Other);
        assert_eq!(reach(&config, 8443, Some(47821)), Reach::Other);
    }

    #[test]
    fn the_serve_configuration_is_read_with_its_proxies_and_funnel() {
        let config = parse_serve(SERVE_BOTH).unwrap();
        assert_eq!(
            config.ports[&443],
            Serving::Proxy("http://127.0.0.1:47991".into())
        );
        assert_eq!(
            config.ports[&8443],
            Serving::Proxy("http://127.0.0.1:47990".into())
        );
        assert_eq!(parse_serve("{}").unwrap(), ServeConfig::default());
        let tcp_only = parse_serve(r#"{"TCP":{"22":{"TCPForward":"127.0.0.1:22"}}}"#).unwrap();
        assert_eq!(tcp_only.ports[&22], Serving::Other);
        let funnel =
            parse_serve(r#"{"AllowFunnel":{"a.t.ts.net:443":true,"a.t.ts.net:8443":false}}"#)
                .unwrap();
        assert_eq!(funnel.funnel, [443]);
    }

    #[test]
    fn enabling_is_planned_without_overwriting_what_is_already_served() {
        let tailscale = Path::new("/usr/bin/tailscale");
        let plan = plan_with(&machine(), &ServeConfig::default(), tailscale, 47821, 443).unwrap();
        assert_eq!(plan.target, "http://127.0.0.1:47821");
        assert_eq!(
            plan.command,
            [
                "/usr/bin/tailscale",
                "serve",
                "--bg",
                "--https=443",
                "http://127.0.0.1:47821"
            ]
        );
        assert_eq!(plan.url(), "https://fedora.tail33e808.ts.net/");
        assert!(!plan.already_in_place);
        assert!(!plan.command.iter().any(|arg| arg.contains("funnel")));

        let config = parse_serve(SERVE_BOTH).unwrap();
        let busy = plan_with(&machine(), &config, tailscale, 47821, 443).unwrap_err();
        assert!(busy.contains("ya sirve http://127.0.0.1:47991") && busy.contains("No lo cambio"));
        let same = plan_with(&machine(), &config, tailscale, 47990, 8443).unwrap();
        assert!(
            same.already_in_place,
            "idempotent: the same proxy is already there"
        );
        assert_eq!(same.url(), "https://fedora.tail33e808.ts.net:8443/");
        let other = parse_serve(r#"{"TCP":{"443":{"HTTPS":true}}}"#).unwrap();
        assert!(plan_with(&machine(), &other, tailscale, 47821, 443).is_err());
    }

    #[test]
    fn a_port_published_on_the_internet_is_refused() {
        let funnel =
            parse_serve(r#"{"AllowFunnel":{"fedora.tail33e808.ts.net:443":true}}"#).unwrap();
        let error = plan_with(&machine(), &funnel, Path::new("tailscale"), 47821, 443).unwrap_err();
        assert!(
            error.contains("Funnel") && error.contains("Internet"),
            "{error}"
        );
    }

    #[test]
    fn disabling_only_removes_a_local_proxy_that_chxchx_could_have_set() {
        let config = parse_serve(SERVE_BOTH).unwrap();
        assert_eq!(
            plan_disable(&config, 443),
            Disable::Remove { https_port: 443 }
        );
        assert_eq!(plan_disable(&config, 9000), Disable::NothingToRemove);
        let foreign = parse_serve(
            r#"{"Web":{"a.t.ts.net:443":{"Handlers":{"/":{"Proxy":"http://192.168.1.5:80"}}}},"TCP":{"443":{"HTTPS":true}}}"#,
        )
        .unwrap();
        assert_eq!(
            plan_disable(&foreign, 443),
            Disable::NotOurs { https_port: 443 }
        );
    }

    #[test]
    fn the_funnel_command_cannot_be_started_at_all() {
        let error = run(Path::new("/bin/true"), &["funnel", "--bg", "443"]).unwrap_err();
        assert!(error.contains("nunca publica"), "{error}");
    }

    /// A shell script standing in for `tailscale`: it keeps the serve table in a file.
    #[cfg(unix)]
    #[test]
    fn a_fake_tailscale_is_driven_through_enable_disable_and_a_failed_apply() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("chxchx-tailscale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("tailscale");
        std::fs::write(&script, format!(r#"#!/bin/sh
state="{dir}/serve.json"; log="{dir}/calls.log"
echo "$@" >> "$log"
case "$1 $2" in
  "status --json") echo '{{"BackendState":"Running","Self":{{"DNSName":"fedora.tail33e808.ts.net."}},"CertDomains":["fedora.tail33e808.ts.net"]}}' ;;
  "serve status") [ -f "$state" ] && cat "$state" || echo '{{}}' ;;
  "serve --bg")
     [ -f "{dir}/refuse" ] && {{ echo "denied" >&2; exit 1; }}
     port="${{3#--https=}}"
     echo "{{\"Web\":{{\"fedora.tail33e808.ts.net:$port\":{{\"Handlers\":{{\"/\":{{\"Proxy\":\"$4\"}}}}}}}}}}" > "$state" ;;
  "serve --https="*) rm -f "$state" ;;
esac
"#, dir = dir.display())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let plan = plan_enable(&script, 47821, 443).unwrap();
        assert!(!plan.already_in_place);
        apply_enable(&script, &plan).unwrap();
        assert!(plan_enable(&script, 47821, 443).unwrap().already_in_place);
        let calls = std::fs::read_to_string(dir.join("calls.log")).unwrap();
        assert_eq!(
            calls.matches("serve --bg").count(),
            1,
            "applying twice runs it once: {calls}"
        );
        assert!(!calls.contains("funnel"));

        let config = serve_config(&script).unwrap();
        assert_eq!(
            plan_disable(&config, 443),
            Disable::Remove { https_port: 443 }
        );
        apply_disable(&script, 443).unwrap();
        assert_eq!(
            plan_disable(&serve_config(&script).unwrap(), 443),
            Disable::NothingToRemove
        );

        std::fs::write(dir.join("refuse"), "").unwrap();
        let again = plan_enable(&script, 47821, 443).unwrap();
        assert!(
            apply_enable(&script, &again)
                .unwrap_err()
                .contains("denied")
        );
    }
}
