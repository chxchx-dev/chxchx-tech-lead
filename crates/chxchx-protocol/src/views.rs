//! What the node tells a client about itself, a project's agents and its conversations.

use crate::{Provider, Role};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobCounts {
    pub queued: usize,
    pub running: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct You {
    pub role: Role,
    /// The person at the PC (local token) rather than a linked device.
    pub local: bool,
}

/// `GET /api/v1/node`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStatus {
    pub version: String,
    pub protocol: u32,
    pub started_ms: u64,
    pub jobs: JobCounts,
    pub you: You,
    /// Only for the person at the PC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// The address devices open, when remote access is enabled. Only for the person at the PC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelOption {
    pub id: String,
    pub label: String,
    pub efforts: Vec<String>,
    pub default_effort: String,
}

/// One agent of a project: whether it is installed here and what it offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSummary {
    pub provider: Provider,
    pub installed: bool,
    /// How to install it, when it is not.
    pub install_hint: String,
    pub default_model: String,
    pub default_effort: String,
    pub models: Vec<ModelOption>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// `user` or `assistant`.
    pub role: String,
    pub text: String,
}

/// The visible conversation a project has with one agent (what Studio shows too).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSummary {
    pub agent: String,
    pub provider: String,
    pub session_id: String,
    pub updated: String,
    pub messages: Vec<ChatMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_node_status_hides_what_only_the_pc_may_see() {
        let status = NodeStatus {
            version: "0.6.3".into(),
            protocol: 1,
            started_ms: 5,
            jobs: JobCounts {
                queued: 1,
                running: 2,
            },
            you: You {
                role: Role::Reader,
                local: false,
            },
            pid: None,
            remote: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert!(json.get("pid").is_none() && json.get("remote").is_none());
        assert_eq!(
            json["you"],
            serde_json::json!({ "role": "reader", "local": false })
        );
        assert_eq!(serde_json::from_value::<NodeStatus>(json).unwrap(), status);
    }
}
