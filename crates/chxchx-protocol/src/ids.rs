//! Identifiers that cross the wire. Each one is validated where it is built or deserialized, so
//! a value of these types is always safe to log, compare and use as a file name component.

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdError {
    pub kind: &'static str,
    pub reason: &'static str,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} no válido: {}", self.kind, self.reason)
    }
}

impl std::error::Error for IdError {}

fn check(
    kind: &'static str,
    value: &str,
    max: usize,
    allowed: impl Fn(char) -> bool,
) -> Result<(), IdError> {
    let fail = |reason| Err(IdError { kind, reason });
    if value.is_empty() {
        return fail("está vacío");
    }
    if value.len() > max {
        return fail("es demasiado largo");
    }
    if !value.chars().all(allowed) {
        return fail("tiene caracteres no permitidos");
    }
    Ok(())
}

macro_rules! identifier {
    ($(#[$doc:meta])* $name:ident, $kind:literal, $max:expr, $allowed:expr) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, IdError> {
                let value = value.into();
                check($kind, &value, $max, $allowed)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

fn token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

identifier!(
    /// A job, chosen by the node. Letters, digits, `-` and `_`.
    JobId,
    "Identificador de trabajo",
    64,
    token_char
);

identifier!(
    /// A device linked to the node, chosen by the node.
    DeviceId,
    "Identificador de dispositivo",
    64,
    token_char
);

identifier!(
    /// A registered project, by its alias in the project registry. A remote client never sends
    /// a path: the node resolves the alias and checks the project's trust itself.
    ProjectId,
    "Proyecto",
    128,
    |c: char| token_char(c) || c == '.'
);

identifier!(
    /// Chosen by the client so a retried request does not create a second job.
    IdempotencyKey,
    "Clave de idempotencia",
    128,
    |c: char| c.is_ascii_graphic()
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_empty_long_and_path_like_values() {
        assert!(JobId::parse("job-01_A").is_ok());
        assert!(JobId::parse("").is_err());
        assert!(JobId::parse("a".repeat(65)).is_err());
        assert!(JobId::parse("../x").is_err());

        assert!(ProjectId::parse("chxchx-tech-lead").is_ok());
        assert!(ProjectId::parse("web.app").is_ok());
        for bad in ["a/b", "a\\b", "a b", "ñ", ""] {
            assert!(ProjectId::parse(bad).is_err(), "{bad:?} must be rejected");
        }
        assert!(IdempotencyKey::parse("retry key").is_err());
        assert!(IdempotencyKey::parse("4f1c-retry").is_ok());
    }

    #[test]
    fn deserializing_an_invalid_identifier_fails_instead_of_carrying_it() {
        assert!(serde_json::from_str::<ProjectId>("\"../etc\"").is_err());
        let id: JobId = serde_json::from_str("\"j1\"").unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"j1\"");
    }
}
