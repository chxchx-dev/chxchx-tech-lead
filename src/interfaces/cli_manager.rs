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
    }
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
