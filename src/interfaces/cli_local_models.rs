//! Rust CLI for diagnosing and managing Ollama models in a project.
use clap::Subcommand;
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(super) enum LocalModelsCommand {
    /// Muestra hardware, Ollama, OpenCode y los modelos que caben.
    Status {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Imprime el contrato JSON `chxchx.local-models`.
        #[arg(long)]
        json: bool,
    },
    /// Descarga, ajusta el contexto y conecta un modelo con OpenCode.
    Setup {
        tag: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value_t = 32768, value_parser = clap::value_parser!(u64).range(8192..))]
        context: u64,
        #[arg(long)]
        set_default: bool,
        /// Confirma la descarga de un modelo que todavía no existe en Ollama.
        #[arg(long)]
        yes: bool,
        /// Intenta configurar un modelo aunque la memoria estimada sea insuficiente.
        #[arg(long)]
        force: bool,
        /// Muestra los pasos y el opencode.json resultante sin cambiar nada.
        #[arg(long)]
        dry_run: bool,
        /// Emite eventos JSONL.
        #[arg(long)]
        json: bool,
    },
    /// Borra el modelo y sus variantes de contexto y las quita del opencode.json.
    Remove {
        tag: String,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Confirma el borrado del modelo de Ollama.
        #[arg(long)]
        yes: bool,
        /// Muestra lo que se borraría sin cambiar nada.
        #[arg(long)]
        dry_run: bool,
        /// Emite eventos JSONL.
        #[arg(long)]
        json: bool,
    },
}

pub(super) fn run(command: LocalModelsCommand) -> Result<(), String> {
    match command {
        LocalModelsCommand::Status {
            path,
            json: as_json,
        } => status(&path, as_json),
        LocalModelsCommand::Setup {
            tag,
            path,
            context,
            set_default,
            yes,
            force,
            dry_run,
            json: as_json,
        } => operate(
            &path,
            crate::integrations::local_models::Op::Setup { tag, set_default },
            context,
            crate::integrations::local_models::OperationOptions {
                confirmed: yes,
                dry_run,
                force,
            },
            as_json,
        ),
        LocalModelsCommand::Remove {
            tag,
            path,
            yes,
            dry_run,
            json: as_json,
        } => operate(
            &path,
            crate::integrations::local_models::Op::Remove { tag },
            32768,
            crate::integrations::local_models::OperationOptions {
                confirmed: yes,
                dry_run,
                force: false,
            },
            as_json,
        ),
    }
}

fn status(path: &std::path::Path, as_json: bool) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!(
            "No existe el directorio del proyecto: {}",
            path.display()
        ));
    }
    let snapshot = crate::integrations::local_models::snapshot(path)?;
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&crate::integrations::local_models::snapshot_json(
                &snapshot
            ))
            .map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    let gib = 1024_f64.powi(3);
    println!(
        "Equipo: {:.0} GB RAM ({:.0} GB disponibles), {} núcleos, {}, {:.0} GB libres en disco.",
        snapshot.hardware.total_ram as f64 / gib,
        snapshot.hardware.available_ram as f64 / gib,
        snapshot.hardware.cpu_cores,
        snapshot.hardware.accelerator,
        snapshot.hardware.disk_free as f64 / gib,
    );
    if !snapshot.ollama.installed {
        println!("Ollama no está instalado: {}", snapshot.ollama.install_url);
    } else if !snapshot.ollama.running {
        println!(
            "Ollama no responde en {}. Inicia la aplicación o `ollama serve`.",
            snapshot.ollama.base_url
        );
    } else {
        println!(
            "Ollama {} activo en {} ({} modelos).",
            snapshot.ollama.version,
            snapshot.ollama.base_url,
            snapshot.ollama.models.len()
        );
    }
    if !snapshot.opencode_installed {
        println!("OpenCode no está instalado: {}", snapshot.opencode_hint);
    }
    println!("\nModelos locales (estimación):");
    for model in &snapshot.models {
        let state = if model.configured {
            "conectado"
        } else if model.downloaded {
            "descargado"
        } else {
            ""
        };
        let recommendation = if model.recommended { " ★" } else { "" };
        println!(
            "  {:24} {:>6}  {:8} {:12} {}",
            format!("{}{recommendation}", model.tag),
            crate::integrations::local_models::size_label(model.size),
            fit_label(&model.fit),
            state,
            model.summary,
        );
    }
    println!("\nPrevisualiza con `chxchx-tech local-models setup MODELO --dry-run`.");
    Ok(())
}

fn fit_label(fit: &str) -> &'static str {
    match fit {
        "gpu" => "GPU",
        "partial" => "GPU+RAM",
        "cpu" => "CPU",
        _ => "No cabe",
    }
}

fn operate(
    path: &std::path::Path,
    op: crate::integrations::local_models::Op,
    context: u64,
    options: crate::integrations::local_models::OperationOptions,
    as_json: bool,
) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!(
            "No existe el directorio del proyecto: {}",
            path.display()
        ));
    }
    crate::integrations::local_models::run_cli(path, op, context, options, |event| {
        emit_event(as_json, event)
    })?;
    if !as_json {
        println!(
            "{}",
            if options.dry_run {
                "Vista previa completada; no se cambió el proyecto ni Ollama."
            } else {
                "Operación completada."
            }
        );
    }
    Ok(())
}

fn emit_event(as_json: bool, event: Value) {
    if as_json {
        let mut value = event.as_object().cloned().unwrap_or_default();
        value.insert("schema".into(), json!("chxchx.local-models-event"));
        value.insert("schema_version".into(), json!(1));
        println!("{}", Value::Object(value));
        return;
    }
    match event.get("type").and_then(Value::as_str) {
        Some("step") => println!("→ {}: {}", text(&event, "step"), text(&event, "model")),
        Some("notice") => println!("{}", text(&event, "message")),
        Some("preview") => println!("{}\n{}", text(&event, "path"), text(&event, "content")),
        Some("progress") if event.get("total").and_then(Value::as_u64).unwrap_or(0) > 0 => {
            let completed = event.get("completed").and_then(Value::as_u64).unwrap_or(0);
            let total = event.get("total").and_then(Value::as_u64).unwrap_or(0);
            if completed == total {
                println!("  {} 100%", text(&event, "status"));
            }
        }
        Some("done") => println!("✓ {}", text(&event, "model")),
        Some("error") => eprintln!("Error: {}", text(&event, "message")),
        _ => {}
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}
