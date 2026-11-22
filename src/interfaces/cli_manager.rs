//! `chxchx-tech manager status|plan|apply`: what this machine has, what is missing, and doing the
//! part of it that belongs to ChxChx (ADR 0017, H6). `plan` only shows; `apply` does the automatic
//! steps and says what is left for the person; running it again changes nothing.
use crate::features::manager::{self, Auto, Facts, Fix, Health, Step};
use crate::integrations::manager_probe;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(super) enum ManagerCommand {
    /// Lo que hay en esta máquina: agentes, Python, servicio de agentes, nodo y Tailscale.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Lo que haría falta para dejarla lista, sin cambiar nada.
    Plan,
    /// Busca una versión nueva firmada y, con `--apply`, la instala (solo Linux; con vuelta atrás).
    ///
    /// Comprueba la firma del manifiesto y el SHA-256 del paquete, espera a que no haya trabajos en
    /// curso y, si el programa instalado no responde después, restaura los archivos anteriores.
    Update {
        /// Instala la versión encontrada; sin esto solo la muestra.
        #[arg(long)]
        apply: bool,
        /// Usa el canal de desarrollo en vez del estable.
        #[arg(long)]
        dev: bool,
    },
    /// Vigila el nodo y el servicio de agentes y los levanta si se caen (espera creciente; se
    /// detiene y lo explica tras 3 reinicios en una hora).
    Watch {
        /// Una sola pasada y termina.
        #[arg(long)]
        once: bool,
        /// Segundos entre revisiones.
        #[arg(long, default_value_t = 15)]
        interval: u64,
    },
    /// Repara lo que es de ChxChx: copias viejas del servicio de agentes y su copia rota.
    Recover {
        #[arg(long)]
        dry_run: bool,
    },
    /// Genera el par de claves con el que se firman los lanzamientos (lo hace el mantenedor).
    Keygen {
        /// Dónde guardar la clave privada (se crea con permisos privados; no se sobrescribe).
        #[arg(long)]
        out: std::path::PathBuf,
        /// Escribe la clave pública en este archivo (normalmente `packaging/release-public-key.txt`),
        /// conservando sus comentarios, para no tener que copiarla a mano.
        #[arg(long)]
        public_file: Option<std::path::PathBuf>,
    },
    /// Hace los pasos automáticos del plan (los que son de ChxChx y se pueden repetir).
    Apply {
        /// Solo muestra lo que haría.
        #[arg(long)]
        dry_run: bool,
    },
}

pub(super) fn run(command: ManagerCommand) -> Result<(), String> {
    let facts = manager_probe::read();
    match command {
        ManagerCommand::Status { json } => status(&facts, json),
        ManagerCommand::Plan => {
            show_plan(&manager::plan(&facts));
            Ok(())
        }
        ManagerCommand::Apply { dry_run } => apply(&facts, dry_run),
        ManagerCommand::Update { .. }
        | ManagerCommand::Watch { .. }
        | ManagerCommand::Recover { .. }
        | ManagerCommand::Keygen { .. } => unreachable!("handled before reading the machine"),
    }
}

/// The commands that do not need the inventory.
pub(super) fn run_other(command: ManagerCommand) -> Result<(), String> {
    use crate::integrations::manager_recover as recover;
    match command {
        ManagerCommand::Update { apply, dev } => update(apply, dev),
        ManagerCommand::Watch { once, interval } => recover::watch(
            once,
            std::time::Duration::from_secs(interval.max(1)),
            &|line| println!("{line}"),
        ),
        ManagerCommand::Recover { dry_run } => {
            for line in recover::recover(dry_run)? {
                println!("{line}");
            }
            Ok(())
        }
        ManagerCommand::Keygen { out, public_file } => keygen(&out, public_file.as_deref()),
        other => run(other),
    }
}

