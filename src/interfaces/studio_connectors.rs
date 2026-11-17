//! MCP connectors in Studio: give the agents hands in Excel, Power BI, databases… from a card.
//! Adding or removing one shows the exact change first; nothing is written until it is applied.
use super::*;
use crate::features::mcp_connectors::{Connector, connectors};
use crate::features::mcp_connectors_plan::{
    ConnectorState, ConnectorsApplication, ConnectorsChange, ConnectorsPlan, Standing,
    apply_connectors, connector_states, default_clients, plan_connectors,
};
use crate::features::mcp_setup::McpClient;
use iced::widget::{column, text_input};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct State {
    pub states: Option<Vec<ConnectorState>>,
    pub loading: bool,
    pub busy: bool,
    /// What is waiting for confirmation.
    pub plan: Option<(String, ConnectorsPlan)>,
    /// Values typed for connector parameters: (connector, parameter) -> value.
    pub values: BTreeMap<(String, String), String>,
    pub custom_name: String,
    pub custom_command: String,
    pub custom_env: String,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Load,
    Param(String, String, String),
    /// Previews adding (or removing, when already there) this connector.
    Toggle(String),
    CustomName(String),
    CustomCommand(String),
    CustomEnv(String),
    PreviewCustom,
    Apply,
    Cancel,
}

fn clients(root: &std::path::Path) -> Vec<McpClient> {
    let found = default_clients(root);
    if found.is_empty() {
        McpClient::parse("all").unwrap_or_default()
    } else {
        found
    }
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    let root = state.root.clone();
    match event {
        Event::Load => {
            if state.connectors.loading {
                return Task::none();
            }
            state.connectors.loading = true;
            return jobs::run(
                state,
                move || connector_states(&root, &McpClient::parse("all").unwrap_or_default()),
                |state, result| {
                    state.connectors.loading = false;
                    match result {
                        Ok(states) => state.connectors.states = Some(states),
                        Err(error) => state.message = error,
                    }
                    Task::none()
                },
            );
        }
        Event::Param(connector, param, value) => {
            state.connectors.values.insert((connector, param), value);
        }
        Event::CustomName(value) => state.connectors.custom_name = value,
        Event::CustomCommand(value) => state.connectors.custom_command = value,
        Event::CustomEnv(value) => state.connectors.custom_env = value,
        Event::Toggle(name) => {
            let Some(c) = crate::features::mcp_connectors::connector(&name) else {
                return Task::none();
            };
            let values: BTreeMap<String, String> = c
                .params
                .iter()
                .map(|p| {
                    let typed = state.connectors.values.get(&(name.clone(), p.name.clone()));
                    (p.name.clone(), typed.cloned().unwrap_or_default())
                })
                .collect();
            let chosen = clients(&root);
            let configured = state.connectors.states.as_ref().is_some_and(|all| {
                all.iter().find(|s| s.name == name).is_some_and(|s| {
                    chosen.iter().all(|client| {
                        s.standing
                            .iter()
                            .any(|(k, st)| k == client && *st == Standing::Configured)
                    })
                })
            });
            let (change, title) = if configured {
                (
                    ConnectorsChange::Remove { names: vec![name] },
                    format!("Quitar {}", c.title),
                )
            } else {
                (
                    ConnectorsChange::Add {
                        names: vec![name],
                        values,
                    },
                    format!("Añadir {}", c.title),
                )
            };
            return preview(state, change, chosen, title);
        }
        Event::PreviewCustom => {
            let words: Vec<String> = state
                .connectors
                .custom_command
                .split_whitespace()
                .map(str::to_owned)
                .collect();
            let env: Vec<String> = state
                .connectors
                .custom_env
                .split([',', ' '])
                .filter(|e| !e.is_empty())
                .map(str::to_owned)
                .collect();
            let change = ConnectorsChange::Custom {
                name: state.connectors.custom_name.trim().to_owned(),
                command: words,
                env,
            };
            let title = format!("Añadir {}", state.connectors.custom_name.trim());
            return preview(state, change, clients(&root), title);
        }
        Event::Cancel => state.connectors.plan = None,
        Event::Apply => {
            let Some((_, plan)) = state.connectors.plan.take() else {
                return Task::none();
            };
            state.connectors.busy = true;
            return jobs::run(state, move || apply_connectors(&plan), applied);
        }
    }
    Task::none()
}

fn preview(
    state: &mut Dashboard,
    change: ConnectorsChange,
    chosen: Vec<McpClient>,
    title: String,
) -> Task<Message> {
    let root = state.root.clone();
    jobs::run(
        state,
        move || plan_connectors(&root, &change, &chosen).map(|plan| (title, plan)),
        |state, result| {
            match result {
                Ok(planned) => state.connectors.plan = Some(planned),
                Err(error) => state.message = error,
            }
            Task::none()
        },
    )
}

fn applied(state: &mut Dashboard, result: Result<ConnectorsApplication, String>) -> Task<Message> {
    state.connectors.busy = false;
    match result {
        Ok(done) => {
            state.message = format!(
                "Conectores aplicados. Reinicia el agente para que cargue los servidores.{}",
                done.backup
                    .map(|b| format!(" Respaldo: {b}"))
                    .unwrap_or_default()
            );
        }
        Err(error) => state.message = error,
    }
    state.connectors.states = None;
    update(state, Event::Load)
}

