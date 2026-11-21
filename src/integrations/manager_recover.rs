//! What the manager may fix on its own (ADR 0017, H6), and the loop that keeps things alive.
//!
//! The list is closed: restart a process ChxChx itself started (the node), remove copies of the
//! agent service that ChxChx itself unpacked and are out of date, and unpack it again if the copy in
//! use stopped answering. Nothing else is touched without the person asking: not their settings,
//! not their projects, not other programs.

use super::{agent_adapter, agent_adapter_bundle, node_link, node_process};
use crate::features::project_profiles::application_home;
use crate::features::supervision::{self, Breaker, Outcome, Target};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn root() -> Result<PathBuf, String> {
    application_home().ok_or_else(|| "No se pudo resolver la carpeta de ChxChx.".to_owned())
}

fn now() -> u64 {
    crate::features::jobs::store::now_ms()
}

/// Old copies of the agent service that this build no longer uses.
pub fn stale_copies(base: &Path) -> Vec<PathBuf> {
    let current = agent_adapter_bundle::current_name();
    let Ok(entries) = std::fs::read_dir(base.join("agent-adapter")) else {
        return Vec::new();
    };
    let mut stale: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir() && entry.file_name().to_string_lossy() != current)
        .map(|entry| entry.path())
        .collect();
    stale.sort();
    stale
}

/// Throws away the copy in use and unpacks it again.
pub fn recreate_adapter(base: &Path) -> Result<PathBuf, String> {
    let folder = base
        .join("agent-adapter")
        .join(agent_adapter_bundle::current_name());
    if folder.is_dir() {
        std::fs::remove_dir_all(&folder)
            .map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    agent_adapter_bundle::ensure()
}

/// Whether the agent service in use answers its contract. Nobody who has neither Codex nor
/// OpenCode needs it, so for them it is fine.
fn adapter_works() -> bool {
    if super::executable_search::find_executable("codex").is_none()
        && super::executable_search::find_executable("opencode").is_none()
    {
        return true;
    }
    agent_adapter::known()
        .or_else(|| agent_adapter::locate(None))
        .is_some_and(|path| agent_adapter::describe(&path).is_ok())
}

struct NodeTarget;
impl Target for NodeTarget {
    fn name(&self) -> &'static str {
        "El nodo"
    }
    fn healthy(&self) -> bool {
        node_link::endpoint().is_some()
    }
    fn restart(&self) -> Result<(), String> {
        node_process::start()
    }
}

struct AdapterTarget(PathBuf);
impl Target for AdapterTarget {
    fn name(&self) -> &'static str {
        "El servicio de Codex y OpenCode"
    }
    fn healthy(&self) -> bool {
        adapter_works()
    }
    fn restart(&self) -> Result<(), String> {
        recreate_adapter(&self.0).map(|_| ())
    }
}

fn state_file(base: &Path) -> PathBuf {
    base.join("manager").join("restarts.json")
}

fn load(base: &Path) -> (Vec<u64>, Vec<u64>) {
    let value: serde_json::Value = std::fs::read_to_string(state_file(base))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let read = |key: &str| {
        value[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_u64)
            .collect()
    };
    (read("node"), read("adapter"))
}

fn save(base: &Path, node: &Breaker, adapter: &Breaker) {
    let file = state_file(base);
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let text = serde_json::json!({ "node": node.times(), "adapter": adapter.times() });
    let _ = std::fs::write(file, text.to_string());
}

fn log(base: &Path, line: &str) {
    use std::io::Write;
    let file = base.join("manager").join("manager.log");
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut out) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
    {
        let _ = writeln!(out, "{} {line}", now());
    }
}

/// Looks after the node and the agent service until stopped. `once` makes a single pass.
/// Returns `Err` with the explanation when it gave up on something.
pub fn watch(once: bool, interval: Duration, say: &dyn Fn(&str)) -> Result<(), String> {
    let base = root()?;
    let targets: [Box<dyn Target>; 2] =
        [Box::new(NodeTarget), Box::new(AdapterTarget(base.clone()))];
    let (node_times, adapter_times) = load(&base);
    let mut breakers = [
        Breaker::from_times(node_times),
        Breaker::from_times(adapter_times),
    ];
    let mut down_since = [None, None];
    loop {
        for (index, target) in targets.iter().enumerate() {
            let outcome = supervision::step(
                target.as_ref(),
                &mut breakers[index],
                &mut down_since[index],
                now(),
            );
            let line = match &outcome {
                Outcome::Healthy => None,
                Outcome::Waiting => Some(format!(
                    "{}: caído, esperando antes de reintentar.",
                    target.name()
                )),
                Outcome::Restarted => Some(format!("{}: se volvió a levantar.", target.name())),
                Outcome::RestartFailed(error) => {
                    Some(format!("{}: no se pudo levantar: {error}", target.name()))
                }
                Outcome::Tripped(why) => Some(why.clone()),
            };
            if let Some(line) = line {
                say(&line);
                log(&base, &line);
            }
            save(&base, &breakers[0], &breakers[1]);
            if let Outcome::Tripped(why) = outcome {
                return Err(why);
            }
        }
        if once {
            return Ok(());
        }
        std::thread::sleep(interval);
    }
}

/// The closed list of things fixed on request. With `dry_run` it only says what it would do.
pub fn recover(dry_run: bool) -> Result<Vec<String>, String> {
    let base = root()?;
    let mut done = Vec::new();
    for stale in stale_copies(&base) {
        if dry_run {
            done.push(format!("Borraría la copia vieja {}", stale.display()));
        } else {
            std::fs::remove_dir_all(&stale)
                .map_err(|error| format!("{}: {error}", stale.display()))?;
            done.push(format!("Borrada la copia vieja {}", stale.display()));
        }
    }
    if !adapter_works() {
        if dry_run {
            done.push("Volvería a desempaquetar el servicio de Codex y OpenCode".to_owned());
        } else {
            let path = recreate_adapter(&base)?;
            done.push(format!(
                "Servicio de agentes recreado en {}",
                path.display()
            ));
        }
    }
    if done.is_empty() {
        done.push("No hay nada que reparar.".to_owned());
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_copies_this_build_does_not_use_are_stale() {
        let base = std::env::temp_dir().join(format!("chxchx-stale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let current = base
            .join("agent-adapter")
            .join(agent_adapter_bundle::current_name());
        let old = base.join("agent-adapter").join("0000000000000000");
        std::fs::create_dir_all(&current).unwrap();
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(base.join("agent-adapter").join("loose-file"), "x").unwrap();
        assert_eq!(stale_copies(&base), vec![old]);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn restart_counts_survive_a_restart_of_the_watcher() {
        let base = std::env::temp_dir().join(format!("chxchx-restarts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        save(
            &base,
            &Breaker::from_times(vec![1, 2]),
            &Breaker::from_times(vec![9]),
        );
        assert_eq!(load(&base), (vec![1, 2], vec![9]));
        let _ = std::fs::remove_dir_all(&base);
    }
}
