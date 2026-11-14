//! The message box of a chat: attachments, text and the send/stop button.
use super::super::*;
use super::{Chat, Event, cards, images, send};
use iced::widget::{column, text_editor, tooltip};

pub(super) fn composer(chat: &Chat) -> Element<'_, Message> {
    let running = chat.turn.is_some();
    let has_content = !chat.composer.text().trim().is_empty() || !chat.images.is_empty();
    let ready = has_content && !running && !chat.preparing && chat.problem.is_none();
    let editor = text_editor(&chat.composer)
        .placeholder(
            "Escribe un mensaje · Intro envía, Mayús+Intro agrega una línea · Ctrl+V pega capturas",
        )
        .on_action(|action| send(Event::Edit(action)))
        .height(iced::Length::Shrink)
        .min_height(44)
        .max_height(160)
        .padding(10)
        .size(13)
        .style(styles::message_box)
        .key_binding(|key| {
            use iced::keyboard::{Key, key::Named};
            let enter = matches!(key.key.as_ref(), Key::Named(Named::Enter));
            let paste = key.modifiers.command()
                && matches!(key.key.as_ref(), Key::Character(c) if c.eq_ignore_ascii_case("v"));
            if enter && !key.modifiers.shift() {
                Some(text_editor::Binding::Custom(send(Event::Send)))
            } else if paste {
                Some(text_editor::Binding::Custom(send(Event::Paste)))
            } else {
                text_editor::Binding::from_key_press(key)
            }
        });
    let attach = styles::tip(
        button(icons::icon("plus", design::TEXT, 14.0))
            .padding([8, 10])
            .style(styles::secondary)
            .on_press(send(Event::Attach)),
        "Adjuntar imágenes (también puedes arrastrarlas o pegarlas)",
        tooltip::Position::Top,
    );
    let action = if running {
        button(
            row![
                icons::icon("stop", design::TEXT, 13.0),
                text("Detener").size(12)
            ]
            .spacing(6)
            .align_y(iced::Center),
        )
        .padding([8, 14])
        .style(styles::danger)
        .on_press(send(Event::Stop))
    } else {
        button(text("Enviar").size(12).font(design::UI_BOLD))
            .padding([8, 14])
            .style(styles::primary)
            .on_press_maybe(ready.then(|| send(Event::Send)))
    };
    let mut body = column![].spacing(8);
    if !chat.images.is_empty() {
        let mut strip = row![].spacing(8);
        for (index, image) in chat.images.iter().enumerate() {
            strip = strip.push(
                container(
                    row![
                        cards::thumb(image, 48.0),
                        column![
                            text(&image.prepared.name)
                                .size(11)
                                .wrapping(iced::widget::text::Wrapping::None),
                            text(images::note(image))
                                .size(10)
                                .color(design::FAINT)
                                .wrapping(iced::widget::text::Wrapping::None),
                        ]
                        .spacing(2),
                        button(icons::icon("close", design::MUTED, 11.0))
                            .padding(3)
                            .style(styles::ghost(false))
                            .on_press(send(Event::RemoveImage(index))),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                )
                .padding(6)
                .style(styles::chip_surface(false)),
            );
        }
        body = body.push(
            iced::widget::scrollable(strip).direction(
                iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(3)
                        .scroller_width(3),
                ),
            ),
        );
    }
    body = body.push(
        row![attach, editor, action]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
    );
    container(body)
        .padding([10, 16])
        .width(Fill)
        .style(styles::surface(design::BG_PANEL))
        .into()
}