fn update(apply: bool, dev: bool) -> Result<(), String> {
    use crate::features::release_manifest as release;
    use crate::integrations::self_update as upd;
    let current = env!("CARGO_PKG_VERSION");
    let work = crate::features::project_profiles::application_home()
        .ok_or("No se pudo resolver la carpeta de ChxChx.")?
        .join("manager")
        .join("update");
    let source = upd::GitHub { dev };
    let Some(offer) = upd::check(&source, &work, current, release::built_in_key())? else {
        println!("Ya tienes la última versión ({current}).");
        return Ok(());
    };
    println!(
        "Hay una versión firmada más nueva: {} (tienes {current}). Paquete: {}",
        offer.manifest.version, offer.file
    );
    if !apply {
        println!("Vista previa: no se cambió nada. Añade --apply para instalarla.");
        return Ok(());
    }
    upd::platform_supported(std::env::consts::OS, std::env::consts::ARCH)?;
    let install = upd::install_dir()?;
    let was_running = node_is_running();
    let outcome = upd::apply(
        &offer,
        &source,
        &install,
        &work,
        &drain_node,
        &upd::answers_with,
    );
    if was_running {
        restart_node();
    }
    for line in outcome? {
        println!("{line}");
    }
    println!(
        "Versión {} instalada. Los archivos anteriores quedan en {}/.update-backup",
        offer.manifest.version,
        install.display()
    );
    Ok(())
}

#[cfg(feature = "node")]
fn node_is_running() -> bool {
    crate::integrations::node_link::endpoint().is_some()
}

#[cfg(not(feature = "node"))]
fn node_is_running() -> bool {
    false
}

/// Lets running jobs finish before files change: with any running it refuses, it never cancels them.
#[cfg(feature = "node")]
fn drain_node() -> Result<(), String> {
    use crate::integrations::node_link;
    let Some(endpoint) = node_link::endpoint() else {
        return Ok(());
    };
    let running = node_link::status(&endpoint)?.jobs.running;
    if running > 0 {
        return Err(format!(
            "Hay {running} trabajo(s) en curso. Espera a que terminen (o cancélalos tú) y repite."
        ));
    }
    node_link::stop(&endpoint)?;
    for _ in 0..50 {
        if node_link::endpoint().is_none() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err("El nodo no terminó de detenerse.".to_owned())
}

#[cfg(not(feature = "node"))]
fn drain_node() -> Result<(), String> {
    Ok(())
}

/// Starts the node again with the new program: through its service when it has one.
#[cfg(feature = "node")]
fn restart_node() {
    use crate::integrations::{node_process, user_service as svc};
    let managed = svc::current_setup(crate::interfaces::node::DEFAULT_PORT)
        .map(|setup| svc::is_installed(&svc::plan(&setup)))
        .unwrap_or(false);
    if managed && cfg!(target_os = "linux") {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "restart", svc::NAME])
            .status();
    } else if let Err(error) = node_process::start() {
        println!("No pude volver a iniciar el nodo: {error}");
    }
}

#[cfg(not(feature = "node"))]
fn restart_node() {}

#[cfg(feature = "node")]
fn keygen(out: &std::path::Path, public_file: Option<&std::path::Path>) -> Result<(), String> {
    use crate::features::release_manifest as release;
    if out.exists() {
        return Err(format!("{} ya existe: no se sobrescribe.", out.display()));
    }
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed)
        .map_err(|error| format!("Sin fuente de azar del sistema: {error}"))?;
    let public = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    crate::features::node_state::write_private(
        out,
        format!("{}\n", release::to_hex(&seed)).as_bytes(),
    )?;
    let public_hex = release::to_hex(&public);
    println!(
        "Clave privada guardada en {} (no la subas a git; guarda una copia segura).",
        out.display()
    );
    if let Some(file) = public_file {
        let kept: Vec<String> = std::fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .filter(|line| line.trim_start().starts_with('#'))
            .map(str::to_owned)
            .collect();
        std::fs::write(file, format!("{}\n{public_hex}\n", kept.join("\n")))
            .map_err(|error| format!("{}: {error}", file.display()))?;
        println!(
            "Clave pública escrita en {}: solo falta subirla (git).",
            file.display()
        );
    } else {
        println!("Clave pública (ponla en packaging/release-public-key.txt): {public_hex}");
    }
    println!(
        "Falta un paso tuyo: en GitHub → Settings → Secrets and variables → Actions → New repository secret, \
         nombre RELEASE_SIGNING_KEY y como valor el contenido de {}.",
        out.display()
    );
    Ok(())
}

