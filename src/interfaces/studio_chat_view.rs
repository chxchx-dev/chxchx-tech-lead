//! The chat page: header with the controls, transcript, usage bar and composer.
use super::super::agents;
use super::super::*;
use super::{ACCESS, Chat, Event, VISIBLE_ENTRIES, cards, composer, perms, send, usage};
use iced::widget::{column, pick_list, tooltip};

pub(in crate::interfaces::studio) fn view<'a>(
    state: &'a Dashboard,
    id: &str,
) -> Element<'a, Message> {
    super::CURRENT.with(|current| *current.borrow_mut() = id.to_owned());
    let Some(chat) = state.chat.chats.iter().find(|c| c.agent.id == id) else {
        return container(text("Abre un agente desde el panel Agentes.").color(design::MUTED))
            .center(Fill)
            .into();
    };
    let mut transcript = column![].spacing(12).padding([16, 24]).max_width(900);
    if chat.preparing {
        transcript = transcript.push(text("Preparando el chat…").size(12).color(design::MUTED));
    }
    if let Some(problem) = &chat.problem {
        transcript = transcript.push(cards_note(problem));
    }
    if chat.model.entries.is_empty() && !chat.preparing && chat.problem.is_none() {
        transcript = transcript.push(welcome(chat));
    }
    // A long chat keeps only its latest messages on screen: every frame of a scroll lays them all out.
    let skipped = chat.model.entries.len().saturating_sub(VISIBLE_ENTRIES);
    if skipped > 0 {
        transcript = transcript.push(
            text(format!(
                "{skipped} mensajes anteriores ocultos para mantener el chat fluido."
            ))
            .size(11)
            .color(design::FAINT),
        );
    }
    for (index, entry) in chat.model.entries.iter().enumerate().skip(skipped) {
        transcript = transcript.push(cards::entry(chat, index, entry));
    }
    if chat.turn.is_some() && !chat.model.awaiting_answer() {
        transcript = transcript.push(text("Trabajando…").size(11).color(design::ACCENT));
    }
    let body = styles::vscroll(container(transcript).center_x(Fill))
        .anchor_bottom()
        .height(Fill);
    container(
        column![
            header(chat),
            body,
            usage::usage_bar(&chat.model.usage),
            composer::composer(chat)
        ]
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(styles::surface(design::BG_BASE))
    .into()
}

fn cards_note(problem: &str) -> Element<'_, Message> {
    container(text(problem).size(12))
        .padding([8, 12])
        .width(Fill)
        .style(styles::tinted(design::WARNING))
        .into()
}

const STARTERS: [(&str, &str); 4] = [
    (
        "Explicar el proyecto",
        "Explícame cómo está organizado este proyecto y por dónde empezar.",
    ),
    (
        "Revisar mis cambios",
        "Revisa los cambios sin guardar en Git y dime si ves errores o riesgos.",
    ),
    (
        "Correr las pruebas",
        "Corre las pruebas del proyecto y resume lo que falle.",
    ),
    (
        "Resolver desde una captura",
        "Te adjunto una captura: ve directo al problema que muestra y resuélvelo.",
    ),
];

fn welcome(chat: &Chat) -> Element<'_, Message> {
    let mut starters = row![].spacing(8);
    for (label, prompt) in STARTERS {
        starters = starters.push(
            button(text(label).size(12))
                .padding([6, 12])
                .style(styles::secondary)
                .on_press(send(Event::Starter(prompt.to_owned()))),
        );
    }
    column![
        agents::avatar(chat.agent.provider, 40.0),
        text(format!("Chat con {}", chat.agent.label()))
            .size(20)
            .font(design::UI_BOLD),
        text("Escribe, pega una captura (Ctrl+V) o arrastra imágenes aquí. El agente te pedirá permiso en este chat antes de hacer algo que tu nivel de acceso no cubra, y puedes cambiarlo cuando quieras en «Permisos».")
            .size(12)
            .color(design::MUTED),
        iced::widget::scrollable(starters).direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new().width(3).scroller_width(3),
        )),
    ]
    .spacing(10)
    .into()
}

fn access_label(mode: &str) -> &'static str {
    ACCESS
        .iter()
        .find(|(m, _)| *m == mode)
        .map_or("Acceso", |(_, l)| l)
}

fn header<'a>(chat: &'a Chat) -> Element<'a, Message> {
    let working = chat.turn.is_some();
    let (status, tint) = if chat.preparing {
        ("Preparando…", design::MUTED)
    } else if chat.model.awaiting_answer() {
        ("Espera tu respuesta", design::WARNING)
    } else if working {
        ("Trabajando…", design::ACCENT)
    } else {
        ("Listo", design::SUCCESS)
    };
    let mut top = row![
        agents::avatar(chat.agent.provider, 30.0),
        column![
            text(chat.agent.label()).size(14).font(design::UI_BOLD),
            row![
                icons::icon("dot", tint, 10.0),
                text(status).size(11).color(tint)
            ]
            .spacing(4)
            .align_y(iced::Center),
        ]
        .spacing(1),
        Space::new().width(Fill),
    ]
    .spacing(10)
    .align_y(iced::Center);
    if let Some(options) = chat.options.as_ref().filter(|o| !o.models.is_empty()) {
        let ids: Vec<String> = options.models.iter().map(|m| m.id.clone()).collect();
        top = top.push(
            pick_list(ids, chat.model_id.clone(), |id| send(Event::Model(id)))
                .text_size(12)
                .padding([4, 8]),
        );
        let efforts = options
            .models
            .iter()
            .find(|m| Some(&m.id) == chat.model_id.as_ref())
            .map(|m| m.efforts.clone())
            .unwrap_or_default();
        if !efforts.is_empty() {
            top = top.push(
                pick_list(efforts, chat.effort.clone(), |e| send(Event::Effort(e)))
                    .text_size(12)
                    .padding([4, 8]),
            );
        }
    }
    let danger = chat.mode == "full-access";
    let permissions = button(
        row![
            icons::icon(
                "shield",
                if danger {
                    design::WARNING
                } else {
                    design::TEXT
                },
                13.0
            ),
            text(format!(
                "Permisos · {}{}",
                access_label(&chat.mode),
                if chat.network { " + red" } else { "" }
            ))
            .size(12),
            icons::icon(
                if chat.perms_open {
                    "chevron-down"
                } else {
                    "chevron-right"
                },
                design::MUTED,
                11.0
            ),
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .padding([5, 10])
    .style(styles::secondary)
    .on_press(send(Event::Permissions));
    top = top
        .push(styles::tip(
            permissions,
            "Cambia qué puede hacer el agente sin preguntarte",
            tooltip::Position::Bottom,
        ))
        .push(
            button(text("Nuevo chat").size(12))
                .padding([5, 10])
                .style(styles::secondary)
                .on_press(send(Event::NewChat)),
        )
        .push(
            button(
                row![
                    icons::icon("terminal", design::TEXT, 12.0),
                    text("Terminal").size(12)
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .padding([5, 10])
            .style(styles::secondary)
            .on_press(Message::Agents(agents::Event::Terminal(
                chat.agent.id.clone(),
            ))),
        );
    let mut head = column![container(top).padding([8, 16]).width(Fill)];
    if chat.perms_open {
        head = head.push(perms::panel(chat));
    }
    container(head)
        .width(Fill)
        .style(styles::surface(design::BG_PANEL))
        .into()
}
