//! Agents and access levels (ADR 0014), as the wire names them.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
    #[serde(rename = "opencode")]
    OpenCode,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
            Provider::OpenCode => "opencode",
        }
    }
}

/// The access levels of the visual chat, from least to most permissive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccessMode {
    ReadOnly,
    WorkspaceWrite,
    WorkspaceCommands,
    FullAccess,
}

impl AccessMode {
    /// The level a remote request may use (ADR 0017).
    pub const REMOTE_DEFAULT_CAP: AccessMode = AccessMode::WorkspaceWrite;

    pub fn as_str(self) -> &'static str {
        match self {
            AccessMode::ReadOnly => "read-only",
            AccessMode::WorkspaceWrite => "workspace-write",
            AccessMode::WorkspaceCommands => "workspace-commands",
            AccessMode::FullAccess => "full-access",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [
            AccessMode::ReadOnly,
            AccessMode::WorkspaceWrite,
            AccessMode::WorkspaceCommands,
            AccessMode::FullAccess,
        ]
        .into_iter()
        .find(|mode| mode.as_str() == value)
    }

    /// Never more than `cap`: a request cannot raise its own level.
    pub fn capped(self, cap: AccessMode) -> AccessMode {
        self.min(cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_match_the_ones_the_product_already_uses() {
        for mode in [
            AccessMode::ReadOnly,
            AccessMode::WorkspaceWrite,
            AccessMode::WorkspaceCommands,
            AccessMode::FullAccess,
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{}\"", mode.as_str()));
            assert_eq!(AccessMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(
            serde_json::to_string(&Provider::OpenCode).unwrap(),
            "\"opencode\""
        );
        assert_eq!(AccessMode::parse("root"), None);
    }

    #[test]
    fn a_cap_lowers_a_request_and_never_raises_it() {
        let cap = AccessMode::REMOTE_DEFAULT_CAP;
        assert_eq!(
            AccessMode::FullAccess.capped(cap),
            AccessMode::WorkspaceWrite
        );
        assert_eq!(AccessMode::ReadOnly.capped(cap), AccessMode::ReadOnly);
    }
}
