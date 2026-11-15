//! The chat page: header with the controls, transcript, usage bar and composer.
use super::super::agents;
use super::super::*;
use super::{
    ACCESS, Chat, Event, VISIBLE_ENTRIES, art, cards, composer, models, perms, send, usage,
};
use iced::widget::{column, tooltip};

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
    let last = chat.model.entries.last().map(|e| e.kind);
    if chat.turn.is_none()
        && matches!(
            last,
            Some(
                crate::features::chat_model::Kind::Assistant
                    | crate::features::chat_model::Kind::Error
            )
        )
    {
        transcript = transcript.push(row![
            button(
                row![
                    icons::icon("refresh", design::MUTED, 12.0),
                    text("Reintentar").size(11)
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .padding([4, 10])
            .style(styles::ghost(false))
            .on_press(send(Event::Retry)),
        ]);
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
    let mut intro = column![
        art::banner(chat.agent.provider, chat.agent.label()),
        text(format!("Chat con {}", chat.agent.label()))
            .size(14)
            .font(design::UI_BOLD),
    ]
    .spacing(10);
    if chat
        .model_id
        .as_deref()
        .is_some_and(|id| id.starts_with("ollama/"))
    {
        intro = intro.push(
            row![
                icons::icon("cpu", design::SUCCESS, 13.0),
                text("Este modelo corre en tu equipo: tus mensajes no salen a internet.")
                    .size(12)
                    .color(design::SUCCESS),
            ]
            .spacing(6)
            .align_y(iced::Center),
        );
    }
    intro
        .push(text("Escribe, pega una captura (Ctrl+V) o arrastra imágenes aquí. El agente te pedirá permiso en este chat antes de hacer algo que tu nivel de acceso no cubra, y puedes cambiarlo cuando quieras en «Permisos».")
            .size(12)
            .color(design::MUTED))
        .push(iced::widget::scrollable(starters).direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new().width(3).scroller_width(3),
        )))
        .into()
}

fn access_label(mode: &str) -> &'static str {
    ACCESS
        .iter()
        .find(|(m, _)| *m == mode)
        .map_or("Acceso", |(_, l)| l)
}

/// A chip that opens one of the header panels.
fn chip<'a>(
    icon: &'static str,
    label: String,
    open: bool,
    warn: bool,
    message: Message,
    tip: &'static str,
) -> Element<'a, Message> {
    let tint = if warn { design::WARNING } else { design::TEXT };
    styles::tip(
        button(
            row![
                icons::icon(
                    icon,
                    if warn { design::WARNING } else { design::MUTED },
                    12.0
                ),
                text(label).size(11).color(tint),
                icons::icon(
                    if open {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    },
                    design::FAINT,
                    10.0
                ),
            ]
            .spacing(6)
            .align_y(iced::Center),
        )
        .padding([5, 10])
        .style(move |theme, status| {
            if open {
                styles::ghost(true)(theme, status)
            } else {
                styles::secondary(theme, status)
            }
        })
        .on_press(message),
        tip,
        tooltip::Position::Bottom,
    )
}

fn icon_button<'a>(
    icon: &'static str,
    tip: &'static str,
    message: Message,
) -> Element<'a, Message> {
    styles::tip(
        button(icons::icon(icon, design::MUTED, 14.0))
            .padding(6)
            .style(styles::ghost(false))
            .on_press(message),
        tip,
        tooltip::Position::Bottom,
    )
}

fn header<'a>(chat: &'a Chat) -> Element<'a, Message> {
    let (status, tint) = if chat.preparing {
        ("Preparando…", design::MUTED)
    } else if chat.model.awaiting_answer() {
        ("Espera tu respuesta", design::WARNING)
    } else if chat.turn.is_some() {
        ("Trabajando…", design::ACCENT)
    } else {
        ("Listo", design::SUCCESS)
    };
    let top = row![
        agents::avatar(chat.agent.provider, 28.0),
        column![
            text(chat.agent.label()).size(13).font(design::UI_BOLD),
            row![
                icons::icon("dot", tint, 9.0),
                text(status).size(10).color(tint)
            ]
            .spacing(4)
            .align_y(iced::Center),
        ]
        .spacing(1),
        Space::new().width(Fill),
        icon_button("plus", "Nuevo chat: empieza de cero", send(Event::NewChat)),
        icon_button(
            "terminal",
            "Abrir este agente en una terminal",
            Message::Agents(agents::Event::Terminal(chat.agent.id.clone())),
        ),
    ]
    .spacing(10)
    .align_y(iced::Center);

    let mut chips = row![].spacing(4).align_y(iced::Center);
    if chat.options.is_some() {
        let model = chat.model_id.as_deref().map_or_else(
            || "Modelo por defecto".to_owned(),
            |id| models::short_name(id).to_owned(),
        );
        let local = chat
            .model_id
            .as_deref()
            .is_some_and(|id| id.starts_with("ollama/"));
        chips = chips.push(chip(
            if local { "cpu" } else { "sparkle" },
            if local {
                format!("{model} · local")
            } else {
                model
            },
            chat.models_open,
            false,
            send(Event::ModelMenu),
            "Elegir modelo y esfuerzo",
        ));
        let has_efforts = chat
            .options
            .as_ref()
            .and_then(|o| {
                o.models
                    .iter()
                    .find(|m| Some(&m.id) == chat.model_id.as_ref())
            })
            .is_some_and(|m| !m.efforts.is_empty());
        if has_efforts || chat.effort.is_some() {
            chips = chips.push(chip(
                "activity",
                format!(
                    "Esfuerzo: {}",
                    chat.effort.as_deref().unwrap_or("automático")
                ),
                chat.models_open,
                false,
                send(Event::ModelMenu),
                "Esfuerzo de razonamiento",
            ));
        }
    }
    chips = chips.push(chip(
        "shield",
        format!(
            "{}{}",
            access_label(&chat.mode),
            if chat.network { " + red" } else { "" }
        ),
        chat.perms_open,
        chat.mode == "full-access",
        send(Event::Permissions),
        "Cambia qué puede hacer el agente sin preguntarte",
    ));
    let mut head = column![
        container(top).padding(iced::Padding::new(10.0).left(16.0).right(12.0).bottom(2.0)),
        container(chips).padding(iced::Padding::new(0.0).left(10.0).right(10.0).bottom(8.0)),
    ];
    if chat.models_open {
        head = head.push(models::panel(chat));
    }
    if chat.perms_open {
        head = head.push(perms::panel(chat));
    }
    container(head)
        .width(Fill)
        .style(styles::surface(design::BG_PANEL))
        .into()
}
