//! Agents of the project in one place: who is available, and the ways to open each one.
//!
//! Every agent opens in a terminal of the dock (its own interface, with keyboard, colors and
//! scrollback) from the sidebar panel or from the home page.
use super::pages::eyebrow;
use super::*;
use crate::features::agents::{self as catalog, Agent, AgentCatalog, Provider};
use iced::widget::{column, tooltip};

#[derive(Default)]
pub(super) struct State {
    pub catalog: AgentCatalog,
    pub loaded: bool,
    /// An agent whose chat opens as soon as the agents are known (`CHXCHX_STUDIO_CHAT`).
    pub auto_chat: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Refresh,
    /// Opens the agent (by id) in a terminal tab, or goes back to the one it already has.
    Terminal(String),
    /// Opens the agent's visual chat in the editor area.
    Chat(String),
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Refresh => refresh(state),
        Event::Chat(id) => match find(state, &id) {
            Some(agent) => Task::done(Message::Chat(chat::Event::Open(agent))),
            None => Task::none(),
        },
        Event::Terminal(id) => {
            let Some(agent) = find(state, &id) else {
                return Task::none();
            };
            Task::done(Message::Terminal(terminal::Event::OpenAgent(agent)))
        }
    }
}

fn find(state: &Dashboard, id: &str) -> Option<Agent> {
    state
        .agents
        .catalog
        .agents
        .iter()
        .find(|agent| agent.id == id)
        .cloned()
}

/// Reads the configuration and looks for the agents on this machine, off the UI thread.
pub(super) fn refresh(state: &mut Dashboard) -> Task<Message> {
    let root = state.root.clone();
    jobs::run(
        state,
        move || catalog::list(&root),
        |state, found| {
            state.agents.catalog = found;
            state.agents.loaded = true;
            match state.agents.auto_chat.take() {
                Some(id) if id == "local" => Task::done(Message::Local(local::Event::Open)),
                Some(id) => Task::done(Message::Agents(Event::Chat(id))),
                None => Task::none(),
            }
        },
    )
}

/// `(available, total)` for the status bar and the home page.
pub(super) fn counts(state: &Dashboard) -> (usize, usize) {
    let agents = &state.agents.catalog.agents;
    (
        agents.iter().filter(|a| a.available()).count(),
        agents.len(),
    )
}

fn trusted(state: &Dashboard) -> bool {
    state.overview.as_ref().is_ok_and(|p| p.workspace.trusted)
}

/// Color and initial that make each provider recognizable at a glance.
pub(super) fn identity(provider: Provider) -> (iced::Color, &'static str) {
    match provider {
        Provider::Claude => (design::TINT_ORANGE, "C"),
        Provider::Codex => (design::TINT_GREEN, "X"),
        Provider::OpenCode => (design::TINT_PURPLE, "O"),
        Provider::Other => (design::BRAND_CYAN, "A"),
    }
}

pub(super) fn avatar<'a>(provider: Provider, size: f32) -> Element<'a, Message> {
    let (tint, initial) = identity(provider);
    container(
        text(initial)
            .size(size * 0.5)
            .font(design::UI_BOLD)
            .color(tint),
    )
    .center(size)
    .width(size)
    .height(size)
    .style(styles::chip(tint))
    .into()
}

enum Standing {
    Running,
    Ready,
    Missing,
}

fn standing(state: &Dashboard, agent: &Agent) -> Standing {
    if state.terminal.agent_tab(&agent.id).is_some() {
        Standing::Running
    } else if agent.available() {
        Standing::Ready
    } else {
        Standing::Missing
    }
}

fn status_chip<'a>(standing: &Standing) -> Element<'a, Message> {
    let (label, tint) = match standing {
        Standing::Running => ("En terminal", design::ACCENT),
        Standing::Ready => ("Disponible", design::SUCCESS),
        Standing::Missing => ("No instalado", design::WARNING),
    };
    container(text(label).size(10).font(design::UI_BOLD))
        .padding([2, 8])
        .style(styles::chip(tint))
        .into()
}

