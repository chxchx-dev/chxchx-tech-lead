//! Local models: the "Modelos locales" tab. Studio diagnoses the machine, shows which Ollama
//! models fit and runs the setup that downloads one and connects it to OpenCode, with progress.
use super::*;
use crate::integrations::chat_bridge as bridge;
use crate::integrations::local_models::{self as models, Job, Op, Snapshot};
use serde_json::Value;

#[path = "studio_local_view.rs"]
mod view_impl;
pub(in crate::interfaces::studio) use view_impl::view;

#[derive(Clone, Debug, Default)]
pub(super) struct Progress {
    pub text: String,
    pub done: u64,
    pub total: u64,
}

#[derive(Default)]
pub(super) struct State {
    pub snapshot: Option<Snapshot>,
    pub loading: bool,
    pub error: Option<String>,
    pub job: Option<Job>,
    pub progress: Progress,
    /// What the last operation reported.
    pub result: Option<String>,
    /// Makes the next installed model OpenCode's default for the project.
    pub set_default: bool,
    cli: Option<std::path::PathBuf>,
}

impl State {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    /// Opens the tab (and diagnoses the machine the first time).
    Open,
    Refresh,
    Loaded(Box<Result<Snapshot, String>>, Option<std::path::PathBuf>),
    Install(String),
    Remove(String),
    Cancel,
    ToggleDefault,
    /// The visual chat of OpenCode with this connected model (its name in `opencode.json`).
    Chat(String),
    /// OpenCode in a terminal with this connected model.
    Terminal(String),
    Copy(String),
    Tick,
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Open => {
            state.panes.show(panes::Content::Local);
            if state.local.snapshot.is_none() && !state.local.loading {
                return refresh(state);
            }
        }
        Event::Refresh => return refresh(state),
        Event::Loaded(result, cli) => {
            state.local.loading = false;
            if cli.is_some() {
                state.local.cli = cli;
            }
            match *result {
                Ok(snapshot) => {
                    state.local.snapshot = Some(snapshot);
                    state.local.error = None;
                }
                Err(error) => state.local.error = Some(error),
            }
        }
        Event::Install(tag) => start(
            state,
            Op::Setup {
                tag,
                set_default: state.local.set_default,
            },
        ),
        Event::Remove(tag) => start(state, Op::Remove { tag }),
        Event::Cancel => {
            if let Some(job) = state.local.job.as_mut() {
                job.cancel();
            }
        }
        Event::ToggleDefault => state.local.set_default = !state.local.set_default,
        Event::Chat(name) => match opencode(state) {
            Some(agent) => {
                return Task::done(Message::Chat(chat::Event::OpenWith(
                    agent,
                    model_ref(&name),
                )));
            }
            None => state.message = NEEDS_OPENCODE.into(),
        },
        Event::Terminal(name) => match opencode(state) {
            Some(agent) => {
                let title = format!("{} · local", short(&name));
                let local = with_model(&agent, &name);
                return Task::done(Message::Terminal(terminal::Event::OpenAgentAs(
                    local, title,
                )));
            }
            None => state.message = NEEDS_OPENCODE.into(),
        },
        Event::Copy(text) => return iced::clipboard::write(text),
        Event::Tick => return tick(state),
    }
    Task::none()
}

const NEEDS_OPENCODE: &str =
    "Los modelos locales se usan a través de OpenCode: instálalo desde el panel Agentes.";

/// How OpenCode names a model served by Ollama.
pub(super) fn model_ref(name: &str) -> String {
    format!("ollama/{name}")
}

/// `qwen2.5-coder:7b-ctx32k` -> `qwen2.5-coder:7b`.
pub(super) fn short(name: &str) -> &str {
    name.rsplit_once("-ctx").map_or(name, |(base, _)| base)
}

fn opencode(state: &Dashboard) -> Option<crate::features::agents::Agent> {
    state
        .agents
        .catalog
        .agents
        .iter()
        .find(|a| a.provider == crate::features::agents::Provider::OpenCode && a.available())
        .cloned()
}

/// OpenCode started with a local model: its own tab, distinct from the plain OpenCode one.
fn with_model(
    agent: &crate::features::agents::Agent,
    name: &str,
) -> crate::features::agents::Agent {
    let mut local = agent.clone();
    local.id = format!("{}:local:{name}", agent.id);
    local
        .command
        .extend(["--model".to_owned(), model_ref(name)]);
    local
}

