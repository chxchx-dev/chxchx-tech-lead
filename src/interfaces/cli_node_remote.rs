//! `chxchx-tech node remote`: reach the node from the person's own devices over Tailscale.
//!
//! Like every change in this product it previews first and only `--apply` confirms. `disable` is
//! the way back: it stops the node answering to the Tailscale name and removes the `serve` proxy,
//! but only a local proxy that ChxChx could have set.
use crate::features::node_state::{self, RemoteConfig};
use crate::integrations::tailscale::{self, Disable};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(in crate::interfaces::cli) enum RemoteCommand {
    /// Dice si Tailscale está listo y si el nodo se alcanza desde tus dispositivos.
    Status,
    /// Hace que el nodo responda en tu red privada de Tailscale (`tailscale serve`).
    ///
    /// Sin `--apply` solo muestra la vista previa. Nunca usa Funnel: el nodo no sale a Internet.
    Enable {
        /// Puerto HTTPS de Tailscale que se usará.
        #[arg(long, default_value_t = crate::features::connection::load().https_port())]
        https_port: u16,
        /// Puerto local del nodo; por defecto el del nodo en marcha o el fijo.
        #[arg(long)]
        node_port: Option<u16>,
        /// Aplica el cambio mostrado.
        #[arg(long)]
        apply: bool,
    },
    /// Deja de servir el nodo por Tailscale y de aceptar su nombre.
    Disable {
        #[arg(long)]
        apply: bool,
    },
}

pub(in crate::interfaces::cli) fn run(command: RemoteCommand) -> Result<(), String> {
    match command {
        RemoteCommand::Status => status(),
        RemoteCommand::Enable {
            https_port,
            node_port,
            apply,
        } => enable(https_port, node_port, apply),
        RemoteCommand::Disable { apply } => disable(apply),
    }
}

/// What the person is looking at: a preview, or the change that is about to be made.
fn heading(apply: bool) -> &'static str {
    if apply {
        "Se va a aplicar:"
    } else {
        "Vista previa (no se cambia nada sin --apply):"
    }
}

fn cli() -> Result<PathBuf, String> {
    tailscale::locate().ok_or_else(|| {
        "No encuentro Tailscale. Instálalo desde https://tailscale.com/download e inicia sesión \
         (`sudo tailscale up`)."
            .to_owned()
    })
}

fn dir() -> Result<PathBuf, String> {
    node_state::dir().ok_or_else(|| "No se pudo resolver CHXCHX_TECH_HOME".to_owned())
}

fn status() -> Result<(), String> {
    let dir = dir()?;
    let remote = node_state::read_remote(&dir);
    let info = node_state::read_info(&dir);
    match &info {
        Some(info) => println!("Nodo: en marcha en 127.0.0.1:{}", info.port),
        None => println!("Nodo: no está en marcha (`chxchx-tech node start`)"),
    }
    let tailscale = match cli() {
        Ok(path) => {
            println!("Tailscale: {}", path.display());
            path
        }
        Err(problem) => {
            println!("Tailscale: {problem}");
            return Ok(());
        }
    };
    match tailscale::machine(&tailscale) {
        Ok(machine) => println!(
            "Esta máquina: {} · HTTPS {}",
            machine.host,
            if machine.https_ready {
                "disponible"
            } else {
                "sin activar en la tailnet"
            }
        ),
        Err(problem) => {
            println!("Conexión: {problem}");
            return Ok(());
        }
    }
    let Some(remote) = remote else {
        println!("Acceso remoto: no activado (`chxchx-tech node remote enable`)");
        return Ok(());
    };
    println!("Acceso remoto: activado → {}", remote.url());
    let config = tailscale::serve_config(&tailscale)?;
    if config.funnel.contains(&remote.https_port) {
        println!(
            "⚠ El puerto {} está publicado en Internet (Funnel). Desactívalo: `tailscale funnel --https={} off`.",
            remote.https_port, remote.https_port
        );
    }
    match tailscale::reach(&config, remote.https_port, info.map(|info| info.port)) {
        tailscale::Reach::Serving(proxy) => println!("Tailscale sirve el nodo ({proxy}) ✓"),
        tailscale::Reach::WrongPort { proxy, node_port } => println!(
            "⚠ Tailscale apunta a {proxy}, pero el nodo está en el puerto {node_port}. Vuelve a ejecutar `node remote enable --apply`."
        ),
        tailscale::Reach::NodeDown(proxy) => println!("Tailscale apunta a {proxy}"),
        tailscale::Reach::Other => println!("⚠ El puerto {} sirve otra cosa.", remote.https_port),
        tailscale::Reach::NotServed => println!(
            "⚠ Tailscale ya no sirve ese puerto: vuelve a ejecutar `node remote enable --apply`."
        ),
    }
    Ok(())
}

