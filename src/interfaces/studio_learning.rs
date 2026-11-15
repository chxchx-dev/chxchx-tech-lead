//! Learning path on the home page: the steps to get the most out of Studio, ticked off by what
//! the project and the workspace already have, each with the button that does it.
use super::pages::eyebrow;
use super::*;
use iced::widget::{column, progress_bar};

struct Step {
    title: &'static str,
    detail: &'static str,
    label: &'static str,
    /// `None` when Studio cannot tell (it is never ticked).
    done: Option<bool>,
    action: Option<Message>,
}

fn steps(state: &Dashboard) -> Vec<Step> {
    let workspace = state.overview.as_ref().ok().map(|p| &p.workspace);
    let first_chat = state
        .agents
        .catalog
        .agents
        .iter()
        .find(|a| a.provider.has_visual_chat() && a.available())
        .map(|a| Message::Agents(agents::Event::Chat(a.id.clone())));
    let editor = state.panes.find(|c| *c == panes::Content::Editor);
    vec![
        Step {
            title: "Prepara tu proyecto",
            detail: "Crea la configuración y las guías para trabajar con asistentes. Revisas los archivos antes de aplicar.",
            label: "Revisar preparación",
            done: workspace.map(|w| w.config_exists),
            action: Some(Message::PreviewInit { minimal: false }),
        },
        Step {
            title: "Concede confianza local",
            detail: "Sin confianza ningún agente ni terminal ejecuta nada. Es una decisión tuya y puedes revocarla.",
            label: "Revisar confianza",
            done: workspace.map(|w| w.trusted),
            action: Some(Message::PreviewTrust { desired: true }),
        },
        Step {
            title: "Encuentra tus agentes",
            detail: "Claude Code, Codex y OpenCode se detectan solos. Si falta alguno, el panel te dice cómo instalarlo.",
            label: "Ver agentes",
            done: Some(agents::counts(state).0 > 0),
            action: Some(Message::Shell(shell::Event::ShowSide(shell::Side::Agents))),
        },
        Step {
            title: "Conversa con un agente",
            detail: "El chat visual pide permiso antes de actuar, muestra tokens y contexto, y acepta capturas (Ctrl+V).",
            label: "Abrir un chat",
            done: Some(!state.chat.chats.is_empty()),
            action: first_chat,
        },
        Step {
            title: "Abre una terminal de verdad",
            detail: "Un shell del proyecto o un agente con su propia interfaz, con colores, historial y teclado completo.",
            label: "Abrir terminal",
            done: Some(state.terminal.has_sessions()),
            action: Some(Message::Terminal(terminal::Event::Open)),
        },
        Step {
            title: "Acomoda tu espacio",
            detail: "Arrastra la barra de título de un panel hasta el borde de otro para dividir la pantalla; maximiza el que necesites.",
            label: "Dividir con una terminal",
            done: Some(state.panes.grid.len() >= 2 || state.panes.terminal_panes() >= 2),
            action: editor.map(|pane| {
                Message::Panes(panes::Event::Split(
                    pane,
                    iced::widget::pane_grid::Axis::Vertical,
                ))
            }),
        },
        Step {
            title: "Revisa tus cambios con Git",
            detail: "Historial con grafo, cambios por grupos y mensajes de commit sugeridos, sin salir de Studio.",
            label: "Abrir Git",
            done: Some(state.git.is_repository()),
            action: Some(Message::Shell(shell::Event::ShowSide(shell::Side::Git))),
        },
        Step {
            title: "Conecta tus herramientas (MCP)",
            detail: "Prepara las conexiones de tus asistentes con los comandos y el alcance a la vista.",
            label: "Revisar conexiones",
            done: None,
            action: Some(Message::PreviewMcp),
        },
    ]
}

pub(super) fn card(state: &Dashboard) -> Element<'_, Message> {
    let steps = steps(state);
    let known = steps.iter().filter(|s| s.done.is_some()).count();
    let done = steps.iter().filter(|s| s.done == Some(true)).count();
    let next = steps.iter().position(|s| s.done != Some(true));
    let mut list = column![].spacing(8);
    for (index, step) in steps.into_iter().enumerate() {
        list = list.push(row_of(step, Some(index) == next));
    }
    container(
        column![
            row![
                column![
                    eyebrow("RUTA DE APRENDIZAJE"),
                    text("Aprende Studio paso a paso")
                        .size(16)
                        .font(design::UI_BOLD),
                    text("Se marca sola según lo que ya hiciste en este proyecto.")
                        .size(12)
                        .color(design::MUTED),
                ]
                .spacing(4),
                Space::new().width(Fill),
                text(format!("{done} de {known}"))
                    .size(12)
                    .font(design::UI_BOLD)
                    .color(design::ACCENT),
            ]
            .align_y(iced::Center),
            progress_bar(0.0..=known.max(1) as f32, done as f32)
                .girth(5)
                .style(|_: &iced::Theme| iced::widget::progress_bar::Style {
                    background: iced::Background::Color(design::BORDER),
                    bar: iced::Background::Color(design::ACCENT),
                    border: iced::Border::default().rounded(3.0),
                }),
            list,
        ]
        .spacing(12),
    )
    .padding(20)
    .width(Fill)
    .style(styles::card)
    .into()
}

fn row_of(step: Step, next: bool) -> Element<'static, Message> {
    let (icon, tint) = match step.done {
        Some(true) => ("check", design::SUCCESS),
        _ if next => ("arrow-right", design::ACCENT),
        _ => ("dot", design::FAINT),
    };
    let action = button(text(step.label).size(12))
        .padding([5, 12])
        .style(if next {
            styles::primary
        } else {
            styles::secondary
        })
        .on_press_maybe(step.action);
    container(
        row![
            icons::icon(icon, tint, 15.0),
            column![
                text(step.title).size(13).font(design::UI_BOLD),
                text(step.detail).size(11).color(design::MUTED),
            ]
            .spacing(2)
            .width(Fill),
            action,
        ]
        .spacing(12)
        .align_y(iced::Center),
    )
    .padding([9, 12])
    .width(Fill)
    .style(move |theme: &iced::Theme| {
        if next {
            styles::tinted(design::ACCENT)(theme)
        } else {
            styles::well(theme)
        }
    })
    .into()
}
