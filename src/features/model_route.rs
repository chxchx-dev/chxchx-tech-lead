//! Plan mode for Codex chats: one model thinks the plan through, another one writes it.
//!
//! While the person plans, the turn runs read-only with the planning model. When that model says it
//! has what it needs (or the person asks for the plan), the next turn runs with the writing model and
//! an effort that depends on how complex the work is. A turn cannot change model halfway, so the
//! switch happens between two turns of the same conversation.
//!
//! The two models and their efforts are the product's starting point; whoever wants others writes
//! `<carpeta de ChxChx>/model-route/model-route.json` (a field left out keeps its default and
//! anything invalid is ignored). Nothing here talks to a process: it is the rules, so they can be
//! tested.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const FOLDER: &str = "model-route";
const FILE: &str = "model-route.json";

pub const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max", "ultra"];

/// The line the planning model ends with when it is ready; the rest of the line says how complex.
pub const READY_LINE: &str = "Listo para escribir el plan";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Route {
    /// Reads, reasons and asks while the plan is being worked out.
    pub planner_model: String,
    pub planner_effort: String,
    /// Writes the plan once it is ready.
    pub writer_model: String,
    /// Effort when the work is ordinary, and when it is complex.
    pub writer_effort: String,
    pub writer_deep_effort: String,
}

impl Default for Route {
    fn default() -> Self {
        Self {
            planner_model: "gpt-6.1-terra".into(),
            planner_effort: "high".into(),
            writer_model: "gpt-6-luna".into(),
            writer_effort: "high".into(),
            writer_deep_effort: "xhigh".into(),
        }
    }
}

fn model_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/' | ':'))
}

fn effort(value: &str) -> bool {
    EFFORTS.contains(&value.trim())
}

impl Route {
    /// The same route with every invalid field back to its default.
    pub fn sanitized(&self) -> Self {
        let base = Self::default();
        let model = |mine: &str, fallback: &str| {
            if model_name(mine) {
                mine.trim().to_owned()
            } else {
                fallback.to_owned()
            }
        };
        let level = |mine: &str, fallback: &str| {
            if effort(mine) {
                mine.trim().to_owned()
            } else {
                fallback.to_owned()
            }
        };
        Self {
            planner_model: model(&self.planner_model, &base.planner_model),
            planner_effort: level(&self.planner_effort, &base.planner_effort),
            writer_model: model(&self.writer_model, &base.writer_model),
            writer_effort: level(&self.writer_effort, &base.writer_effort),
            writer_deep_effort: level(&self.writer_deep_effort, &base.writer_deep_effort),
        }
    }
}

fn file(home: &Path) -> PathBuf {
    home.join(FOLDER).join(FILE)
}

pub fn load_in(home: &Path) -> Route {
    fs::read(file(home))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Route>(&bytes).ok())
        .map(|route| route.sanitized())
        .unwrap_or_default()
}

pub fn load() -> Route {
    // Tests never read the file of whoever runs them.
    #[cfg(test)]
    return Route::default();
    #[cfg(not(test))]
    crate::features::project_profiles::application_home()
        .map(|home| load_in(&home))
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Planning,
    Writing,
}

/// How demanding the work is; it only changes the writer's effort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    Normal,
    Deep,
}

/// What a turn runs with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    pub model: String,
    pub effort: String,
}

pub fn pick(route: &Route, phase: Phase, depth: Depth) -> Pick {
    match phase {
        Phase::Planning => Pick {
            model: route.planner_model.clone(),
            effort: route.planner_effort.clone(),
        },
        Phase::Writing => Pick {
            model: route.writer_model.clone(),
            effort: match depth {
                Depth::Normal => route.writer_effort.clone(),
                Depth::Deep => route.writer_deep_effort.clone(),
            },
        },
    }
}

fn plain(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            other => other,
        })
        .collect()
}

/// The planning model said it is ready, and how complex the work is. Only the last lines count, so a
/// plan that merely quotes the sentence does not switch anything.
pub fn ready(reply: &str) -> Option<Depth> {
    let key = plain(READY_LINE);
    let line = reply
        .lines()
        .rev()
        .take(3)
        .map(plain)
        .find(|line| line.contains(&key))?;
    let deep = ["alta", "profunda", "compleja", "xhigh"]
        .iter()
        .any(|word| line.contains(word));
    Some(if deep { Depth::Deep } else { Depth::Normal })
}

/// The person asks for the plan to be written now.
pub fn asks_for_plan(text: &str) -> bool {
    let text = plain(text);
    [
        "escribe el plan",
        "escribeme el plan",
        "escribelo",
        "redacta el plan",
        "dame el plan",
        "armame el plan",
        "arma el plan",
        "genera el plan",
        "plan final",
        "ya escribe",
        "write the plan",
    ]
    .iter()
    .any(|phrase| text.contains(phrase))
}

