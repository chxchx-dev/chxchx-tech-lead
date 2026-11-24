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
        Kind::Notice => note("info", design::accent(), &entry.text),
        Kind::Error => note("warning", design::danger(), &entry.text),
        Kind::Permission => super::asks::permission(index, entry),
        Kind::Question => super::asks::question(chat, index, entry),
    }
}

/// Characters after which a sent message is folded in the transcript.
const LONG_MESSAGE: usize = 1500;

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
        // A huge pasted message is laid out on every frame: fold it until the person asks.
        let long = body.chars().nth(LONG_MESSAGE).is_some();
        if let Some(editor) = selectable(chat, index, 13) {
            bubble = bubble.push(editor);
        } else if long && !chat.expanded.contains(&index) {
            let head: String = body.chars().take(LONG_MESSAGE / 2).collect();
            bubble = bubble
                .push(text(format!("{}…", head.trim_end())).size(13))
                .push(
                    button(
                        text("Ver mensaje completo")
                            .size(11)
                            .color(design::accent()),
                    )
                    .padding(0)
                    .style(styles::plain(false))
                    .on_press(send(Event::Toggle(index))),
                );
        } else {
            bubble = bubble.push(text(body).size(13));
            if long {
                bubble = bubble.push(
                    button(text("Mostrar menos").size(11).color(design::accent()))
                        .padding(0)
                        .style(styles::plain(false))
                        .on_press(send(Event::Toggle(index))),
                );
            }
        }
    }
    if !body.trim().is_empty() {
        bubble = bubble.push(tools(
            index,
            chat.selecting.iter().any(|(e, _)| *e == index),
        ));
    }
    row![
        Space::new().width(Fill),
        container(bubble)
            .padding([9, 13])
            .max_width(620)
            .style(styles::tinted(design::brand_blue())),
    ]
    .into()
}

/// Small icon buttons under a message: copy it, or switch it to text that can be selected.
fn tools<'a>(index: usize, selecting: bool) -> Element<'a, Message> {
    row![
        styles::tip(
            button(icons::icon("file-text", design::faint(), 12.0))
                .padding(3)
                .style(styles::ghost(false))
                .on_press(send(Event::CopyEntry(index))),
            "Copiar el mensaje",
            iced::widget::tooltip::Position::Top,
        ),
        styles::tip(
            button(icons::icon("code", design::faint(), 12.0))
                .padding(3)
                .style(styles::ghost(selecting))
                .on_press(send(Event::Select(index))),
            if selecting {
                "Volver al texto con formato"
            } else {
                "Seleccionar texto (Ctrl+C copia lo seleccionado)"
            },
            iced::widget::tooltip::Position::Top,
        ),
    ]
    .spacing(2)
    .into()
}

/// The message as text the mouse can select; `None` while it is shown with its format.
fn selectable<'a>(chat: &'a Chat, index: usize, size: u32) -> Option<Element<'a, Message>> {
    let (_, content) = chat.selecting.iter().find(|(entry, _)| *entry == index)?;
    let to = super::sender();
    Some(
        iced::widget::text_editor(content)
            .on_action(move |action| to(Event::Selected(index, action)))
            .padding(0)
            .size(size as f32)
            .style(styles::composer_editor)
            .into(),
    )
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

/// "8 s", "2 min 5 s".
fn took(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds} s")
    } else {
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

/// An icon for what the tool does, so a long run reads at a glance.
fn tool_icon(kind: &str) -> &'static str {
    let kind = kind.to_lowercase();
    if ["bash", "shell", "command", "exec", "terminal"]
        .iter()
        .any(|k| kind.contains(k))
    {
        "terminal"
    } else if ["search", "grep", "glob", "find"]
        .iter()
        .any(|k| kind.contains(k))
    {
        "search"
    } else if ["edit", "write", "patch", "read", "file"]
        .iter()
        .any(|k| kind.contains(k))
    {
        "file-text"
    } else {
        "code"
    }
}

/// How a message's Markdown reacts: a link opens in the browser (web links only), and a code block can be copied.
struct LinkViewer;

impl<'a> markdown::Viewer<'a, Message> for LinkViewer {
    fn on_link_click(url: markdown::Uri) -> Message {
        send(Event::OpenLink(url))
    }