fn chip<'a>(label: String, tint: iced::Color) -> Element<'a, Message> {
    container(text(label).size(10).font(design::UI_BOLD))
        .padding([2, 8])
        .style(styles::chip(tint))
        .into()
}

fn standing_chips<'a>(state: Option<&ConnectorState>) -> Element<'a, Message> {
    let mut chips = row![].spacing(6);
    for (client, standing) in state.map(|s| s.standing.clone()).unwrap_or_default() {
        let (glyph, tint) = match standing {
            Standing::Configured => ("✓", design::success()),
            Standing::Different => ("≠", design::warning()),
            Standing::Absent => ("·", design::faint()),
        };
        chips = chips.push(chip(format!("{} {glyph}", client.label()), tint));
    }
    chips.into()
}

fn connector_card<'a>(state: &'a Dashboard, c: &'a Connector) -> Element<'a, Message> {
    let found = state
        .connectors
        .states
        .as_ref()
        .and_then(|all| all.iter().find(|s| s.name == c.name));
    let origin_tint = match c.origin.as_str() {
        "oficial" => design::success(),
        "referencia" => design::accent(),
        _ => design::muted(),
    };
    let mut needs: Vec<String> = Vec::new();
    for program in &c.requires {
        let missing = found.is_some_and(|s| s.missing_requirements.contains(program));
        needs.push(format!("{} {program}", if missing { "✗" } else { "✓" }));
    }
    for env in c.required_env() {
        let missing = found.is_some_and(|s| s.missing_env.contains(&env.name));
        needs.push(format!(
            "{} {} (de tu entorno)",
            if missing { "✗" } else { "✓" },
            env.name
        ));
    }
    let configured_everywhere =
        found.is_some_and(|s| s.standing.iter().all(|(_, st)| *st == Standing::Configured));
    let mut body = column![
        row![
            text(&c.title).size(14).font(design::UI_BOLD).width(Fill),
            chip(c.origin.clone(), origin_tint),
            chip(c.category.clone(), design::tint_purple()),
        ]
        .spacing(8)
        .align_y(iced::Center),
        text(&c.description).size(12).color(design::muted()),
        text(format!("Necesita: {}", needs.join("  ·  ")))
            .size(11)
            .color(design::faint()),
    ]
    .spacing(6);
    if !c.note.is_empty() {
        body = body.push(text(&c.note).size(11).color(design::warning()));
    }
    let mut controls = row![standing_chips(found), Space::new().width(Fill)]
        .spacing(8)
        .align_y(iced::Center);
    for param in &c.params {
        let name = c.name.clone();
        let param_name = param.name.clone();
        let current = state
            .connectors
            .values
            .get(&(c.name.clone(), param.name.clone()))
            .cloned()
            .unwrap_or_default();
        controls = controls.push(
            text_input(&param.placeholder, &current)
                .on_input(move |v| {
                    Message::Connectors(Event::Param(name.clone(), param_name.clone(), v))
                })
                .size(12)
                .padding([6, 10])
                .width(220)
                .style(styles::input),
        );
    }
    controls = controls.push(
        button(
            text(if configured_everywhere {
                "Quitar"
            } else {
                "Añadir"
            })
            .size(12),
        )
        .padding([6, 14])
        .style(if configured_everywhere {
            styles::secondary
        } else {
            styles::primary
        })
        .on_press_maybe(
            (!state.connectors.busy && found.is_some())
                .then(|| Message::Connectors(Event::Toggle(c.name.clone()))),
        ),
    );
    container(body.push(controls))
        .padding(14)
        .width(Fill)
        .style(styles::card)
        .into()
}

fn plan_panel<'a>(title: &'a str, plan: &'a ConnectorsPlan, busy: bool) -> Element<'a, Message> {
    let mut lines = column![text(title).size(14).font(design::UI_BOLD)].spacing(6);
    for action in &plan.actions {
        lines = lines.push(text(format!("• {action}")).size(12));
    }
    for warning in &plan.warnings {
        lines = lines.push(
            text(format!("▲ {warning}"))
                .size(11)
                .color(design::warning()),
        );
    }
    if let Some(reason) = &plan.blocked_reason {
        lines = lines.push(
            text(format!("✗ No se puede aplicar: {reason}"))
                .size(12)
                .color(design::danger()),
        );
    }
    for (file, content) in plan.previews() {
        lines = lines.push(text(file).size(11).color(design::muted()));
        lines = lines.push(
            container(
                styles::vscroll(text(content).size(11).font(design::MONO))
                    .height(iced::Length::Fixed(150.0)),
            )
            .padding(8)
            .width(Fill)
            .style(styles::chip_surface(false)),
        );
    }
    let can_apply = plan.changes_state() && plan.blocked_reason.is_none() && !busy;
    lines = lines.push(
        row![
            button(text("Aplicar").size(12))
                .padding([6, 14])
                .style(styles::primary)
                .on_press_maybe(can_apply.then_some(Message::Connectors(Event::Apply))),
            button(text("Cancelar").size(12))
                .padding([6, 14])
                .style(styles::secondary)
                .on_press(Message::Connectors(Event::Cancel)),
        ]
        .spacing(8),
    );
    container(lines)
        .padding(14)
        .width(Fill)
        .style(styles::tinted(design::accent()))
        .into()
}