/// A guess at the work's complexity from what the person wrote, for when they ask for the plan
/// before the planning model said how complex it is.
pub fn depth_of(text: &str) -> Depth {
    let text = plain(text);
    let words = [
        "arquitectura",
        "migracion",
        "seguridad",
        "refactor",
        "concurrencia",
        "base de datos",
        "rendimiento",
        "produccion",
        "compatib",
    ];
    if text.chars().count() > 600 || words.iter().any(|word| text.contains(word)) {
        Depth::Deep
    } else {
        Depth::Normal
    }
}

pub const PLANNING: &str = "[Modo plan: estás planeando, no implementando] Lee lo que haga falta del proyecto y razona; \
si algo es ambiguo, pregunta. NO edites archivos ni ejecutes cambios. Cuando tengas lo necesario para escribir el plan, \
termina tu respuesta con esta línea exacta y nada después: «Listo para escribir el plan · complejidad: normal» o \
«Listo para escribir el plan · complejidad: alta» (alta = toca muchas partes, hay riesgo para datos, seguridad o \
migraciones, o queda mucha incertidumbre).";

pub const WRITING: &str = "[Modo plan: escribe el plan] Escríbelo completo y listo para ejecutar: objetivo, pasos \
numerados con los archivos concretos, riesgos y cómo verificar cada paso. Es solo el plan: no edites archivos.";

/// What the hidden turn that follows a «listo» says.
pub const WRITE_NOW: &str = "Ya tienes lo necesario. Escribe ahora el plan completo.";

/// What one turn of a plan-mode chat runs with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnRoute {
    pub phase: Phase,
    pub pick: Pick,
    /// Added to what the agent reads (never to what the transcript shows).
    pub instructions: &'static str,
}

/// Where a chat is in plan mode.
#[derive(Clone, Debug)]
pub struct PlanState {
    next: Phase,
    running: Option<Phase>,
    last: Option<Pick>,
}

impl Default for PlanState {
    fn default() -> Self {
        Self {
            next: Phase::Planning,
            running: None,
            last: None,
        }
    }
}

impl PlanState {
    /// Decides how the turn that is about to start runs. `follow_up` is the complexity the planning
    /// model gave when it said it was ready (the automatic turn that writes the plan).
    pub fn begin(&mut self, route: &Route, text: &str, follow_up: Option<Depth>) -> TurnRoute {
        let (phase, depth) = match follow_up {
            Some(depth) => (Phase::Writing, depth),
            None if self.next == Phase::Planning && asks_for_plan(text) => {
                (Phase::Writing, depth_of(text))
            }
            None => (self.next, Depth::Normal),
        };
        let pick = pick(route, phase, depth);
        self.running = Some(phase);
        self.last = Some(pick.clone());
        TurnRoute {
            phase,
            pick,
            instructions: match phase {
                Phase::Planning => PLANNING,
                Phase::Writing => WRITING,
            },
        }
    }

    /// A turn ended well. After planning, a «listo» asks for the writing turn (returned);
    /// after writing, the chat goes back to planning so the plan can be refined.
    pub fn end(&mut self, replies: &[String]) -> Option<Depth> {
        match self.running.take() {
            Some(Phase::Planning) => replies.iter().rev().find_map(|reply| ready(reply)),
            Some(Phase::Writing) => {
                self.next = Phase::Planning;
                None
            }
            None => None,
        }
    }

    /// A turn failed or was cancelled: nothing changes.
    pub fn abort(&mut self) {
        self.running = None;
    }

