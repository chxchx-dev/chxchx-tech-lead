//! Wire contract between the ChxChx node and its clients (ADR 0017).
//!
//! Only data and the rules that belong to the data live here: identifiers and their validation,
//! job states and their transitions, the access cap of a remote request, and events. Nothing here
//! touches files, processes or the network, so the same types build for the desktop and for
//! WebAssembly.

mod access;
mod answers;
mod devices;
mod error;
mod events;
mod ids;
mod jobs;
mod views;

pub use access::{AccessMode, Provider};
pub use answers::{AnswerBehavior, JobAnswer};
pub use devices::{
    Device, NewPairing, PAIRING_TTL_MS, PairRequest, PairResponse, PairingOffer, ProjectSummary,
    Role, normalize_pairing_code, validate_device_name,
};
pub use error::{ErrorCode, JobError};
pub use events::{JobEvent, JobEventBody};
pub use ids::{DeviceId, IdError, IdempotencyKey, JobId, ProjectId};
pub use jobs::{AgentTurnSpec, Job, JobSpec, JobState, MAX_PROMPT_BYTES};
pub use views::{AgentSummary, ChatMessage, ChatSummary, JobCounts, ModelOption, NodeStatus, You};

/// Version of this contract. A client and a node only talk when they agree on it.
pub const PROTOCOL_VERSION: u32 = 1;

/// Prefix of every route of this protocol version.
pub const API_PREFIX: &str = "/api/v1";