fn chat_button<'a>(state: &Dashboard, agent: &Agent) -> Element<'a, Message> {
    let visual = agent.provider.has_visual_chat();
    let enabled = visual && agent.available() && trusted(state);
    let open = state.chat.has_chat(&agent.id);
    let why = if !visual {
        "Este agente solo se abre en la terminal."
    } else if !agent.available() {
        "Instala el agente para poder abrirlo."
    } else if !trusted(state) {
        "Concede confianza al proyecto para hablar con agentes."
    } else {
        "Conversación visual con permisos, uso de tokens y liberación de contexto."
    };
    styles::tip(
        button(
            row![
                icons::icon("sparkle", design::TEXT, 13.0),
                text(if open { "Ver chat" } else { "Chat" }).size(12)
            ]
            .spacing(7)
            .align_y(iced::Center),
        )
        .padding([6, 12])
        .style(styles::primary)
        .on_press_maybe(enabled.then(|| Message::Agents(Event::Chat(agent.id.clone())))),
        why,
        tooltip::Position::Top,
    )
}

fn open_button<'a>(state: &Dashboard, agent: &Agent) -> Element<'a, Message> {
    row![terminal_button(state, agent), chat_button(state, agent)]
        .spacing(8)
        .into()
}

fn terminal_button<'a>(state: &Dashboard, agent: &Agent) -> Element<'a, Message> {
    let running = state.terminal.agent_tab(&agent.id).is_some();
    let label = if running { "Ver terminal" } else { "Terminal" };
    let enabled = agent.available() && trusted(state) && !state.terminal.opening;
    let why = if !agent.available() {
        "Instala el agente para poder abrirlo."
    } else if !trusted(state) {
        "Concede confianza al proyecto para abrir agentes."
    } else {
        "Abre el agente con su propia interfaz en el panel de abajo."
    };
    styles::tip(
        button(
            row![
                icons::icon("terminal", design::TEXT, 13.0),
                text(label).size(12)
            ]
            .spacing(7)
            .align_y(iced::Center),
        )
        .padding([6, 12])
        .style(styles::secondary)
        .on_press_maybe(enabled.then(|| Message::Agents(Event::Terminal(agent.id.clone())))),
        why,
        tooltip::Position::Top,
    )
}

fn agent_card<'a>(state: &Dashboard, agent: &Agent) -> Element<'a, Message> {
    let standing = standing(state, agent);
    let origin = if agent.configured {
        "configurado en el proyecto"
    } else {
        "detectado en este equipo"
    };
    let mut body = column![
        row![
            avatar(agent.provider, 30.0),
            text(agent.label())
                .size(13)
                .font(design::UI_BOLD)
                .width(Fill),
            status_chip(&standing),
        ]
        .spacing(10)
        .align_y(iced::Center),
        container(
            text(format!("{} · {origin}", agent.command.join(" ")))
                .size(10)
                .font(design::MONO)
                .color(design::FAINT)
                .wrapping(iced::widget::text::Wrapping::None),
        )
        .width(Fill)
        .clip(true),
    ]
    .spacing(8);
    if matches!(standing, Standing::Missing) && !agent.provider.install_hint().is_empty() {
        body = body.push(
            text(format!("Instálalo con: {}", agent.provider.install_hint()))
                .size(11)
                .font(design::MONO)
                .color(design::MUTED),
        );
    }
    body = body.push(open_button(state, agent));
    container(body)
        .padding(12)
        .width(Fill)
        .clip(true)
        .style(styles::card)
        .into()
}

