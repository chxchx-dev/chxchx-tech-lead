//! `chxchx-tech jobs`: run an agent turn without Studio and look at jobs afterwards.
use crate::features::jobs::{self, JobRunner, JobStore, ProductExecutor};
use chxchx_protocol::{
    AccessMode, AgentTurnSpec, IdempotencyKey, Job, JobEvent, JobEventBody, JobId, JobSpec,
    ProjectId, Provider,
};
use clap::{Subcommand, ValueEnum};
use std::time::Duration;

#[derive(Debug, Subcommand)]
pub(super) enum JobsCommand {
    /// Lista los trabajos recientes, del más nuevo al más viejo.
    List {
        /// Imprime los trabajos como JSON (`chxchx-protocol` v1).
        #[arg(long)]
        json: bool,
    },
    /// Muestra un trabajo y, con `--events`, lo que fue pasando.
    Show {
        id: String,
        #[arg(long)]
        events: bool,
        #[arg(long)]
        json: bool,
    },
    /// Habla con el agente de un proyecto registrado y de confianza, sin abrir Studio.
    ///
    /// El trabajo queda guardado: si este comando se corta, aparece como `interrupted`.
    /// Sin nadie que apruebe, cada permiso o pregunta del agente se rechaza y se anota.
    Run {
        /// Alias del proyecto (`chxchx-tech projects list`).
        #[arg(long)]
        project: String,
        #[arg(long, value_enum)]
        provider: ProviderArg,
        /// Nivel de acceso, como en el chat de Studio.
        #[arg(long, value_enum, default_value = "workspace-write")]
        access: AccessArg,
        #[arg(long)]
        network: bool,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        effort: Option<String>,
        /// Sesión del proveedor a continuar; sin ella, conversación nueva.
        #[arg(long)]
        session: Option<String>,
        /// Clave de idempotencia: repetir el comando con la misma clave no crea otro trabajo.
        #[arg(long)]
        key: Option<String>,
        /// Lo que le dices al agente.
        #[arg(required = true, num_args = 1..)]
        prompt: Vec<String>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum ProviderArg {
    Claude,
    Codex,
    #[value(name = "opencode")]
    OpenCode,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum AccessArg {
    ReadOnly,
    WorkspaceWrite,
    WorkspaceCommands,
    FullAccess,
}

pub(super) fn run(command: JobsCommand) -> Result<(), String> {
    let dir = jobs::default_dir().ok_or("No se pudo resolver CHXCHX_TECH_HOME")?;
    match command {
        JobsCommand::List { json } => {
            let list = jobs::read(&dir)?;
            if json {
                return print_json(&list);
            }
            if list.is_empty() {
                println!("No hay trabajos todavía.");
            }
            for job in &list {
                println!("{}", line(job));
            }
            Ok(())
        }
        JobsCommand::Show { id, events, json } => {
            let id = JobId::parse(id).map_err(|error| error.to_string())?;
            let job = jobs::read(&dir)?
                .into_iter()
                .find(|job| job.id == id)
                .ok_or_else(|| format!("No existe el trabajo {id}"))?;
            let history = if events {
                jobs::read_events(&dir, &id, 0)
            } else {
                Vec::new()
            };
            if json {
                return print_json(&serde_json::json!({ "job": job, "events": history }));
            }
            println!("{}", line(&job));
            if let Some(summary) = &job.summary {
                println!("Resultado: {summary}");
            }
            if let Some(error) = &job.error {
                println!("Error: {error}");
            }
            for event in &history {
                print_event(event);
            }
            Ok(())
        }
        JobsCommand::Run {
            project,
            provider,
            access,
            network,
            model,
            effort,
            session,
            key,
            prompt,
        } => {
            let spec = JobSpec::AgentTurn(AgentTurnSpec {
                project: ProjectId::parse(project).map_err(|error| error.to_string())?,
                provider: match provider {
                    ProviderArg::Claude => Provider::Claude,
                    ProviderArg::Codex => Provider::Codex,
                    ProviderArg::OpenCode => Provider::OpenCode,
                },
                prompt: prompt.join(" "),
                session,
                model,
                effort,
                access: match access {
                    AccessArg::ReadOnly => AccessMode::ReadOnly,
                    AccessArg::WorkspaceWrite => AccessMode::WorkspaceWrite,
                    AccessArg::WorkspaceCommands => AccessMode::WorkspaceCommands,
                    AccessArg::FullAccess => AccessMode::FullAccess,
                },
                network,
            });
            let key = key
                .map(IdempotencyKey::parse)
                .transpose()
                .map_err(|error| error.to_string())?;
            run_turn(&dir, spec, key)
        }
    }
}

fn run_turn(
    dir: &std::path::Path,
    spec: JobSpec,
    key: Option<IdempotencyKey>,
) -> Result<(), String> {
    let store = JobStore::open(dir)?;
    for warning in &store.warnings {
        eprintln!("Aviso: {warning}");
    }
    // The person at this machine may use any level, as in Studio.
    let executor = ProductExecutor {
        access_cap: AccessMode::FullAccess,
    };
    let runner = JobRunner::new(store, executor, 1);
    let events = runner.subscribe();
    let job = runner.submit(spec, key).map_err(|error| error.message)?;
    println!(
        "Trabajo {} · `chxchx-tech jobs show {} --events`",
        job.id, job.id
    );
    if job.state.is_terminal() {
        println!("Ya existía con esa clave: {}", line(&job));
        return Ok(());
    }
    loop {
        match events.recv_timeout(Duration::from_millis(200)) {
            Ok(event) if event.job == job.id => print_event(&event),
            Ok(_) => {}
            Err(_) => {}
        }
        let current = runner.get(&job.id).ok_or("El trabajo desapareció")?;
        if current.state.is_terminal() {
            for event in events.try_iter().filter(|event| event.job == job.id) {
                print_event(&event);
            }
            println!("{}", line(&current));
            return match current.error {
                Some(error) if current.state != chxchx_protocol::JobState::Cancelled => {
                    Err(error.message)
                }
                _ => Ok(()),
            };
        }
    }
}

fn line(job: &Job) -> String {
    let what = match &job.spec {
        JobSpec::AgentTurn(turn) => format!("{} en {}", turn.provider.as_str(), turn.project),
        JobSpec::WorkspaceStart { project } => format!("iniciar workspace de {project}"),
        JobSpec::WorkspaceStop { project } => format!("detener workspace de {project}"),
        JobSpec::LocalModelInstall { model } => format!("instalar {model}"),
        JobSpec::LocalModelRemove { model } => format!("quitar {model}"),
    };
    format!(
        "{}  {:<11}  {what}  ({})",
        job.id,
        format!("{:?}", job.state).to_lowercase(),
        ago(job.updated_ms)
    )
}

fn ago(ms: u64) -> String {
    let seconds = jobs::store::now_ms().saturating_sub(ms) / 1000;
    match seconds {
        0..60 => format!("hace {seconds} s"),
        60..3600 => format!("hace {} min", seconds / 60),
        3600..86_400 => format!("hace {} h", seconds / 3600),
        _ => format!("hace {} d", seconds / 86_400),
    }
}

fn print_event(event: &JobEvent) {
    match &event.body {
        JobEventBody::State { state } => println!("· {}", format!("{state:?}").to_lowercase()),
        JobEventBody::Progress { message, .. } => println!("  · {message}"),
        JobEventBody::Error { error } => println!("  ✗ {error}"),
        JobEventBody::Agent { event } => match event["type"].as_str().unwrap_or_default() {
            "message" if event["role"] == "assistant" => {
                println!("{}", event["text"].as_str().unwrap_or_default());
            }
            "permission" => println!(
                "  ⚠ permiso pedido: {}",
                event["summary"].as_str().unwrap_or_default()
            ),
            "question" => println!("  ⚠ el agente hizo una pregunta"),
            "tool" => {
                let title = event["title"]
                    .as_str()
                    .or_else(|| event["summary"].as_str())
                    .unwrap_or_default();
                if !title.is_empty() {
                    println!("  ▸ {title}");
                }
            }
            "error" => println!("  ✗ {}", event["message"].as_str().unwrap_or_default()),
            _ => {}
        },
    }
}

fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}
