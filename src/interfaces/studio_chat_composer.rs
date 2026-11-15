//! The message box of a chat: attachments, text and the send/stop button.
use super::super::*;
use super::{Chat, Event, cards, images, send, slash};
use iced::widget::{column, text_editor, tooltip};

pub(super) fn composer(chat: &Chat) -> Element<'_, Message> {
    let running = chat.turn.is_some();
    let has_content = !chat.composer.text().trim().is_empty() || !chat.images.is_empty();
    let ready = has_content && !running && !chat.preparing && chat.problem.is_none();
    let (to_edit, to_keys) = (super::sender(), super::sender());
    let menu_open = slash::menu(chat).is_some();
    let editor = text_editor(&chat.composer)
        .placeholder("Escribe un mensaje, / para comandos, o pega una captura…")
        .on_action(move |action| to_edit(Event::Edit(action)))
        .height(iced::Length::Shrink)
        .min_height(40)
        .max_height(180)
        .padding([10, 12])
        .size(13)
        .style(styles::composer_editor)
        .key_binding(move |key| {
            use iced::keyboard::{Key, key::Named};
            let enter = matches!(key.key.as_ref(), Key::Named(Named::Enter));
            let paste = key.modifiers.command()
                && matches!(key.key.as_ref(), Key::Character(c) if c.eq_ignore_ascii_case("v"));
            if enter && !key.modifiers.shift() {
                Some(text_editor::Binding::Custom(to_keys(if menu_open {
                    Event::SlashEnter
                } else {
                    Event::Send
                })))
            } else if paste {
                Some(text_editor::Binding::Custom(to_keys(Event::Paste)))
            } else {
                text_editor::Binding::from_key_press(key)
            }
        });
    let attach = styles::tip(
        button(icons::icon("plus", design::MUTED, 15.0))
            .padding(6)
            .style(styles::ghost(false))
            .on_press(send(Event::Attach)),
        "Adjuntar imágenes (también puedes arrastrarlas o pegarlas)",
        tooltip::Position::Top,
    );
    let action = if running {
        styles::tip(
            button(icons::icon("stop", design::TEXT, 14.0))
                .padding(8)
                .style(styles::danger)
                .on_press(send(Event::Stop)),
            "Detener al agente",
            tooltip::Position::Top,
        )
    } else {
        styles::tip(
            button(icons::icon("arrow-up", iced::Color::WHITE, 14.0))
                .padding(8)
                .style(styles::primary)
                .on_press_maybe(ready.then(|| send(Event::Send))),
            "Enviar (Intro)",
            tooltip::Position::Top,
        )
    };
    let mut body = column![].spacing(8);
    if let Some(menu) = slash::menu(chat) {
        body = body.push(menu);
    }
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
    let footer = row![
        attach,
        text("Intro envía · Mayús+Intro, nueva línea")
            .size(10)
            .color(design::FAINT),
        Space::new().width(Fill),
        action,
    ]
    .spacing(8)
    .align_y(iced::Center);
    body = body.push(
        container(
            column![editor, footer]
                .spacing(2)
                .padding(iced::Padding::new(6.0).top(0.0)),
        )
        .style(styles::composer_card),
    );
    container(body)
        .padding([8, 16])
        .width(Fill)
        .style(styles::surface(design::BG_BASE))
        .into()
}
