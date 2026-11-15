//! The cards where the agent asks: permissions and questions.
use super::super::*;
use super::{Chat, Event, send};
use crate::features::chat_model::Entry;
use iced::widget::column;

pub(super) fn permission(index: usize, entry: &Entry) -> Element<'_, Message> {
    let mut body = column![
        row![
            icons::icon("shield", design::WARNING, 15.0),
            text("El agente pide permiso")
                .size(12)
                .font(design::UI_BOLD),
        ]
        .spacing(8)
        .align_y(iced::Center),
        text(&entry.text).size(13),
    ]
    .spacing(8);
    if !entry.output.is_empty() {
        body = body.push(text(&entry.output).size(11).color(design::MUTED));
    }
    if !entry.certain {
        body = body.push(
            text("No estoy seguro de qué se bloqueó: revisa antes de aceptar.")
                .size(11)
                .color(design::WARNING),
        );
    }
    if entry.decision.is_empty() {
        let mut actions = row![].spacing(8);
        for (choice, option) in entry.choices.iter().enumerate() {
            actions = actions.push(
                button(text(&option.label).size(12))
                    .padding([6, 12])
                    .style(styles::primary)
                    .on_press(send(Event::Decide(index, Some(choice)))),
            );
        }
        actions = actions.push(
            button(text("Denegar").size(12))
                .padding([6, 12])
                .style(styles::secondary)
                .on_press(send(Event::Decide(index, None))),
        );
        body = body.push(
            iced::widget::scrollable(actions).direction(
                iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(3)
                        .scroller_width(3),
                ),
            ),
        );
    } else {
        body = body.push(
            text(format!("Respuesta: {}", entry.decision))
                .size(11)
                .color(design::MUTED),
        );
    }
    container(body)
        .padding(12)
        .width(Fill)
        .style(styles::tinted(design::WARNING))
        .into()
}

pub(super) fn question<'a>(chat: &'a Chat, index: usize, entry: &'a Entry) -> Element<'a, Message> {
    let mut body =
        column![text("El agente te pregunta").size(12).font(design::UI_BOLD)].spacing(10);
    for (number, item) in entry.questions.iter().enumerate() {
        body = body.push(text(&item.question).size(13));
        let mut options = row![].spacing(6);
        for (label, detail) in &item.options {
            let picked = chat
                .picks
                .iter()
                .any(|p| p.0 == index && p.1 == number && &p.2 == label);
            let button = button(text(label).size(12))
                .padding([5, 10])
                .style(if picked {
                    styles::primary
                } else {
                    styles::secondary
                });
            let button = if entry.decision.is_empty() {
                button.on_press(send(Event::Pick(index, number, label.clone())))
            } else {
                button
            };
            options = options.push(styles::tip(
                button,
                detail,
                iced::widget::tooltip::Position::Top,
            ));
        }
        body = body.push(options);
    }
    if entry.decision.is_empty() {
        let ready = chat.picks.iter().any(|p| p.0 == index);
        body = body.push(
            button(text("Responder").size(12))
                .padding([6, 12])
                .style(styles::primary)
                .on_press_maybe(ready.then(|| send(Event::Reply(index)))),
        );
    } else {
        body = body.push(
            text(format!("Respuesta: {}", entry.decision))
                .size(11)
                .color(design::MUTED),
        );
    }
    container(body)
        .padding(12)
        .width(Fill)
        .style(styles::tinted(design::ACCENT))
        .into()
}
