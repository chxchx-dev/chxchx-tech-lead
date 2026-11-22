//! What a job reports while it runs. Events carry a sequence number per job, so a client that
//! reconnects asks for what came after the last one it saw instead of starting over.

use crate::{JobError, JobId, JobState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobEvent {
    pub job: JobId,
    /// Starts at 1 and grows by one for each event of this job.
    pub seq: u64,
    /// Milliseconds since the Unix epoch.
    pub at_ms: u64,
    #[serde(flatten)]
    pub body: JobEventBody,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobEventBody {
    State {
        state: JobState,
    },
    Progress {
        message: String,
        /// Between 0 and 1 when the job knows how far it is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fraction: Option<f32>,
    },
    /// A normalized agent event (text, tool card, permission request, usage…), as the product
    /// already produces it for Studio. Kept as JSON so the agent event schema can grow without a
    /// new protocol version.
    Agent {
        event: serde_json::Value,
    },
    Error {
        error: JobError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorCode;

    #[test]
    fn events_are_flat_objects_with_a_type() {
        let event = JobEvent {
            job: JobId::parse("j1").unwrap(),
            seq: 3,
            at_ms: 10,
            body: JobEventBody::State {
                state: JobState::Running,
            },
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "job": "j1", "seq": 3, "at_ms": 10, "type": "state", "state": "running" })
        );
        assert_eq!(serde_json::from_value::<JobEvent>(json).unwrap(), event);
    }

    #[test]
    fn every_kind_of_event_round_trips() {
        let bodies = [
            JobEventBody::Progress {
                message: "Descargando".into(),
                fraction: Some(0.5),
            },
            JobEventBody::Agent {
                event: serde_json::json!({ "type": "text", "text": "hola" }),
            },
            JobEventBody::Error {
                error: JobError::new(ErrorCode::ProviderFailed, "falló"),
            },
        ];
        for (seq, body) in bodies.into_iter().enumerate() {
            let event = JobEvent {
                job: JobId::parse("j1").unwrap(),
                seq: seq as u64 + 1,
                at_ms: 0,
                body,
            };
            let text = serde_json::to_string(&event).unwrap();
            assert_eq!(serde_json::from_str::<JobEvent>(&text).unwrap(), event);
        }
    }
}
