//! One card per transcript entry: messages, reasoning, tools, notices and the cards that ask.
use super::super::agents;
use super::super::*;
use super::{Chat, Event, send};
use crate::features::chat_model::{Entry, Kind};
use iced::widget::{column, markdown};

pub(super) fn entry<'a>(chat: &'a Chat, index: usize, entry: &'a Entry) -> Element<'a, Message> {
    match entry.kind {
        Kind::User => user(chat, index, &entry.text),
        Kind::Assistant => assistant(chat, index, entry),
        Kind::Reasoning => reasoning(chat, index, &entry.text),
        Kind::Tool => tool(chat, index, entry),
        Kind::Notice => note("info", design::ACCENT, &entry.text),
        Kind::Error => note("warning", design::DANGER, &entry.text),
        Kind::Permission => super::asks::permission(index, entry),
        Kind::Question => super::asks::question(chat, index, entry),
    }
}

fn user<'a>(chat: &'a Chat, index: usize, body: &'a str) -> Element<'a, Message> {
    let mut bubble = column![].spacing(8);
    if let Some((_, images)) = chat.sent.iter().find(|(entry, _)| *entry == index) {
        let mut strip = row![].spacing(6);
        for image in images {
            strip = strip.push(thumb(image, 84.0));
        }
        bubble = bubble.push(strip);
    }
    if !body.trim().is_empty() {
        bubble = bubble.push(text(body).size(13));
    }
    row![
        Space::new().width(Fill),
        container(bubble)
            .padding([9, 13])
            .max_width(620)
            .style(styles::tinted(design::BRAND_BLUE)),
    ]
    .into()
}

pub(super) fn thumb<'a>(image: &'a super::Image, side: f32) -> Element<'a, Message> {
    container(
        iced::widget::image(image.thumb.clone())
            .width(side)
            .height(side)
            .content_fit(iced::ContentFit::Cover),
    )
    .clip(true)
    .style(styles::chip_surface(false))
    .into()
}

fn assistant<'a>(chat: &'a Chat, index: usize, entry: &'a Entry) -> Element<'a, Message> {
    let body: Element<'a, Message> = match chat.rendered.get(index) {
        Some((_, content)) => markdown::view(
            content.items(),
            markdown::Settings::with_text_size(13, design::theme()),
        )
        .map(|_| send(Event::Link)),
        None => text(&entry.text).size(13).into(),
    };
    row![
        agents::avatar(chat.agent.provider, 26.0),
        column![
            row![
                text(chat.agent.label())
                    .size(11)
                    .font(design::UI_BOLD)
                    .color(design::MUTED),
                Space::new().width(Fill),
                styles::tip(
                    button(icons::icon("file-text", design::FAINT, 12.0))
                        .padding(3)
                        .style(styles::ghost(false))
                        .on_press(send(Event::Copy(entry.text.clone()))),
                    "Copiar respuesta",
                    iced::widget::tooltip::Position::Left,
                ),
            ]
            .align_y(iced::Center),
            body,
        ]
        .spacing(4)
        .width(Fill),
    ]
    .spacing(10)
    .into()
}

/// A header that expands or collapses the card below it.
fn fold<'a>(
    chat: &Chat,
    index: usize,
    head: Element<'a, Message>,
    detail: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let open = chat.expanded.contains(&index);
    let toggle = button(
        row![
            icons::icon(
                if open {
                    "chevron-down"
                } else {
                    "chevron-right"
                },
                design::FAINT,
                12.0
            ),
            head,
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .padding(0)
    .style(styles::plain(false))
    .on_press(send(Event::Toggle(index)));
    let mut body = column![toggle].spacing(6);
    if open && let Some(detail) = detail {
        body = body.push(detail);
    }
    body.into()
}

fn reasoning<'a>(chat: &Chat, index: usize, body: &'a str) -> Element<'a, Message> {
    let open = chat.expanded.contains(&index);
    let first = body
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(90)
        .collect::<String>();
    let head = text(if open {
        "Razonamiento".to_owned()
    } else {
        format!("Razonamiento · {first}…")
    })
    .size(11)
    .color(design::FAINT)
    .into();
    fold(
        chat,
        index,
        head,
        Some(text(body).size(11).color(design::FAINT).into()),
    )
}

fn note<'a>(icon: &'static str, tint: iced::Color, body: &'a str) -> Element<'a, Message> {
    container(
        row![icons::icon(icon, tint, 14.0), text(body).size(12)]
            .spacing(8)
            .align_y(iced::Center),
    )
    .padding([8, 12])
    .width(Fill)
    .style(styles::tinted(tint))
    .into()
}

fn tool<'a>(chat: &Chat, index: usize, entry: &'a Entry) -> Element<'a, Message> {
    let (icon, tint) = match entry.status.as_str() {
        "completed" | "done" | "success" => ("check", design::SUCCESS),
        "failed" | "error" => ("warning", design::DANGER),
        _ => ("refresh", design::ACCENT),
    };
    let head = row![
        icons::icon(icon, tint, 13.0),
        text(&entry.text)
            .size(12)
            .font(design::MONO)
            .wrapping(iced::widget::text::Wrapping::None),
    ]
    .spacing(8)
    .align_y(iced::Center)
    .into();
    let detail = (!entry.output.trim().is_empty()).then(|| {
        text(tail(&entry.output, 4000))
            .size(11)
            .font(design::MONO)
            .color(design::MUTED)
            .into()
    });
    container(fold(chat, index, head, detail))
        .padding([7, 12])
        .width(Fill)
        .clip(true)
        .style(styles::well)
        .into()
}

/// The last `max` characters, so long outputs do not take the whole chat.
fn tail(output: &str, max: usize) -> String {
    let output = output.trim_end();
    let count = output.chars().count();
    if count <= max {
        return output.to_owned();
    }
    let skipped: String = output.chars().skip(count - max).collect();
    format!("…{skipped}")
}
