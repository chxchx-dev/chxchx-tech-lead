//! The permissions panel of a chat: access level, network and the rules already approved.
use super::super::*;
use super::{ACCESS, Chat, Event, send};
use iced::widget::{checkbox, column};

fn describe(mode: &str) -> &'static str {
    match mode {
        "read-only" => "Lee y responde. No cambia nada.",
        "workspace-write" => "Edita archivos del proyecto. Pregunta antes de ejecutar comandos.",
        "workspace-commands" => "Edita archivos y ejecuta comandos del proyecto.",
        _ => "Sin límites. Úsalo solo si confías en este proyecto.",
    }
}

fn level<'a>(chat: &Chat, mode: &'static str, label: &'static str) -> Element<'a, Message> {
    let active = chat.mode == mode;
    let tint = if mode == "full-access" {
        design::warning()
    } else {
        design::accent()
    };
    button(
        column![
            row![
                icons::icon(
                    if active { "check" } else { "shield" },
                    if active { tint } else { design::muted() },
                    13.0
                ),
                text(label).size(12).font(design::UI_BOLD),
            ]
            .spacing(6)
            .align_y(iced::Center),
            text(describe(mode)).size(10).color(design::muted()),
        ]
        .spacing(4),
    )
    .width(Fill)
    .padding(10)
    .style(move |theme: &iced::Theme, status| {
        let mut style = styles::ghost(active)(theme, status);
        style.border = iced::Border {
            color: if active { tint } else { design::border() },
            width: 1.0,
            radius: 8.0.into(),
        };
        style
    })
    .on_press(send(Event::Access(mode.to_owned())))
    .into()
}

fn rules<'a>(title: &'a str, list: &'a [String], remembered: bool) -> Element<'a, Message> {
    let mut body = column![
        text(title)
            .size(10)
            .font(design::UI_BOLD)
            .color(design::faint())
    ]
    .spacing(4);
    for rule in list {
        body = body.push(
            row![
                text(rule).size(11).font(design::MONO).width(Fill),
                button(icons::icon("close", design::muted(), 11.0))
                    .padding(3)
                    .style(styles::ghost(false))
                    .on_press(send(Event::Revoke(rule.clone(), remembered))),
            ]
            .align_y(iced::Center),
        );
    }
    body.into()
}

pub(super) fn panel(chat: &Chat) -> Element<'_, Message> {
    let mut levels = row![].spacing(8);
    for (mode, label) in ACCESS {
        levels = levels.push(level(chat, mode, label));
    }
    let to = super::sender();
    let mut body = column![
        levels,
        checkbox(chat.network)
            .label("Permitir red (descargas, instalar paquetes, consultar la web)")
            .size(14)
            .text_size(12)
            .on_toggle(move |on| to(Event::Network(on))),
    ]
    .spacing(10);
    if !chat.approved.is_empty() {
        body = body.push(rules("APROBADO PARA ESTE CHAT", &chat.approved, true));
    }
    if !chat.once.is_empty() {
        body = body.push(rules("SOLO EL PRÓXIMO MENSAJE", &chat.once, false));
    }
    let note = if chat.turn.is_some() {
        "El agente está trabajando: el cambio vale desde tu siguiente mensaje."
    } else {
        "Cuando el agente necesite algo más, te lo preguntará aquí con una tarjeta."
    };
    container(body.push(text(note).size(10).color(design::faint())))
        .padding([12, 16])
        .width(Fill)
        .style(styles::surface(design::bg_raised()))
        .into()
}
