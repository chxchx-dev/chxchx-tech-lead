//! Linked devices, their roles and how they are linked (ADR 0017, H3).

use crate::{DeviceId, ErrorCode, JobError, ProjectId};
use serde::{Deserialize, Serialize};

/// What a device may do. Ordered: each role can do everything the previous one can.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Sees projects, jobs and events.
    Reader,
    /// Also starts and cancels jobs and answers what an agent asks.
    Operator,
    /// Also manages linked devices and reads the audit log.
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Reader => "reader",
            Role::Operator => "operator",
            Role::Admin => "admin",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [Role::Reader, Role::Operator, Role::Admin]
            .into_iter()
            .find(|role| role.as_str() == value)
    }
}

/// A linked device as it is shown: never its token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    pub role: Role,
    pub created_ms: u64,
    /// The last time it made an authenticated request, as far as this run of the node knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_ms: Option<u64>,
}

/// How long a pairing code lasts.
pub const PAIRING_TTL_MS: u64 = 5 * 60 * 1000;

/// Asked on the PC (with the local token): a one-use code for a new device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewPairing {
    #[serde(default = "default_role")]
    pub role: Role,
}

fn default_role() -> Role {
    Role::Operator
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingOffer {
    /// `XXXXX-XXXXX`, to type or to put in a QR.
    pub code: String,
    pub role: Role,
    pub expires_ms: u64,
}

/// Sent by the new device with the code it was shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairRequest {
    pub code: String,
    /// How the person will recognise this device in the list (e.g. "Pixel de Ana").
    pub name: String,
}

/// The answer to a valid pairing: the device's token, shown exactly once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairResponse {
    pub device: Device,
    pub token: String,
}

/// A project a client may see. Never a path: the node resolves the alias itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub name: String,
    pub profile: String,
    pub stacks: Vec<String>,
    pub trusted: bool,
}

/// Upper case, without the dash or spaces, so `abcde-fgh23` and `ABCDEFGH23` are the same code.
pub fn normalize_pairing_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

pub fn validate_device_name(name: &str) -> Result<(), JobError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(JobError::new(
            ErrorCode::InvalidRequest,
            "El nombre del dispositivo debe tener entre 1 y 64 caracteres, sin saltos de línea",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_ordered_and_have_stable_names() {
        assert!(Role::Admin > Role::Operator && Role::Operator > Role::Reader);
        for role in [Role::Reader, Role::Operator, Role::Admin] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
            assert_eq!(
                serde_json::to_string(&role).unwrap(),
                format!("\"{}\"", role.as_str())
            );
        }
        assert_eq!(Role::parse("root"), None);
    }

    #[test]
    fn pairing_codes_compare_without_caring_about_case_dashes_or_spaces() {
        assert_eq!(normalize_pairing_code("abcde-fgh23"), "ABCDEFGH23");
        assert_eq!(normalize_pairing_code(" ABCDE FGH23 "), "ABCDEFGH23");
    }

    #[test]
    fn a_new_pairing_defaults_to_operator_and_refuses_unknown_fields() {
        let pairing: NewPairing = serde_json::from_str("{}").unwrap();
        assert_eq!(pairing.role, Role::Operator);
        assert!(serde_json::from_str::<NewPairing>(r#"{"role":"admin","x":1}"#).is_err());
        assert!(serde_json::from_str::<PairRequest>(r#"{"code":"A","name":"B","x":1}"#).is_err());
    }

    #[test]
    fn device_names_are_short_single_line_text() {
        assert!(validate_device_name("Pixel de Ana").is_ok());
        for bad in ["", "   ", "a\nb", &"x".repeat(65)] {
            assert!(validate_device_name(bad).is_err(), "{bad:?}");
        }
    }
}