/// The local models already connected to OpenCode, each one a click from its chat or terminal.
fn connected_models<'a>(state: &Dashboard) -> Element<'a, Message> {
    let mut list = column![].spacing(6);
    for name in state
        .local
        .snapshot
        .iter()
        .flat_map(|snapshot| snapshot.configured.iter())
    {
        let tool = |icon: &'static str, tip: &'static str, event: local::Event| {
            styles::tip(
                button(icons::icon(icon, design::TEXT, 13.0))
                    .padding(6)
                    .style(styles::secondary)
                    .on_press(Message::Local(event)),
                tip,
                tooltip::Position::Top,
            )
        };
        list = list.push(
            row![
                icons::icon("dot", design::SUCCESS, 9.0),
                text(local::short(name).to_owned())
                    .size(11)
                    .width(Fill)
                    .wrapping(iced::widget::text::Wrapping::None),
                tool(
                    "terminal",
                    "Abrir en una terminal",
                    local::Event::Terminal(name.clone())
                ),
                tool(
                    "sparkle",
                    "Abrir el chat visual",
                    local::Event::Chat(name.clone())
                ),
            ]
            .spacing(6)
            .align_y(iced::Center),
        );
    }
    list.into()
}

/// The way into local models: runs on this machine through Ollama.
fn local_card<'a>(state: &Dashboard) -> Element<'a, Message> {
    let open = state.panes.find(|c| *c == panes::Content::Local).is_some();
    container(
        column![
            row![
                container(icons::icon("cpu", design::TINT_GREEN, 16.0))
                    .center(30)
                    .width(30)
                    .height(30)
                    .style(styles::chip(design::TINT_GREEN)),
                column![
                    text("Modelos locales").size(13).font(design::UI_BOLD),
                    text("Ollama · en tu equipo").size(10).color(design::FAINT),
                ]
                .spacing(1)
                .width(Fill),
            ]
            .spacing(10)
            .align_y(iced::Center),
            text("Diagnostica tu PC, mira qué modelos caben y conéctalos a OpenCode, sin internet ni costo por uso.")
                .size(11)
                .color(design::MUTED),
            connected_models(state),
            button(
                row![
                    icons::icon("cpu", design::TEXT, 13.0),
                    text(if open { "Ver modelos locales" } else { "Explorar" }).size(12)
                ]
                .spacing(7)
                .align_y(iced::Center),
            )
            .padding([6, 12])
            .style(styles::secondary)
            .on_press(Message::Local(local::Event::Open)),
        ]
        .spacing(8),
    )
    .padding(12)
    .width(Fill)
    .style(styles::card)
    .into()
}

