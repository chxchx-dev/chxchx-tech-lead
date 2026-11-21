//! What a job asks for, where it is in its life, and what it ended with.

use crate::{AccessMode, ErrorCode, IdempotencyKey, JobError, JobId, ProjectId, Provider};
use serde::{Deserialize, Serialize};

/// Largest prompt a job accepts; a bigger one is refused before anything runs.
pub const MAX_PROMPT_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    /// Its owner stopped (closed, crashed, the machine slept) before it finished.
    Interrupted,
}

impl JobState {
    pub fn is_terminal(self) -> bool {
        !matches!(self, JobState::Queued | JobState::Running)
    }

    /// The only moves a job can make; anything else is a bug or a stale writer.
    pub fn can_become(self, next: JobState) -> bool {
        use JobState::*;
        matches!(
            (self, next),
            (Queued, Running | Cancelled | Interrupted)
                | (Running, Succeeded | Failed | Cancelled | Interrupted)
        )
    }
}

/// One chat turn with an agent, in a registered and trusted project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTurnSpec {
    pub project: ProjectId,
    pub provider: Provider,
    pub prompt: String,
    /// Provider session to continue; a new conversation when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub access: AccessMode,
    #[serde(default)]
    pub network: bool,
}

/// The actions a job can perform: only use cases the product already has (ADR 0017).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobSpec {
    AgentTurn(AgentTurnSpec),
    WorkspaceStart { project: ProjectId },
    WorkspaceStop { project: ProjectId },
    LocalModelInstall { model: String },
    LocalModelRemove { model: String },
}

impl JobSpec {
    pub fn project(&self) -> Option<&ProjectId> {
        match self {
            JobSpec::AgentTurn(turn) => Some(&turn.project),
            JobSpec::WorkspaceStart { project } | JobSpec::WorkspaceStop { project } => {
                Some(project)
            }
            JobSpec::LocalModelInstall { .. } | JobSpec::LocalModelRemove { .. } => None,
        }
    }

    /// Checks what the types cannot: sizes and empty values.
    pub fn validate(&self) -> Result<(), JobError> {
        let invalid = |message: &str| Err(JobError::new(ErrorCode::InvalidRequest, message));
        match self {
            JobSpec::AgentTurn(turn) => {
                if turn.prompt.trim().is_empty() {
                    return invalid("El mensaje está vacío");
                }
                if turn.prompt.len() > MAX_PROMPT_BYTES {
                    return invalid("El mensaje es demasiado largo");
                }
                for value in [&turn.session, &turn.model, &turn.effort]
                    .into_iter()
                    .flatten()
                {
                    if value.is_empty() || value.len() > 200 || value.chars().any(char::is_control)
                    {
                        return invalid("Sesión, modelo o esfuerzo no válidos");
                    }
                }
                Ok(())
            }
            JobSpec::LocalModelInstall { model } | JobSpec::LocalModelRemove { model } => {
                let valid = !model.is_empty()
                    && model.len() <= 200
                    && model
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c));
                if valid {
                    Ok(())
                } else {
                    invalid("Nombre de modelo no válido")
                }
            }
            JobSpec::WorkspaceStart { .. } | JobSpec::WorkspaceStop { .. } => Ok(()),
        }
    }
}

/// A job as the node keeps it and reports it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub spec: JobSpec,
    pub state: JobState,
    /// Milliseconds since the Unix epoch.
    pub created_ms: u64,
    pub updated_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    /// Short human summary of the outcome (the end of an agent's answer, a model installed…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JobError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(prompt: &str) -> JobSpec {
        JobSpec::AgentTurn(AgentTurnSpec {
            project: ProjectId::parse("demo").unwrap(),
            provider: Provider::Claude,
            prompt: prompt.into(),
            session: None,
            model: None,
            effort: None,
            access: AccessMode::WorkspaceWrite,
            network: false,
        })
    }

    #[test]
    fn states_only_move_forward_and_terminal_states_are_final() {
        use JobState::*;
        assert!(Queued.can_become(Running));
        assert!(Running.can_become(Interrupted));
        assert!(!Running.can_become(Queued));
        assert!(!Queued.can_become(Succeeded), "it has to run first");
        for terminal in [Succeeded, Failed, Cancelled, Interrupted] {
            assert!(terminal.is_terminal());
            for next in [Queued, Running, Succeeded, Failed, Cancelled, Interrupted] {
                assert!(!terminal.can_become(next));
            }
        }
    }

    #[test]
    fn an_agent_turn_has_a_stable_wire_shape() {
        let json = serde_json::to_value(turn("hola")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "kind": "agent_turn",
                "project": "demo",
                "provider": "claude",
                "prompt": "hola",
                "access": "workspace-write",
                "network": false
            })
        );
        let back: JobSpec = serde_json::from_value(json).unwrap();
        assert_eq!(back, turn("hola"));
    }

    #[test]
    fn requests_with_unknown_fields_or_kinds_are_refused() {
        let extra = serde_json::json!({
            "kind": "agent_turn", "project": "demo", "provider": "claude",
            "prompt": "hola", "access": "read-only", "cwd": "/etc"
        });
        assert!(serde_json::from_value::<JobSpec>(extra).is_err());
        let shell = serde_json::json!({ "kind": "shell", "command": "rm -rf /" });
        assert!(serde_json::from_value::<JobSpec>(shell).is_err());
    }

    #[test]
    fn validation_refuses_empty_oversized_and_odd_values() {
        assert!(turn("hola").validate().is_ok());
        assert!(turn("   ").validate().is_err());
        assert!(turn(&"x".repeat(MAX_PROMPT_BYTES + 1)).validate().is_err());
        let ok = JobSpec::LocalModelInstall {
            model: "qwen2.5-coder:7b".into(),
        };
        assert!(ok.validate().is_ok());
        let bad = JobSpec::LocalModelInstall {
            model: "x; rm -rf ~".into(),
        };
        assert_eq!(bad.validate().unwrap_err().code, ErrorCode::InvalidRequest);
    }

    #[test]
    fn a_job_round_trips_and_omits_what_it_does_not_have() {
        let job = Job {
            id: JobId::parse("j1").unwrap(),
            spec: turn("hola"),
            state: JobState::Queued,
            created_ms: 1,
            updated_ms: 1,
            idempotency_key: None,
            summary: None,
            error: None,
        };
        let json = serde_json::to_value(&job).unwrap();
        assert!(json.get("error").is_none());
        assert_eq!(serde_json::from_value::<Job>(json).unwrap(), job);
    }
}
