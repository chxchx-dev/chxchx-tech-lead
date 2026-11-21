//! Projects as a client sees them: the list, each project's agents and its conversations.
//!
//! A device only reaches trusted projects, nobody gets a path, and asking for a project a device
//! may not see answers exactly as asking for one that does not exist.

use super::api::{ApiError, Principal, error, require};
use crate::features::agents::{self, Agent, Provider as AgentProvider};
use crate::features::chat_state::{self, ChatState};
use crate::features::jobs::agent_turn::project_root;
use crate::features::project_catalog::ProjectCatalog;
use crate::integrations::agent_catalog::{self, AgentOptions};
use axum::Extension;
use axum::Json;
use axum::extract::Path;
use chxchx_protocol::{
    AgentSummary, ChatMessage, ChatSummary, ErrorCode, JobError, ModelOption, ProjectId,
    ProjectSummary, Provider, Role,
};
use std::path::PathBuf;

/// Messages of a conversation a client receives: the latest ones.
const MAX_CHAT_MESSAGES: usize = 40;

/// Registered projects. A device only sees the trusted ones, and nobody sees a path.
pub(super) async fn projects(
    Extension(who): Extension<Principal>,
) -> Result<Json<Vec<ProjectSummary>>, ApiError> {
    require(&who, Role::Reader)?;
    let everything = who.is_local();
    let summaries = tokio::task::spawn_blocking(move || {
        let catalog = crate::features::project_catalog::inspect_registered_projects()?;
        Ok::<_, String>(summaries(&catalog, everything, |path| {
            crate::inspect_project(std::path::Path::new(path)).is_ok_and(|p| p.workspace.trusted)
        }))
    })
    .await
    .map_err(|_| error(ErrorCode::Internal, "Falló la lectura de proyectos"))?
    .map_err(|problem| error(ErrorCode::Internal, problem))?;
    Ok(Json(summaries))
}

/// The projects a caller may see. `trusted(path)` asks whether the project is trusted.
pub(super) fn summaries(
    catalog: &ProjectCatalog,
    include_untrusted: bool,
    trusted: impl Fn(&str) -> bool,
) -> Vec<ProjectSummary> {
    catalog
        .projects
        .iter()
        .filter(|project| project.available)
        .filter_map(|project| {
            let id = ProjectId::parse(project.alias.clone()).ok()?;
            let is_trusted = project.path.as_deref().is_some_and(&trusted);
            (is_trusted || include_untrusted).then(|| ProjectSummary {
                id,
                name: project.name.clone(),
                profile: project.profile.clone(),
                stacks: project.stacks.clone(),
                trusted: is_trusted,
            })
        })
        .collect()
}

/// The folder of a registered project the caller may see.
fn visible_root(alias: &ProjectId, everything: bool) -> Result<PathBuf, JobError> {
    let catalog = crate::features::project_catalog::inspect_registered_projects()
        .map_err(|problem| JobError::new(ErrorCode::Internal, problem))?;
    let root = project_root(&catalog, alias)?;
    let trusted = crate::inspect_project(&root).is_ok_and(|project| project.workspace.trusted);
    if trusted || everything {
        Ok(root)
    } else {
        Err(JobError::new(
            ErrorCode::NotFound,
            format!("No hay un proyecto registrado con el alias {alias}"),
        ))
    }
}

fn project_id(raw: String) -> Result<ProjectId, ApiError> {
    ProjectId::parse(raw).map_err(|problem| error(ErrorCode::InvalidRequest, problem.to_string()))
}