/// The name under which a catalog model is connected in `opencode.json`, if it is.
pub(super) fn connected_name(snapshot: &Snapshot, tag: &str) -> Option<String> {
    snapshot
        .configured
        .iter()
        .find(|name| name.as_str() == tag || short(name) == tag)
        .cloned()
}

fn refresh(state: &mut Dashboard) -> Task<Message> {
    if state.local.loading || state.local.busy() {
        return Task::none();
    }
    state.local.loading = true;
    let (root, known) = (state.root.clone(), state.local.cli.clone());
    Task::perform(
        async move {
            jobs::offload(move || {
                let cli = known.or_else(|| bridge::find_cli(&root));
                match &cli {
                    Some(cli) => (models::snapshot(cli, &root), Some(cli.clone())),
                    None => (Err(bridge::missing_cli_hint()), None),
                }
            })
            .await
            .unwrap_or_else(|| (Err("No se pudo diagnosticar el equipo".into()), None))
        },
        |(result, cli)| Message::Local(Event::Loaded(Box::new(result), cli)),
    )
}

fn start(state: &mut Dashboard, op: Op) {
    if state.local.busy() {
        return;
    }
    if !state.overview.as_ref().is_ok_and(|p| p.workspace.trusted) {
        state.message = "Concede confianza al proyecto para configurar modelos locales.".into();
        return;
    }
    let Some(cli) = state.local.cli.clone() else {
        state.local.error = Some(bridge::missing_cli_hint());
        return;
    };
    let context = state
        .local
        .snapshot
        .as_ref()
        .map_or(32768, |s| s.context.max(8192));
    state.local.result = None;
    state.local.error = None;
    state.local.progress = Progress {
        text: "Preparando…".into(),
        ..Progress::default()
    };
    match Job::start(&cli, &state.root, op, context) {
        Ok(job) => state.local.job = Some(job),
        Err(error) => state.local.error = Some(error),
    }
}

fn text_of(event: &Value, key: &str) -> String {
    event
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn apply(local: &mut State, event: &Value) {
    match event.get("type").and_then(Value::as_str) {
        Some("step") => {
            let model = text_of(event, "model");
            local.progress = Progress {
                text: match event.get("step").and_then(Value::as_str) {
                    Some("download") => format!("Descargando {model}…"),
                    Some("context") => format!("Ampliando el contexto de {model}…"),
                    Some("config") => format!("Conectando {model} a OpenCode…"),
                    Some("remove") => format!("Quitando {model}…"),
                    _ => format!("Trabajando en {model}…"),
                },
                ..Progress::default()
            };
        }
        Some("progress") => {
            local.progress = Progress {
                text: text_of(event, "status"),
                done: event.get("completed").and_then(Value::as_u64).unwrap_or(0),
                total: event.get("total").and_then(Value::as_u64).unwrap_or(0),
            };
        }
        Some("notice") => local.result = Some(text_of(event, "message")),
        Some("error") => local.error = Some(text_of(event, "message")),
        Some("done") => {
            let model = text_of(event, "model");
            local.result = Some(if event.get("removed").is_some() {
                format!("Se quitó {model}.")
            } else if event
                .get("changed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                format!("Listo: {model} quedó conectado a OpenCode en este proyecto.")
            } else {
                format!("{model} ya estaba conectado a OpenCode.")
            });
        }
        _ => {}
    }
}

fn tick(state: &mut Dashboard) -> Task<Message> {
    let Some(job) = state.local.job.as_mut() else {
        return Task::none();
    };
    let events = job.poll();
    let exit = job.exit;
    for event in &events {
        apply(&mut state.local, event);
    }
    let Some(code) = exit else {
        return Task::none();
    };
    state.local.job = None;
    if code != 0 && state.local.error.is_none() {
        state.local.error = Some(if code == -1 || code == 137 || code == 143 {
            "Se canceló la operación.".to_owned()
        } else {
            format!("La operación terminó con error (código {code}).")
        });
    }
    state.local.progress = Progress::default();
    refresh(state)
}

#[cfg(test)]
#[path = "studio_local_tests.rs"]
mod tests;