fn enable(https_port: u16, node_port: Option<u16>, apply: bool) -> Result<(), String> {
    let dir = dir()?;
    let tailscale = cli()?;
    let running = node_state::read_info(&dir);
    let port = node_port
        .or(running.as_ref().map(|info| info.port))
        .unwrap_or(crate::interfaces::node::DEFAULT_PORT);
    let plan = tailscale::plan_enable(&tailscale, port, https_port)?;

    println!("{}", heading(apply));
    println!("  Equipo:     {}", plan.host);
    println!(
        "  Dirección:  {}   (solo dentro de tu tailnet, nunca en Internet)",
        plan.url()
    );
    if plan.already_in_place {
        println!(
            "  Tailscale:  ya sirve {} en el puerto {}; no hay que tocarlo",
            plan.target, plan.https_port
        );
    } else {
        println!("  Comando:    {}", plan.command.join(" "));
    }
    println!(
        "  Efecto:     el nodo, que solo escucha en 127.0.0.1, responderá también a ese nombre."
    );
    println!(
        "              Cada dispositivo tiene que vincularse con un código (`node pair`) para hacer algo."
    );
    println!("  Para deshacerlo: `chxchx-tech node remote disable --apply`.");
    println!("Ten en cuenta:");
    println!(
        "  · Todos los dispositivos de tu tailnet podrán llegar al nodo (sin vincularse no ven nada)."
    );
    println!(
        "  · Tailscale emite un certificado HTTPS para ese nombre: la primera vez tarda hasta ~1 min,"
    );
    println!(
        "    y el nombre de esta máquina queda en los registros públicos de transparencia de certificados."
    );
    match &running {
        None => println!(
            "  · El nodo no está en marcha: inícialo con `chxchx-tech node start` (puerto fijo {port})."
        ),
        Some(info) if info.port != crate::interfaces::node::DEFAULT_PORT => println!(
            "  · El nodo usa el puerto {}, elegido al azar: si lo reinicias con otro, repite este comando. \
             Con `node start` sin --port siempre usa el {}.",
            info.port,
            crate::interfaces::node::DEFAULT_PORT
        ),
        Some(_) => {}
    }
    if !apply {
        println!("Para aplicar: `chxchx-tech node remote enable --apply`.");
        return Ok(());
    }

    tailscale::apply_enable(&tailscale, &plan)?;
    let config = RemoteConfig::new(&plan.host, plan.https_port)?;
    node_state::write_remote(&dir, &config)?;
    println!("✓ Listo: {}", plan.url());
    println!("Vincula tu celular: `chxchx-tech node pair` y abre esa dirección en él.");
    Ok(())
}

fn disable(apply: bool) -> Result<(), String> {
    let dir = dir()?;
    let Some(remote) = node_state::read_remote(&dir) else {
        println!("El acceso remoto no está activado; no hay nada que deshacer.");
        return Ok(());
    };
    let tailscale = cli()?;
    let config = tailscale::serve_config(&tailscale)?;
    let plan = tailscale::plan_disable(&config, remote.https_port);
    println!("{}", heading(apply));
    println!("  El nodo dejará de responder a {}", remote.host);
    match &plan {
        Disable::Remove { https_port } => {
            println!("  Comando:    tailscale serve --https={https_port} off");
        }
        Disable::NothingToRemove => println!("  Tailscale ya no sirve ese puerto."),
        Disable::NotOurs { https_port } => println!(
            "  El puerto {https_port} de Tailscale sirve algo que no es un proxy local: no lo toco."
        ),
    }
    if !apply {
        println!("Para aplicar: `chxchx-tech node remote disable --apply`.");
        return Ok(());
    }
    // First stop answering to the name, so the safe state holds even if the next step fails.
    node_state::remove_remote(&dir);
    if let Disable::Remove { https_port } = plan {
        tailscale::apply_disable(&tailscale, https_port)?;
    }
    println!("✓ Acceso remoto desactivado.");
    Ok(())
}