/// The agents of a project, with the models each one offers. Reading what the providers have
/// installed can take a moment (OpenCode lists its models by running it), so it is off the
/// async workers.
pub(super) async fn project_agents(
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<Vec<AgentSummary>>, ApiError> {
    require(&who, Role::Reader)?;
    let id = project_id(id)?;
    let everything = who.is_local();
    let list = tokio::task::spawn_blocking(move || {
        let root = visible_root(&id, everything)?;
        let found = agents::list(&root);
        Ok::<_, JobError>(summarize_agents(&found.agents, |agent| {
            agent_catalog::options(agent, &root)
        }))
    })
    .await
    .map_err(|_| error(ErrorCode::Internal, "Falló la lectura de agentes"))??;
    Ok(Json(list))
}

pub(super) fn summarize_agents(
    found: &[Agent],
    options: impl Fn(&Agent) -> AgentOptions,
) -> Vec<AgentSummary> {
    [
        (AgentProvider::Claude, Provider::Claude),
        (AgentProvider::Codex, Provider::Codex),
        (AgentProvider::OpenCode, Provider::OpenCode),
    ]
    .into_iter()
    .map(|(local, wire)| {
        let agent = found
            .iter()
            .find(|agent| agent.provider == local && agent.available());
        match agent {
            Some(agent) => {
                let options = options(agent);
                AgentSummary {
                    provider: wire,
                    installed: true,
                    install_hint: String::new(),
                    default_model: options.default_model,
                    default_effort: options.default_effort,
                    models: options
                        .models
                        .into_iter()
                        .map(|model| ModelOption {
                            id: model.id,
                            label: model.label,
                            efforts: model.efforts,
                            default_effort: model.default_effort,
                        })
                        .collect(),
                }
            }
            None => AgentSummary {
                provider: wire,
                installed: false,
                install_hint: local.install_hint().to_owned(),
                default_model: String::new(),
                default_effort: String::new(),
                models: Vec::new(),
            },
        }
    })
    .collect()
}

/// What each agent and this project have said to each other, as Studio would show it.
pub(super) async fn project_chats(
    Extension(who): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ChatSummary>>, ApiError> {
    require(&who, Role::Reader)?;
    let id = project_id(id)?;
    let everything = who.is_local();
    let chats = tokio::task::spawn_blocking(move || {
        let root = visible_root(&id, everything)?;
        let found = agents::list(&root);
        Ok::<_, JobError>(summarize_chats(&found.agents, |agent| {
            chat_state::load(&root, &agent.id)
        }))
    })
    .await
    .map_err(|_| error(ErrorCode::Internal, "Falló la lectura de conversaciones"))??;
    Ok(Json(chats))
}

pub(super) fn summarize_chats(
    found: &[Agent],
    load: impl Fn(&Agent) -> Option<ChatState>,
) -> Vec<ChatSummary> {
    let mut seen = std::collections::BTreeSet::new();
    found
        .iter()
        .filter(|agent| seen.insert(agent.id.clone()))
        .filter_map(|agent| {
            let state = load(agent)?;
            if state.messages.is_empty() {
                return None;
            }
            let skip = state.messages.len().saturating_sub(MAX_CHAT_MESSAGES);
            Some(ChatSummary {
                agent: agent.id.clone(),
                provider: state.provider,
                session_id: state.session_id,
                updated: state.updated,
                messages: state
                    .messages
                    .into_iter()
                    .skip(skip)
                    .map(|message| ChatMessage {
                        role: message.role,
                        text: message.text,
                    })
                    .collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::chat_state::Message;
    use crate::features::project_catalog::RegisteredProject;
    use crate::integrations::agent_catalog::ModelOption as Model;

    fn project(alias: &str, path: &str, available: bool) -> RegisteredProject {
        RegisteredProject {
            alias: alias.into(),
            name: alias.into(),
            profile: "rust".into(),
            path: Some(path.into()),
            stacks: vec!["rust".into()],
            available,
            is_current: false,
        }
    }

    fn agent(id: &str, provider: AgentProvider, installed: bool) -> Agent {
        Agent {
            id: id.into(),
            provider,
            command: vec![id.into()],
            cwd: PathBuf::from("/p"),
            executable: installed.then(|| PathBuf::from(format!("/bin/{id}"))),
            configured: false,
        }
    }

    #[test]
    fn a_device_sees_only_trusted_available_projects_and_never_a_path() {
        let catalog = ProjectCatalog {
            projects: vec![
                project("seguro", "/p/seguro", true),
                project("sin-confianza", "/p/otro", true),
                project("ausente", "/p/ausente", false),
                project("alias con espacios", "/p/seguro", true),
            ],
            current: None,
        };
        let trusted = |path: &str| path == "/p/seguro";
        let remote = summaries(&catalog, false, trusted);
        assert_eq!(remote.len(), 1);
        assert_eq!(remote[0].id.as_str(), "seguro");
        let json = serde_json::to_string(&remote).unwrap();
        assert!(!json.contains("/p/"), "paths stay on the PC: {json}");

        let local = summaries(&catalog, true, trusted);
        let ids: Vec<_> = local.iter().map(|p| (p.id.as_str(), p.trusted)).collect();
        assert_eq!(ids, [("seguro", true), ("sin-confianza", false)]);
    }

    #[test]
    fn every_provider_is_listed_with_its_models_or_how_to_install_it() {
        let found = [
            agent("claude", AgentProvider::Claude, true),
            agent("codex", AgentProvider::Codex, false),
        ];
        let list = summarize_agents(&found, |agent| AgentOptions {
            available: true,
            default_model: format!("{}-default", agent.id),
            models: vec![Model {
                id: "m1".into(),
                label: "Modelo 1".into(),
                efforts: vec!["low".into(), "high".into()],
                default_effort: "low".into(),
            }],
            ..Default::default()
        });
        let providers: Vec<_> = list.iter().map(|a| (a.provider, a.installed)).collect();
        assert_eq!(
            providers,
            [
                (Provider::Claude, true),
                (Provider::Codex, false),
                (Provider::OpenCode, false)
            ]
        );
        assert_eq!(list[0].default_model, "claude-default");
        assert_eq!(list[0].models[0].efforts, ["low", "high"]);
        assert!(list[1].install_hint.len() > 3 && list[1].models.is_empty());
    }

    #[test]
    fn a_conversation_is_the_latest_messages_and_empty_ones_are_left_out() {
        let found = [
            agent("claude", AgentProvider::Claude, true),
            agent("claude", AgentProvider::Claude, true), // listed twice: shown once
            agent("codex", AgentProvider::Codex, true),
            agent("opencode", AgentProvider::OpenCode, true),
        ];
        let state = |count: usize| ChatState {
            provider: "Claude".into(),
            session_id: "s-1".into(),
            updated: "2026-11-20T10:00:00Z".into(),
            messages: (0..count)
                .map(|n| Message {
                    role: if n % 2 == 0 { "user" } else { "assistant" }.into(),
                    text: format!("mensaje {n}"),
                })
                .collect(),
            ..Default::default()
        };
        let chats = summarize_chats(&found, |agent| match agent.id.as_str() {
            "claude" => Some(state(50)),
            "codex" => Some(state(0)),
            _ => None,
        });
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].messages.len(), MAX_CHAT_MESSAGES);
        assert_eq!(chats[0].messages[0].text, "mensaje 10", "the latest ones");
        assert_eq!(chats[0].messages.last().unwrap().text, "mensaje 49");
        assert_eq!(chats[0].session_id, "s-1");
    }
}
