//! The executor that runs the product's own jobs (ADR 0017: only use cases that already exist).

use super::runner::{Context, Executor};
use chxchx_protocol::{AccessMode, ErrorCode, Job, JobError, JobSpec};

pub struct ProductExecutor {
    /// Highest access level a turn may use: `FullAccess` for the person at this machine, a lower
    /// cap for requests from other devices.
    pub access_cap: AccessMode,
}

impl Executor for ProductExecutor {
    fn run(&self, job: &Job, context: &Context) -> Result<Option<String>, JobError> {
        match &job.spec {
            JobSpec::AgentTurn(turn) => super::agent_turn::run(turn, self.access_cap, context),
            JobSpec::WorkspaceStart { .. }
            | JobSpec::WorkspaceStop { .. }
            | JobSpec::LocalModelInstall { .. }
            | JobSpec::LocalModelRemove { .. } => Err(JobError::new(
                ErrorCode::InvalidRequest,
                "Este tipo de trabajo todavía no se puede ejecutar (llega con el nodo)",
            )),
        }
    }
}