#[cfg(not(feature = "node"))]
fn keygen(_: &std::path::Path, _: Option<&std::path::Path>) -> Result<(), String> {
    Err("Esta compilación no incluye `--features node`.".to_owned())
}

fn mark(health: Health) -> &'static str {
    match health {
        Health::Ready => "✓",
        Health::Missing => "·",
        Health::Attention => "!",
    }
}

fn status(facts: &Facts, json: bool) -> Result<(), String> {
    let items = manager::inventory(facts);
    if json {
        let list: Vec<_> = items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "label": item.label,
                    "health": match item.health {
                        Health::Ready => "ready",
                        Health::Missing => "missing",
                        Health::Attention => "attention",
                    },
                    "detail": item.detail,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "chxchx.manager-status",
                "items": list,
            }))
            .map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    for item in &items {
        println!("{} {:<40} {}", mark(item.health), item.label, item.detail);
    }
    Ok(())
}

fn describe(step: &Step) -> String {
    match &step.fix {
        Fix::Auto(Auto::UnpackAgentService) => {
            "Desempaquetar el servicio de Codex y OpenCode".to_owned()
        }
        Fix::Auto(Auto::InstallNodeService) => {
            "Instalar el nodo como servicio de tu usuario (arranca al iniciar sesión)".to_owned()
        }
        Fix::Manual(text) => text.clone(),
    }
}

fn show_plan(steps: &[Step]) {
    if steps.is_empty() {
        println!("Todo está listo: no hace falta cambiar nada.");
        return;
    }
    for step in steps {
        let kind = if matches!(step.fix, Fix::Auto(_)) {
            "automático"
        } else {
            "lo haces tú"
        };
        println!("[{kind}] {}\n    {}", describe(step), step.why);
    }
}

fn apply(facts: &Facts, dry_run: bool) -> Result<(), String> {
    let steps = manager::plan(facts);
    show_plan(&steps);
    if dry_run || steps.is_empty() {
        if dry_run {
            println!("\n--dry-run: no se cambió nada.");
        }
        return Ok(());
    }
    println!();
    let mut left = 0;
    for step in &steps {
        match step.fix {
            Fix::Auto(auto) => match perform(auto) {
                Ok(done) => println!("✓ {done}"),
                Err(error) => {
                    left += 1;
                    println!("✗ {}: {error}", describe(step));
                }
            },
            Fix::Manual(_) => left += 1,
        }
    }
    if left > 0 {
        println!("\nQuedan {left} paso(s) que no puede hacer ChxChx por ti (arriba).");
    }
    Ok(())
}

fn perform(auto: Auto) -> Result<String, String> {
    match auto {
        Auto::UnpackAgentService => crate::integrations::agent_adapter_bundle::ensure()
            .map(|path| format!("Servicio de agentes listo en {}", path.display())),
        #[cfg(feature = "node")]
        Auto::InstallNodeService => {
            use crate::integrations::user_service as svc;
            let plan = svc::plan(&svc::current_setup(crate::interfaces::node::DEFAULT_PORT)?);
            svc::install(&plan).map(|lines| lines.join("; "))
        }
        #[cfg(not(feature = "node"))]
        Auto::InstallNodeService => {
            Err("Este programa se compiló sin el nodo (`--features node`).".to_owned())
        }
    }
}