    fn code_block(
        &self,
        settings: markdown::Settings,
        _language: Option<&'a str>,
        code: &'a str,
        lines: &'a [markdown::Text],
    ) -> Element<'a, Message> {
        column![
            row![
                Space::new().width(Fill),
                styles::tip(
                    button(text("Copiar").size(10).color(design::muted()))
                        .padding([2, 8])
                        .style(styles::ghost(false))
                        .on_press(send(Event::CopyText(code.to_owned()))),
                    "Copiar este bloque",
                    iced::widget::tooltip::Position::Top,
                ),
            ],
            markdown::code_block(settings, lines, Self::on_link_click),
        ]
        .spacing(2)
        .into()
    }
}

/// The links and codes of a message as buttons: open the link, copy the link or the code.
fn offered<'a>(message: &str) -> Option<Element<'a, Message>> {
    use crate::features::chat_links::{Action, actions};
    if !message.contains("http") && !message.contains(['-', '`']) {
        return None;
    }
    let found = actions(message);
    if found.is_empty() {
        return None;
    }
    let chip = |label: String, message: Message, primary: bool| {
        button(text(label).size(11))
            .padding([4, 10])
            .style(if primary {
                styles::primary
            } else {
                styles::secondary
            })
            .on_press(message)
    };
    let mut chips = row![].spacing(6);
    for action in found {
        match action {
            Action::Link { url, label } => {
                chips = chips
                    .push(chip(
                        format!("Abrir {label}"),
                        send(Event::OpenLink(url.clone())),
                        true,
                    ))
                    .push(styles::tip(
                        chip(
                            "Copiar enlace".to_owned(),
                            send(Event::CopyText(url)),
                            false,
                        ),
                        "Para pegarlo en otro navegador o perfil",
                        iced::widget::tooltip::Position::Top,
                    ));
            }
            Action::Code { value } => {
                chips = chips.push(chip(
                    format!("Copiar código {value}"),
                    send(Event::CopyText(value)),
                    true,
                ));
            }
        }
    }
    Some(chips.wrap().into())
}

fn assistant<'a>(chat: &'a Chat, index: usize, entry: &'a Entry) -> Element<'a, Message> {
    let selecting = chat.selecting.iter().any(|(e, _)| *e == index);
    let body: Element<'a, Message> = match chat.rendered.get(index) {
        _ if selecting => selectable(chat, index, 13).unwrap_or_else(|| text("").into()),
        Some((_, content)) => markdown::view_with(
            content.items(),
            markdown::Settings::with_text_size(13, design::theme()),
            &LinkViewer,
        ),
        None => text(&entry.text).size(13).into(),
    };
    row![
        agents::avatar(chat.agent.provider, 26.0),
        column![
            row![
                text(
                    match chat.durations.iter().find(|(entry, _)| *entry == index) {
                        Some((_, seconds)) => {
                            format!("{} · {}", chat.agent.label(), took(*seconds))
                        }
                        None => chat.agent.label(),
                    }
                )
                .size(11)
                .font(design::UI_BOLD)
                .color(design::muted()),
                Space::new().width(Fill),
                tools(index, selecting),
            ]
            .align_y(iced::Center),
            body,
            offered(&entry.text).unwrap_or_else(|| Space::new().into()),
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
                design::faint(),
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
    .color(design::faint())
    .into();
    fold(
        chat,
        index,
        head,
        Some(text(body).size(11).color(design::faint()).into()),
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
        "completed" | "done" | "success" => ("check", design::success()),
        "failed" | "error" => ("warning", design::danger()),
        _ => ("refresh", design::accent()),
    };
    let head = row![
        icons::icon(icon, tint, 13.0),
        icons::icon(tool_icon(&entry.tool_kind), design::faint(), 12.0),
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
            .color(design::muted())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_and_tool_icons_read_at_a_glance() {
        assert_eq!((took(8).as_str(), took(125).as_str()), ("8 s", "2 min 5 s"));
        assert_eq!(tool_icon("Bash"), "terminal");
        assert_eq!(tool_icon("grep"), "search");
        assert_eq!(tool_icon("MultiEdit"), "file-text");
        assert_eq!(tool_icon("whatever"), "code");
    }
}
