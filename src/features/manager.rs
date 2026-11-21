//! The manager (ADR 0017, H6): what this machine has, what is missing and what would fix it.
//!
//! It is pure: it takes `Facts` (read by `integrations::manager_probe`) and returns an inventory and
//! a plan. Nothing here touches the machine, so the same facts always give the same plan and the
//! plan can be shown before anything is done. Only changes that are safe to repeat and that belong to
//! ChxChx itself are marked `Auto`; everything else is explained and left to the person.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    pub claude: Option<String>,
    pub codex: Option<String>,
    pub opencode: Option<String>,
    /// A Python 3.11+ (what the Codex/OpenCode service runs on).
    pub python: Option<String>,
    /// The agent service, once unpacked or installed.
    pub agent_service: Option<String>,
    /// `(port, version)` of the node when it answers.
    pub node: Option<(u16, String)>,
    pub node_service_installed: bool,
    pub tailscale: Option<String>,
    pub memory_percent: Option<f64>,
    /// Free space where ChxChx keeps its files.
    pub disk_free_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Ready,
    /// Not there; fine if the person does not need it.
    Missing,
    Attention,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: &'static str,
    pub label: &'static str,
    pub health: Health,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auto {
    /// Unpack the agent service that travels inside the program.
    UnpackAgentService,
    /// Start the node at login, as a user service.
    InstallNodeService,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fix {
    Auto(Auto),
    /// What the person has to do; ChxChx does not do it.
    Manual(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub item: &'static str,
    pub why: String,
    pub fix: Fix,
}

fn item(id: &'static str, label: &'static str, health: Health, detail: impl Into<String>) -> Item {
    Item {
        id,
        label,
        health,
        detail: detail.into(),
    }
}

fn found(id: &'static str, label: &'static str, path: &Option<String>, missing: &str) -> Item {
    match path {
        Some(path) => item(id, label, Health::Ready, path.clone()),
        None => item(id, label, Health::Missing, missing),
    }
}

/// How many agent turns the node runs at once on this machine: about one per 1.5 GiB of free
/// memory and at most one per two CPUs, never fewer than 1 nor more than 4. It only chooses a number
/// for the node's own queue; it touches no setting of the operating system.
pub fn suggested_parallel_jobs(available_bytes: u64, cpus: usize) -> usize {
    const PER_JOB: u64 = 1_536 * 1024 * 1024;
    let by_memory = usize::try_from(available_bytes / PER_JOB).unwrap_or(usize::MAX);
    by_memory.min(cpus / 2).clamp(1, 4)
}

/// The value the node uses: the suggestion for this machine, or 2 when the machine cannot be read.
pub fn parallel_jobs_here() -> usize {
    let system = crate::features::workspace_resources::read_system_resources();
    if system.total_bytes == 0 {
        return 2;
    }
    let cpus = std::thread::available_parallelism().map_or(2, std::num::NonZero::get);
    suggested_parallel_jobs(system.available_bytes, cpus)
}

pub fn inventory(facts: &Facts) -> Vec<Item> {
    let mut items = vec![
        found("claude", "Claude Code", &facts.claude, "no encontrado"),
        found("codex", "Codex", &facts.codex, "no encontrado"),
        found("opencode", "OpenCode", &facts.opencode, "no encontrado"),
        found("python", "Python 3.11+", &facts.python, "no encontrado"),
    ];
    items.push(match &facts.agent_service {
        Some(path) => item(
            "agent-service",
            "Servicio de Codex y OpenCode",
            Health::Ready,
            path.clone(),
        ),
        None => item(
            "agent-service",
            "Servicio de Codex y OpenCode",
            Health::Missing,
            "aún no desempaquetado (viene dentro del programa)",
        ),
    });
    items.push(match &facts.node {
        Some((port, version)) => item(
            "node",
            "Nodo local",
            Health::Ready,
            format!("en marcha en 127.0.0.1:{port}, versión {version}"),
        ),
        None => item("node", "Nodo local", Health::Missing, "detenido"),
    });
    items.push(item(
        "node-service",
        "Arranque del nodo al iniciar sesión",
        if facts.node_service_installed {
            Health::Ready
        } else {
            Health::Missing
        },
        if facts.node_service_installed {
            "instalado"
        } else {
            "no instalado"
        },
    ));
    items.push(found(
        "tailscale",
        "Tailscale (acceso desde el celular)",
        &facts.tailscale,
        "no encontrado",
    ));
    if let Some(percent) = facts.memory_percent {
        items.push(item(
            "memory",
            "Memoria",
            if percent >= 90.0 {
                Health::Attention
            } else {
                Health::Ready
            },
            format!("{percent:.0} % en uso"),
        ));
    }
    if let Some(free) = facts.disk_free_bytes {
        items.push(item(
            "disk",
            "Espacio en disco",
            if free < LOW_DISK {
                Health::Attention
            } else {
                Health::Ready
            },
            format!("{:.1} GiB libres", free as f64 / (1024.0 * 1024.0 * 1024.0)),
        ));
    }
    items
}

/// Below this, downloads and agent work start to fail in confusing ways.
pub const LOW_DISK: u64 = 2 * 1024 * 1024 * 1024;

/// What would leave the machine ready for the agents the person has. Empty when nothing is needed.
pub fn plan(facts: &Facts) -> Vec<Step> {
    let mut steps = Vec::new();
    let needs_service = facts.codex.is_some() || facts.opencode.is_some();
    if needs_service && facts.python.is_none() {
        steps.push(Step {
            item: "python",
            why: "Codex y OpenCode se conducen con un servicio en Python 3.11 o más nuevo.".into(),
            fix: Fix::Manual(
                "Instala Python 3.11+ con el gestor de tu sistema (p. ej. `sudo dnf install python3`, \
                 `sudo apt install python3`, `brew install python`, `winget install Python.Python.3.12`)."
                    .into(),
            ),
        });
    } else if needs_service && facts.agent_service.is_none() {
        steps.push(Step {
            item: "agent-service",
            why: "Codex y OpenCode necesitan su servicio; viene dentro del programa.".into(),
            fix: Fix::Auto(Auto::UnpackAgentService),
        });
    }
    if facts.node.is_none() && !facts.node_service_installed {
        steps.push(Step {
            item: "node-service",
            why: "El nodo mantiene tus agentes aunque cierres Studio y los deja ver desde el celular."
                .into(),
            fix: Fix::Auto(Auto::InstallNodeService),
        });
    } else if facts.node.is_none() {
        steps.push(Step {
            item: "node",
            why: "El servicio está instalado pero el nodo no responde.".into(),
            fix: Fix::Manual(
                "Mira `systemctl --user status chxchx-node` (Linux) o `chxchx-tech node start` para ver el error."
                    .into(),
            ),
        });
    }
    if facts.tailscale.is_none() {
        steps.push(Step {
            item: "tailscale",
            why: "Sin Tailscale el nodo solo se ve desde esta máquina (opcional).".into(),
            fix: Fix::Manual(
                "Instálalo desde https://tailscale.com/download e inicia sesión.".into(),
            ),
        });
    }
    if facts.memory_percent.is_some_and(|percent| percent >= 90.0) {
        steps.push(Step {
            item: "memory",
            why: "La memoria está casi llena; los agentes pueden quedarse sin recursos.".into(),
            fix: Fix::Manual("Cierra procesos pesados antes de lanzar más turnos.".into()),
        });
    }
    if facts.disk_free_bytes.is_some_and(|free| free < LOW_DISK) {
        steps.push(Step {
            item: "disk",
            why: "Queda poco espacio libre (menos de 2 GiB).".into(),
            fix: Fix::Manual("Libera espacio; ChxChx no borra tus archivos.".into()),
        });
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> Facts {
        Facts {
            claude: Some("/bin/claude".into()),
            codex: Some("/bin/codex".into()),
            opencode: None,
            python: Some("/bin/python3".into()),
            agent_service: Some("/x/chxchx-agent-adapter".into()),
            node: Some((47821, "0.6.3".into())),
            node_service_installed: true,
            tailscale: Some("/bin/tailscale".into()),
            memory_percent: Some(40.0),
            disk_free_bytes: Some(50 * 1024 * 1024 * 1024),
        }
    }

    #[test]
    fn a_machine_that_has_everything_needs_nothing() {
        assert_eq!(plan(&ready()), vec![]);
    }

    #[test]
    fn the_same_facts_always_give_the_same_plan() {
        let facts = Facts::default();
        assert_eq!(plan(&facts), plan(&facts));
    }

    #[test]
    fn the_service_is_unpacked_only_when_codex_or_opencode_exist() {
        let mut facts = ready();
        facts.agent_service = None;
        assert_eq!(plan(&facts)[0].fix, Fix::Auto(Auto::UnpackAgentService));
        facts.codex = None;
        assert_eq!(plan(&facts), vec![], "only Claude: no service needed");
    }

    #[test]
    fn without_python_the_person_is_told_and_nothing_is_attempted() {
        let mut facts = ready();
        facts.python = None;
        facts.agent_service = None;
        let steps = plan(&facts);
        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0].fix, Fix::Manual(_)));
    }

    #[test]
    fn a_stopped_node_gets_a_service_unless_it_already_has_one() {
        let mut facts = ready();
        facts.node = None;
        facts.node_service_installed = false;
        assert_eq!(plan(&facts)[0].fix, Fix::Auto(Auto::InstallNodeService));
        facts.node_service_installed = true;
        assert!(matches!(plan(&facts)[0].fix, Fix::Manual(_)));
    }

    #[test]
    fn only_changes_that_belong_to_chxchx_are_automatic() {
        let steps = plan(&Facts {
            codex: Some("c".into()),
            ..Facts::default()
        });
        for step in &steps {
            if let Fix::Auto(auto) = step.fix {
                assert!(matches!(
                    auto,
                    Auto::UnpackAgentService | Auto::InstallNodeService
                ));
            }
        }
        // Tailscale and Python are never installed by the program.
        assert!(
            steps
                .iter()
                .filter(|s| s.item == "tailscale")
                .all(|s| matches!(s.fix, Fix::Manual(_)))
        );
    }

    #[test]
    fn concurrency_follows_memory_and_cpus_within_bounds() {
        const GIB: u64 = 1024 * 1024 * 1024;
        assert_eq!(suggested_parallel_jobs(0, 16), 1, "never below one");
        assert_eq!(suggested_parallel_jobs(2 * GIB, 16), 1);
        assert_eq!(suggested_parallel_jobs(8 * GIB, 4), 2, "cpus limit it");
        assert_eq!(suggested_parallel_jobs(64 * GIB, 64), 4, "never above four");
        assert_eq!(suggested_parallel_jobs(64 * GIB, 1), 1);
    }

    #[test]
    fn little_disk_is_flagged_and_never_cleaned_by_the_program() {
        let mut facts = ready();
        facts.disk_free_bytes = Some(500 * 1024 * 1024);
        let steps = plan(&facts);
        assert_eq!(steps[0].item, "disk");
        assert!(matches!(steps[0].fix, Fix::Manual(_)));
    }

    #[test]
    fn full_memory_is_flagged() {
        let mut facts = ready();
        facts.memory_percent = Some(95.0);
        assert!(
            inventory(&facts)
                .iter()
                .any(|i| i.id == "memory" && i.health == Health::Attention)
        );
        assert_eq!(plan(&facts)[0].item, "memory");
    }
}