fn custom_card<'a>(state: &'a Dashboard) -> Element<'a, Message> {
    let field = |placeholder: &'static str, value: &'a str, make: fn(String) -> Event| {
        text_input(placeholder, value)
            .on_input(move |v| Message::Connectors(make(v)))
            .size(12)
            .padding([6, 10])
            .style(styles::input)
    };
    container(
        column![
            text("Servidor MCP propio").size(14).font(design::UI_BOLD),
            text("Añade cualquier servidor que no esté en el catálogo. Las variables de entorno se leen de tu entorno: no se guardan.").size(12).color(design::muted()),
            field("nombre (letras, números, - y _)", &state.connectors.custom_name, Event::CustomName),
            field("comando, por ejemplo: npx -y mi-paquete", &state.connectors.custom_command, Event::CustomCommand),
            field("variables de entorno, separadas por comas (opcional)", &state.connectors.custom_env, Event::CustomEnv),
            button(text("Previsualizar").size(12)).padding([6, 14]).style(styles::secondary).on_press(Message::Connectors(Event::PreviewCustom)),
        ]
        .spacing(8),
    )
    .padding(14)
    .width(Fill)
    .style(styles::card)
    .into()
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let mut list = column![
        pages::eyebrow("CONECTORES MCP"),
        text("Dale manos a tus agentes en tus apps").size(18).font(design::UI_BOLD),
        text("Excel, Power BI, Word, Google Sheets y bases de datos. Se configuran en este proyecto, para cada agente; nunca se guarda una clave.").size(12).color(design::muted()),
    ]
    .spacing(6);
    if let Some((title, plan)) = &state.connectors.plan {
        list = list.push(plan_panel(title, plan, state.connectors.busy));
    }
    if state.connectors.states.is_none() {
        list = list.push(
            text("Leyendo qué hay configurado…")
                .size(12)
                .color(design::muted()),
        );
    }
    for c in connectors() {
        list = list.push(connector_card(state, c));
    }
    list.push(custom_card(state)).spacing(12).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chxchx-studio-conn-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn typed_values_are_kept_per_connector_and_parameter() {
        let mut state = editor::fixture();
        let _ = update(
            &mut state,
            Event::Param("sqlite".into(), "db_path".into(), "./a.db".into()),
        );
        let _ = update(&mut state, Event::CustomName("mi-api".into()));
        let _ = update(&mut state, Event::CustomCommand("npx -y x".into()));
        let _ = update(&mut state, Event::CustomEnv("A, B".into()));
        assert_eq!(
            state.connectors.values[&("sqlite".to_owned(), "db_path".to_owned())],
            "./a.db"
        );
        assert_eq!(state.connectors.custom_name, "mi-api");
        assert_eq!(state.connectors.custom_command, "npx -y x");
    }

    #[test]
    fn the_section_draws_with_nothing_loaded_with_the_states_and_with_a_plan_waiting() {
        let dir = project("view");
        let mut state = editor::fixture();
        state.root = dir.clone();
        let _ = view(&state);
        let all = McpClient::parse("all").unwrap();
        state.connectors.states = Some(connector_states(&dir, &all).unwrap());
        let _ = view(&state);
        let change = ConnectorsChange::Add {
            names: vec!["excel".into()],
            values: BTreeMap::new(),
        };
        let plan = plan_connectors(&dir, &change, &all).unwrap();
        state.connectors.plan = Some(("Añadir Excel".into(), plan));
        let _ = view(&state);
        let _ = update(&mut state, Event::Cancel);
        assert!(
            state.connectors.plan.is_none(),
            "cancelling drops the plan without writing"
        );
        assert!(!dir.join(".mcp.json").exists());
    }

    #[test]
    fn applying_reports_the_result_and_reloads_what_is_configured() {
        let dir = project("apply");
        let mut state = editor::fixture();
        state.root = dir.clone();
        let change = ConnectorsChange::Add {
            names: vec!["excel".into()],
            values: BTreeMap::new(),
        };
        let plan = plan_connectors(&dir, &change, &[McpClient::Claude]).unwrap();
        let done = apply_connectors(&plan).unwrap();
        state.connectors.busy = true;
        state.connectors.states = Some(Vec::new());
        let _ = applied(&mut state, Ok(done));
        assert!(!state.connectors.busy);
        assert!(
            state.message.contains("Reinicia el agente"),
            "{}",
            state.message
        );
        assert!(
            state.connectors.states.is_none(),
            "stale states are dropped while it reloads"
        );
        assert!(dir.join(".mcp.json").is_file());
        let _ = applied(&mut state, Err("falló".into()));
        assert_eq!(state.message, "falló");
    }
}
