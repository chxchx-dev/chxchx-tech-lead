//! The model picker of a chat: search, models grouped by provider, and the effort levels.
use super::super::*;
use super::{Chat, Event, send};
use iced::widget::{column, text_input};

/// `provider/name` ids (OpenCode) split in two; the others have no provider.
pub(super) fn split_id(id: &str) -> (&str, &str) {
    id.split_once('/').unwrap_or(("", id))
}

/// What the chip of the header says: the model name, without its provider.
pub(super) fn short_name(id: &str) -> &str {
    split_id(id).1
}

fn matches(id: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || id.to_lowercase().contains(&query)
}

pub(super) fn panel(chat: &Chat) -> Element<'_, Message> {
    let Some(options) = chat.options.as_ref() else {
        return Space::new().into();
    };
    let to = super::sender();
    let search = text_input("Buscar modelo…", &chat.model_query)
        .on_input(move |query| to(Event::ModelQuery(query)))
        .size(12)
        .padding([7, 10])
        .style(styles::input);
    let mut list = column![].spacing(2);
    let mut group = "\u{0}";
    let mut shown = 0;
    for model in options
        .models
        .iter()
        .filter(|m| matches(&m.id, &chat.model_query))
    {
        let (provider, name) = split_id(&model.id);
        if provider != group {
            group = provider;
            if !provider.is_empty() {
                list = list.push(
                    container(
                        text(provider.to_uppercase())
                            .size(9)
                            .font(design::UI_BOLD)
                            .color(design::FAINT),
                    )
                    .padding(iced::Padding::new(8.0).bottom(2.0)),
                );
            }
        }
        let selected = chat.model_id.as_deref() == Some(model.id.as_str());
        list = list.push(
            button(
                row![
                    icons::icon(
                        if selected { "check" } else { "dot" },
                        if selected {
                            design::ACCENT
                        } else {
                            design::BORDER_STRONG
                        },
                        12.0
                    ),
                    text(name).size(12).font(if selected {
                        design::UI_BOLD
                    } else {
                        design::UI
                    }),
                    Space::new().width(Fill),
                    if model.efforts.is_empty() {
                        Element::from(Space::new())
                    } else {
                        text(format!("{} niveles", model.efforts.len()))
                            .size(10)
                            .color(design::FAINT)
                            .into()
                    },
                ]
                .spacing(8)
                .align_y(iced::Center),
            )
            .width(Fill)
            .padding([5, 8])
            .style(styles::ghost(selected))
            .on_press(send(Event::Model(model.id.clone()))),
        );
        shown += 1;
    }
    if shown == 0 {
        list = list.push(
            text("Ningún modelo coincide.")
                .size(11)
                .color(design::MUTED),
        );
    }
    let mut body = column![
        search,
        container(styles::vscroll(container(list).padding([0, 6])).height(iced::Length::Shrink))
            .max_height(240),
    ]
    .spacing(8);
    let efforts = options
        .models
        .iter()
        .find(|m| Some(&m.id) == chat.model_id.as_ref())
        .map(|m| m.efforts.clone())
        .unwrap_or_default();
    if !efforts.is_empty() {
        let mut levels = row![text("Esfuerzo").size(11).color(design::MUTED)]
            .spacing(6)
            .align_y(iced::Center);
        for level in efforts {
            let active = chat.effort.as_deref() == Some(level.as_str());
            levels = levels.push(
                button(text(level.clone()).size(11))
                    .padding([3, 9])
                    .style(if active {
                        styles::primary
                    } else {
                        styles::secondary
                    })
                    .on_press(send(Event::Effort(level))),
            );
        }
        body = body.push(
            iced::widget::scrollable(levels).direction(
                iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(3)
                        .scroller_width(3),
                ),
            ),
        );
    }
    if options.models.is_empty() {
        body = body.push(
            text("Este agente no informó modelos: usa el suyo por defecto.")
                .size(11)
                .color(design::MUTED),
        );
    }
    container(body)
        .padding([12, 16])
        .width(Fill)
        .style(styles::surface(design::BG_RAISED))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_are_split_and_the_search_ignores_case() {
        assert_eq!(split_id("opencode/big-pickle"), ("opencode", "big-pickle"));
        assert_eq!(split_id("sonnet"), ("", "sonnet"));
        assert_eq!(short_name("openai/gpt-5/mini"), "gpt-5/mini");
        assert!(
            matches("opencode/Big-Pickle", "PICKLE") && matches("x", "  ") && !matches("a", "b")
        );
    }
}
