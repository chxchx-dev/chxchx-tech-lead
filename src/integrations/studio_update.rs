//! Studio's own updates: asks the installed CLI whether a newer release exists on the channel
//! this install follows and, when told to, installs it beside the running version. The CLI owns
//! the download, the SHA-256 check and the launcher switch; this adapter only drives it.
use super::chat_bridge::{command, run_short};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Info {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub updated: bool,
}

pub fn parse(output: &str) -> Result<Info, String> {
    let value: Value = serde_json::from_str(output.trim())
        .map_err(|error| format!("Respuesta de actualización inválida: {error}"))?;
    if value.get("schema").and_then(Value::as_str) != Some("chxchx.studio-update") {
        let error = value.get("error").and_then(Value::as_str).unwrap_or("");
        return Err(if error.is_empty() {
            "El CLI no devolvió el estado de actualización".to_owned()
        } else {
            error.to_owned()
        });
    }
    let text = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let flag = |key: &str| value.get(key).and_then(Value::as_bool).unwrap_or(false);
    Ok(Info {
        current: text("current_version"),
        latest: text("latest_version"),
        available: flag("available"),
        updated: flag("updated"),
    })
}

/// Whether a newer release is published. Blocking: run it off the UI thread.
pub fn check(cli: &Path) -> Result<Info, String> {
    let mut process = command(cli);
    process.args(["bridge", "studio-update"]);
    parse(&run_short(process, Duration::from_secs(40))?.1)
}

/// Downloads, verifies and installs the published release. Blocking and slow.
pub fn apply(cli: &Path) -> Result<Info, String> {
    let mut process = command(cli);
    process.args(["bridge", "studio-update", "--apply"]);
    parse(&run_short(process, Duration::from_secs(20 * 60))?.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_state_the_cli_reports() {
        let info = parse(
            r#"{"schema":"chxchx.studio-update","schema_version":1,"current_version":"0.5.3",
               "latest_version":"0.6.0a1","available":true,"updated":false}"#,
        )
        .unwrap();
        assert!(info.available && !info.updated && info.latest == "0.6.0a1");
    }

    #[test]
    fn the_error_of_the_cli_becomes_the_message() {
        let error = parse(r#"{"schema":"chxchx.error","error":"sin red"}"#).unwrap_err();
        assert_eq!(error, "sin red");
        assert!(parse("basura").is_err());
    }
}
