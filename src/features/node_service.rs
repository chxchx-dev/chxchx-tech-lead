//! The node's user service (systemd --user, LaunchAgent or scheduled task) as one set of steps shared
//! by the command line and Studio: how it is, what restarting it would change (the port chosen in
//! Conexión, mostly), and doing it. Nothing changes until `apply` gets a preview the person has seen;
//! running jobs are never cancelled: with any running the restart is refused.
//!
//! An installed service keeps the program and the data folder it was installed with; only the port is
//! taken from Conexión. Installing for the first time uses this program and the current environment.

use crate::features::connection;
use crate::integrations::{node_link, node_process, user_service as svc};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Overview {
    pub installed: bool,
    /// What the service manager says (`active`, `inactive`…), when it can be asked.
    pub state: Option<String>,
    /// The port written in the installed service.
    pub unit_port: Option<u16>,
    /// The port chosen in Conexión (the one a restart would use).
    pub wanted_port: u16,
    /// The port the node answers on right now.
    pub running_port: Option<u16>,
    pub running_jobs: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Preview {
    pub installing: bool,
    pub plan: svc::Plan,
    pub port: u16,
    pub from_port: Option<u16>,
    pub lines: Vec<String>,
}

fn unit_file(plan: &svc::Plan) -> Option<&std::path::Path> {
    plan.file.as_ref().map(|(path, _)| path.as_path())
}

/// The plan for today: the installed service's program and folder, the saved port.
fn current_plan() -> Result<(svc::Plan, svc::Existing), String> {
    let port = connection::node_port();
    let mut setup = svc::current_setup(port)?;
    let first = svc::plan(&setup);
    let existing = unit_file(&first)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|text| svc::parse_unit(&text))
        .unwrap_or_default();
    if let Some(exe) = existing.exe.clone().filter(|exe| exe.is_file()) {
        setup.exe = exe;
    }
    if existing.tech_home.is_some() {
        setup.tech_home = existing.tech_home.clone();
    }
    Ok((svc::plan(&setup), existing))
}

pub fn overview() -> Overview {
    let (installed, state, unit_port) = match current_plan() {
        Ok((plan, existing)) => {
            let installed = svc::is_installed(&plan);
            let state = installed.then(|| svc::status(&plan)).map(|said| {
                let said = said.trim_end_matches('.');
                said.rsplit(": ").next().unwrap_or(said).to_owned()
            });
            (installed, state, existing.port)
        }
        Err(_) => (false, None, None),
    };
    let endpoint = node_link::endpoint();
    Overview {
        installed,
        state,
        unit_port,
        wanted_port: connection::node_port(),
        running_port: endpoint.as_ref().map(|e| e.port),
        running_jobs: endpoint
            .as_ref()
            .and_then(|e| node_link::status(e).ok())
            .map(|status| status.jobs.running as u32),
    }
}

/// How things are, in words.
pub fn describe(overview: &Overview) -> Vec<String> {
    let mut lines = vec![if overview.installed {
        format!(
            "Servicio instalado y {}.",
            match overview.state.as_deref() {
                Some("active") => "en marcha".to_owned(),
                Some("inactive") => "detenido".to_owned(),
                Some("failed") => "con un fallo".to_owned(),
                Some(other) => other.to_owned(),
                None => "estado desconocido".to_owned(),
            }
        )
    } else {
        "El servicio no está instalado: el nodo solo corre si lo inicias tú.".into()
    }];
    match overview.running_port {
        Some(port) => lines.push(format!(
            "El nodo responde en el puerto {port}{}.",
            overview
                .running_jobs
                .map(|jobs| format!(" · {jobs} trabajo(s) en curso"))
                .unwrap_or_default()
        )),
        None => lines.push("El nodo no está respondiendo ahora.".into()),
    }
    if let (true, Some(unit)) = (overview.installed, overview.unit_port)
        && unit != overview.wanted_port
    {
        lines.push(format!(
            "⚠ El servicio tiene el puerto {unit} y en Conexión elegiste el {}: reinícialo para aplicarlo.",
            overview.wanted_port
        ));
    }
    lines
}

/// What restarting (or installing) would do. Nothing is written.
pub fn preview() -> Result<Preview, String> {
    let (plan, existing) = current_plan()?;
    let installing = !svc::is_installed(&plan);
    let port = connection::node_port();
    let mut lines = Vec::new();
    if installing {
        lines.push(
            "Se instalará el nodo como servicio de tu usuario: arranca al iniciar sesión y se levanta solo si se cae. \
             Nunca usa root ni administrador."
                .to_owned(),
        );
    }
    if let Some((path, contents)) = &plan.file {
        let same = std::fs::read_to_string(path).ok().as_deref() == Some(contents.as_str());
        lines.push(if same {
            format!("{} ya dice lo que toca: no se reescribe.", path.display())
        } else if installing {
            format!("Se crea {}.", path.display())
        } else {
            format!(
                "Se actualiza {} (la versión anterior queda como .bak).",
                path.display()
            )
        });
    }
    lines.push(match existing.port {
        Some(from) if from != port => format!("Puerto del nodo: {from} → {port}."),
        _ => format!("Puerto del nodo: {port}."),
    });
    let moving = existing.port.is_some_and(|from| from != port);
    if moving
        && crate::features::node_state::dir()
            .and_then(|dir| crate::features::node_state::read_remote(&dir))
            .is_some()
    {
        lines.push(
            "⚠ El acceso desde fuera (Tailscale) sigue apuntando al puerto anterior: después de reiniciar, \
             pulsa «Volver a activar» en Acceso desde fuera para que apunte al nuevo."
                .into(),
        );
    }
    for command in plan.install.iter().chain(if installing {
        &[][..]
    } else {
        &plan.restart[..]
    }) {
        lines.push(format!("Comando: {}", command.join(" ")));
    }
    lines.push(
        "El nodo se detiene unos segundos y vuelve; el teléfono se reconecta solo. \
         Si hay trabajos en curso, no se hace nada."
            .into(),
    );
    Ok(Preview {
        installing,
        plan,
        port,
        from_port: existing.port,
        lines,
    })
}

/// Refuses while jobs run, writes the service, starts or restarts it and waits for the node to answer.
pub fn apply(preview: &Preview) -> Result<Vec<String>, String> {
    if let Some(endpoint) = node_link::endpoint() {
        let running = node_link::status(&endpoint)?.jobs.running;
        if running > 0 {
            return Err(format!(
                "Hay {running} trabajo(s) en curso. Espera a que terminen (o cancélalos tú) y repite."
            ));
        }
    }
    let mut done = svc::install(&preview.plan)?;
    if !preview.installing {
        done.extend(svc::restart(&preview.plan)?);
    } else if node_process::locate().is_none() {
        done.push("Aviso: no encuentro `chxchx-tech` para comprobar el arranque.".into());
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(endpoint) = node_link::endpoint()
            && endpoint.port == preview.port
        {
            done.push(format!("El nodo responde en el puerto {}.", endpoint.port));
            return Ok(done);
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Err(format!(
        "El servicio se reinició pero el nodo no responde en el puerto {} (mira `chxchx-tech node service status` y los registros).",
        preview.port
    ))
}

pub fn unit_path() -> Option<PathBuf> {
    current_plan()
        .ok()
        .and_then(|(plan, _)| unit_file(&plan).map(PathBuf::from))
}