/// The sidebar panel.
pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    if !trusted(state) {
        list = list.push(
            container(
                column![
                    row![
                        icons::icon("shield", design::WARNING, 15.0),
                        text("Falta la confianza del proyecto")
                            .size(12)
                            .font(design::UI_BOLD),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                    text("Los agentes ejecutan comandos en tu proyecto: revísalo y confía en él antes de abrirlos.")
                        .size(11)
                        .color(design::MUTED),
                    button(text("Revisar confianza").size(11))
                        .padding([5, 10])
                        .style(styles::secondary)
                        .on_press(Message::Sequence(vec![
                            Message::Navigate(Page::Setup),
                            Message::PreviewTrust { desired: true },
                        ])),
                ]
                .spacing(7),
            )
            .padding(12)
            .width(Fill)
            .style(styles::card),
        );
    }
    let agents = &state.agents.catalog.agents;
    if let Some(error) = &state.agents.catalog.config_error {
        list = list.push(
            text(format!("Configuración de agentes: {error}"))
                .size(11)
                .color(design::WARNING),
        );
    }
    if agents.is_empty() {
        list = list.push(
            container(
                column![
                    text(if state.agents.loaded {
                        "No encontré agentes instalados"
                    } else {
                        "Buscando agentes…"
                    })
                    .size(12)
                    .font(design::UI_BOLD),
                    text(
                        "Studio abre Claude Code, Codex y OpenCode. Instala uno y vuelve a buscar."
                    )
                    .size(11)
                    .color(design::MUTED),
                ]
                .spacing(6),
            )
            .padding(12)
            .width(Fill)
            .style(styles::card),
        );
    }
    for agent in agents {
        list = list.push(agent_card(state, agent));
    }
    list = list.push(local_card(state));
    // Known agents that are not installed: the way to get them.
    let missing: Vec<_> = Provider::KNOWN
        .iter()
        .filter(|provider| !agents.iter().any(|a| a.provider == **provider))
        .collect();
    if state.agents.loaded && !missing.is_empty() && !agents.is_empty() {
        list = list.push(eyebrow("TAMBIÉN COMPATIBLES"));
        for provider in missing {
            list = list.push(
                row![
                    avatar(*provider, 22.0),
                    column![
                        text(provider.label()).size(12),
                        text(provider.install_hint())
                            .size(10)
                            .font(design::MONO)
                            .color(design::FAINT)
                            .wrapping(iced::widget::text::Wrapping::None),
                    ]
                    .spacing(1),
                ]
                .spacing(8)
                .align_y(iced::Center),
            );
        }
    }
    container(
        column![
            row![
                text("AGENTES")
                    .size(10)
                    .font(design::UI_BOLD)
                    .color(design::FAINT),
                Space::new().width(Fill),
                styles::tip(
                    button(icons::icon("refresh", design::MUTED, 13.0))
                        .padding(4)
                        .style(styles::ghost(false))
                        .on_press(Message::Agents(Event::Refresh)),
                    "Buscar agentes otra vez",
                    tooltip::Position::Left,
                ),
            ]
            .align_y(iced::Center)
            .padding([0, 12]),
            styles::vscroll(container(list).padding([0, 12]).width(Fill)).height(Fill),
        ]
        .spacing(8),
    )
    .padding([10, 0])
    .width(state.shell.explorer_width)
    .height(Fill)
    .style(styles::surface(design::BG_PANEL))
    .into()
}

/// A card for the home page: the agents of the project and a way in.
pub(super) fn home_card(state: &Dashboard) -> Element<'_, Message> {
    let (available, total) = counts(state);
    let summary = if !state.agents.loaded {
        "Buscando agentes…".to_owned()
    } else if total == 0 {
        "Instala Claude Code, Codex u OpenCode para trabajar con un asistente desde aquí."
            .to_owned()
    } else {
        format!(
            "{available} de {total} disponibles. Cada agente se abre con su propia interfaz dentro de Studio."
        )
    };
    let mut cards = row![].spacing(10);
    for agent in state.agents.catalog.agents.iter().take(3) {
        cards = cards.push(
            container(
                row![
                    avatar(agent.provider, 28.0),
                    column![
                        text(agent.label()).size(13).font(design::UI_BOLD),
                        status_chip(&standing(state, agent)),
                    ]
                    .spacing(4),
                    Space::new().width(Fill),
                    open_button(state, agent),
                ]
                .spacing(10)
                .align_y(iced::Center),
            )
            .padding(12)
            .width(Fill)
            .style(styles::well),
        );
    }
    let mut body = column![
        row![
            column![
                eyebrow("AGENTES"),
                text("Trabaja con tu asistente sin salir de Studio")
                    .size(16)
                    .font(design::UI_BOLD),
                text(summary).size(12).color(design::MUTED),
            ]
            .spacing(4),
            Space::new().width(Fill),
            button(text("Ver todos").size(12))
                .padding([6, 12])
                .style(styles::secondary)
                .on_press(Message::Shell(shell::Event::ShowSide(shell::Side::Agents))),
        ]
        .align_y(iced::Center),
    ]
    .spacing(12);
    if !state.agents.catalog.agents.is_empty() {
        body = body.push(cards);
    }
    container(body)
        .padding(20)
        .width(Fill)
        .style(styles::card)
        .into()
}