    /// What the last turn ran with, for the chat header.
    pub fn last(&self) -> Option<&Pick> {
        self.last.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_terra_planning_and_luna_writing() {
        let route = Route::default();
        assert_eq!(
            pick(&route, Phase::Planning, Depth::Deep),
            Pick {
                model: "gpt-6.1-terra".into(),
                effort: "high".into()
            },
            "planear no cambia con la complejidad"
        );
        assert_eq!(pick(&route, Phase::Writing, Depth::Normal).effort, "high");
        let deep = pick(&route, Phase::Writing, Depth::Deep);
        assert_eq!(
            (deep.model.as_str(), deep.effort.as_str()),
            ("gpt-6-luna", "xhigh")
        );
    }

    #[test]
    fn a_file_overrides_only_what_it_says_and_ignores_what_is_invalid() {
        let route: Route = serde_json::from_str(
            r#"{"writer_model":"mi-modelo","planner_effort":"ultra","writer_effort":"altísimo","planner_model":"con espacios y ¡!"}"#,
        )
        .unwrap();
        let route = route.sanitized();
        assert_eq!(route.writer_model, "mi-modelo");
        assert_eq!(route.planner_effort, "ultra");
        assert_eq!(
            route.writer_effort, "high",
            "un esfuerzo inválido vuelve al de siempre"
        );
        assert_eq!(route.planner_model, "gpt-6.1-terra");
        assert_eq!(route.writer_deep_effort, "xhigh");
        let dir = std::env::temp_dir().join(format!("chxchx-route-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(
            load_in(&dir),
            Route::default(),
            "sin archivo, los de siempre"
        );
        fs::create_dir_all(dir.join(FOLDER)).unwrap();
        fs::write(file(&dir), "no es json").unwrap();
        assert_eq!(load_in(&dir), Route::default());
        fs::write(file(&dir), r#"{"writer_model":"otro"}"#).unwrap();
        assert_eq!(load_in(&dir).writer_model, "otro");
    }

    #[test]
    fn the_ready_line_is_read_from_the_end_and_says_how_complex() {
        assert_eq!(
            ready("Revisé todo.\n\nListo para escribir el plan · complejidad: normal"),
            Some(Depth::Normal)
        );
        assert_eq!(
            ready("Hay mucho riesgo.\nListo para escribir el plan · complejidad: alta"),
            Some(Depth::Deep)
        );
        assert_eq!(
            ready("LISTO PARA ESCRIBIR EL PLAN - complejidad: Alta\n"),
            Some(Depth::Deep)
        );
        assert_eq!(ready("Aún tengo una duda: ¿usas Postgres?"), None);
        let quoted = "Cuando esté todo diré «Listo para escribir el plan».\nuno\ndos\ntres\ncuatro";
        assert_eq!(
            ready(quoted),
            None,
            "una cita lejos del final no cambia nada"
        );
    }

    #[test]
    fn asking_for_the_plan_is_noticed_with_or_without_accents() {
        for text in [
            "Escribe el plan",
            "ya escríbelo",
            "dame el plan final por favor",
            "Redacta el plan.",
        ] {
            assert!(asks_for_plan(text), "{text}");
        }
        assert!(!asks_for_plan("¿Qué plan de pagos tiene el usuario?"));
        assert_eq!(depth_of("cambia el color del botón"), Depth::Normal);
        assert_eq!(depth_of("migración de la base de datos"), Depth::Deep);
        assert_eq!(depth_of(&"x".repeat(700)), Depth::Deep);
    }

    #[test]
    fn a_chat_plans_then_writes_then_goes_back_to_planning() {
        let route = Route::default();
        let mut plan = PlanState::default();
        let first = plan.begin(&route, "quiero migrar el login", None);
        assert_eq!(
            (first.phase, first.pick.model.as_str()),
            (Phase::Planning, "gpt-6.1-terra")
        );
        assert!(first.instructions.contains("NO edites"));
        // The planning model is not ready yet: nothing follows.
        assert_eq!(plan.end(&["¿Qué base usas?".into()]), None);
        let second = plan.begin(&route, "Postgres", None);
        assert_eq!(second.phase, Phase::Planning);
        let follow = plan.end(&["Listo para escribir el plan · complejidad: alta".into()]);
        assert_eq!(follow, Some(Depth::Deep));
        let writing = plan.begin(&route, WRITE_NOW, follow);
        assert_eq!(writing.phase, Phase::Writing);
        assert_eq!(
            (writing.pick.model.as_str(), writing.pick.effort.as_str()),
            ("gpt-6-luna", "xhigh")
        );
        assert_eq!(plan.end(&["El plan: …".into()]), None);
        let again = plan.begin(&route, "ajusta el paso 3", None);
        assert_eq!(
            again.phase,
            Phase::Planning,
            "después de escribir se vuelve a planear"
        );
        assert_eq!(plan.last().unwrap().model, "gpt-6.1-terra");
    }

    #[test]
    fn asking_for_the_plan_goes_straight_to_the_writer_and_a_failed_turn_changes_nothing() {
        let route = Route::default();
        let mut plan = PlanState::default();
        let direct = plan.begin(&route, "escribe el plan de la arquitectura nueva", None);
        assert_eq!(direct.phase, Phase::Writing);
        assert_eq!(
            direct.pick.effort, "xhigh",
            "la arquitectura es trabajo complejo"
        );
        plan.abort();
        let retry = plan.begin(&route, "escribe el plan", None);
        assert_eq!(
            (retry.phase, retry.pick.effort.as_str()),
            (Phase::Writing, "high")
        );
        assert_eq!(plan.end(&[]), None);
        assert_eq!(plan.begin(&route, "hola", None).phase, Phase::Planning);
    }
}
