//! Local models through Ollama: the machine's hardware, what Ollama has, which models fit and
//! the setup that downloads one and connects it to OpenCode. The Python CLI keeps doing the
//! work (`bridge local-models`, `local-models setup|remove`); this adapter only drives it.
use super::chat_bridge::{command, run_short};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hardware {
    pub total_ram: u64,
    pub available_ram: u64,
    pub cpu_cores: u64,
    pub disk_free: u64,
    pub models_dir: String,
    pub accelerator: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ollama {
    pub installed: bool,
    pub running: bool,
    pub version: String,
    pub base_url: String,
    pub install_command: Option<String>,
    pub install_url: String,
    /// `(name, size in bytes)` of each downloaded model.
    pub models: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Candidate {
    pub tag: String,
    pub label: String,
    pub size: u64,
    pub context_k: u64,
    pub note: String,
    /// `gpu`, `partial`, `cpu` or `none`.
    pub fit: String,
    pub required: u64,
    pub disk_ok: bool,
    pub summary: String,
    pub recommended: bool,
    pub downloaded: bool,
    pub configured: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub hardware: Hardware,
    pub ollama: Ollama,
    pub opencode_installed: bool,
    pub opencode_hint: String,
    pub configured: Vec<String>,
    pub context: u64,
    pub models: Vec<Candidate>,
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn parse_snapshot(output: &str) -> Result<Snapshot, String> {
    let value: Value = serde_json::from_str(output.trim())
        .map_err(|error| format!("Respuesta de modelos locales inválida: {error}"))?;
    if value.get("schema").and_then(Value::as_str) != Some("chxchx.local-models") {
        let error = string(&value, "error");
        return Err(if error.is_empty() {
            "El CLI no devolvió los modelos locales".to_owned()
        } else {
            error
        });
    }
    let (hw, ol, oc) = (&value["hardware"], &value["ollama"], &value["opencode"]);
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    Ok(Snapshot {
        hardware: Hardware {
            total_ram: number(hw, "total_ram_bytes"),
            available_ram: number(hw, "available_ram_bytes"),
            cpu_cores: number(hw, "cpu_cores"),
            disk_free: number(hw, "disk_free_bytes"),
            models_dir: string(hw, "models_dir"),
            accelerator: string(hw, "accelerator"),
        },
        ollama: Ollama {
            installed: flag(ol, "installed"),
            running: flag(ol, "running"),
            version: string(ol, "version"),
            base_url: string(ol, "base_url"),
            install_command: ol["install"]["command"].as_str().map(str::to_owned),
            install_url: string(&ol["install"], "url"),
            models: list(&ol["models"])
                .iter()
                .map(|m| (string(m, "name"), number(m, "size_bytes")))
                .collect(),
        },
        opencode_installed: flag(oc, "installed"),
        opencode_hint: string(oc, "install_hint"),
        configured: list(&oc["configured_models"])
            .iter()
            .filter_map(|m| m.as_str().map(str::to_owned))
            .collect(),
        context: number(&value, "context"),
        models: list(&value["models"])
            .iter()
            .map(|m| Candidate {
                tag: string(m, "tag"),
                label: string(m, "label"),
                size: number(m, "size_bytes"),
                context_k: number(m, "context_k"),
                note: string(m, "note"),
                fit: string(m, "fit"),
                required: number(m, "required_bytes"),
                disk_ok: flag(m, "disk_ok"),
                summary: string(m, "summary"),
                recommended: flag(m, "recommended"),
                downloaded: flag(m, "downloaded"),
                configured: flag(m, "configured"),
            })
            .filter(|m| !m.tag.is_empty())
            .collect(),
    })
}

/// Hardware, Ollama and the catalog. Blocking: run it off the UI thread.
pub fn snapshot(cli: &Path, project: &Path) -> Result<Snapshot, String> {
    let mut process = command(cli);
    process.args(["bridge", "local-models"]).arg(project);
    let (_, output) = run_short(process, Duration::from_secs(30))?;
    parse_snapshot(&output)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Downloads the model, widens its context and connects it to OpenCode.
    Setup { tag: String, set_default: bool },
    /// Deletes it from Ollama and from the project's `opencode.json`.
    Remove { tag: String },
}

/// A setup or removal running in the CLI, reporting JSON events line by line.
pub struct Job {
    pub op: Op,
    child: Option<Child>,
    events: Receiver<Value>,
    pub exit: Option<i32>,
}

impl Job {
    pub fn start(cli: &Path, project: &Path, op: Op, context: u64) -> Result<Self, String> {
        let mut process = command(cli);
        match &op {
            Op::Setup { tag, set_default } => {
                process
                    .args(["local-models", "setup", tag, "--path"])
                    .arg(project)
                    .args(["--json", "--yes", "--context", &context.to_string()]);
                if *set_default {
                    process.arg("--set-default");
                }
            }
            Op::Remove { tag } => {
                process
                    .args(["local-models", "remove", tag, "--path"])
                    .arg(project)
                    .args(["--json", "--yes"]);
            }
        }
        let mut child = process
            .current_dir(project)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("No pude iniciar el CLI: {error}"))?;
        let stdout = child.stdout.take().ok_or("sin salida")?;
        let (sender, events) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).split(b'\n').map_while(Result::ok) {
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    if sender.send(value).is_err() {
                        break;
                    }
                    super::wake::notify();
                }
            }
            super::wake::notify_after_end();
        });
        Ok(Self {
            op,
            child: Some(child),
            events,
            exit: None,
        })
    }

    pub fn poll(&mut self) -> Vec<Value> {
        let mut found: Vec<Value> = self.events.try_iter().collect();
        if self.exit.is_none()
            && let Some(child) = self.child.as_mut()
            && let Ok(Some(status)) = child.try_wait()
        {
            self.exit = Some(status.code().unwrap_or(-1));
            found.extend(super::chat_bridge::drain_after_exit(&self.events));
        }
        found
    }

    pub fn is_running(&self) -> bool {
        self.exit.is_none()
    }

    /// Stops the work (a download stops when its connection closes).
    pub fn cancel(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if self.exit.is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

/// `7.4 GB` / `820 MB`.
pub fn size_label(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.1} GB", value / GIB)
    } else {
        format!("{:.0} MB", value / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"schema":"chxchx.local-models","schema_version":1,
      "hardware":{"total_ram_bytes":22975639552,"available_ram_bytes":13642752000,"cpu_cores":8,
        "disk_free_bytes":904763838464,"models_dir":"/m","accelerator":"AMD Radeon (2 GB)"},
      "ollama":{"installed":false,"running":false,"version":null,"base_url":"http://127.0.0.1:11434",
        "install":{"command":"curl -fsSL https://ollama.com/install.sh | sh","url":"https://ollama.com/download"},
        "models":[{"name":"qwen2.5-coder:7b","size_bytes":5046586572}],"loaded":[]},
      "opencode":{"installed":true,"install_hint":"npm i","configured_models":["x"]},"context":32768,
      "models":[{"tag":"qwen2.5-coder:7b","label":"Qwen2.5 Coder 7B","size_bytes":5046586572,
        "context_k":32,"note":"n","fit":"partial","required_bytes":7446399548,"disk_ok":true,
        "summary":"s","recommended":true,"downloaded":false,"loaded":false,"configured":false}]}"#;

    #[test]
    fn reads_the_snapshot_of_the_cli() {
        let snapshot = parse_snapshot(SAMPLE).unwrap();
        assert_eq!(snapshot.hardware.cpu_cores, 8);
        assert!(!snapshot.ollama.installed && snapshot.ollama.install_command.is_some());
        assert_eq!(snapshot.ollama.models.len(), 1);
        assert!(snapshot.opencode_installed && snapshot.configured == ["x"]);
        let model = &snapshot.models[0];
        assert!(model.recommended && model.fit == "partial" && model.disk_ok);
    }

    #[test]
    fn a_foreign_answer_is_an_error() {
        assert!(parse_snapshot(r#"{"schema":"otra"}"#).is_err());
        assert!(parse_snapshot("no json").is_err());
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(size_label(5_046_586_572), "4.7 GB");
        assert_eq!(size_label(800 * 1024 * 1024), "800 MB");
    }
}
