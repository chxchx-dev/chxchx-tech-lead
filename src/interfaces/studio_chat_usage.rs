//! Usage line of a chat: tokens, cost, context and the plan's limits.
use super::super::*;
use crate::features::chat_model::{Usage, Window, compact_tokens};
use iced::widget::{progress_bar, tooltip};

fn window_label(label: &str, window: &Window) -> String {
    let span = match window.window_minutes {
        0 => String::new(),
        m if m % 1440 == 0 => format!("{} d", m / 1440),
        m if m % 60 == 0 => format!("{} h", m / 60),
        m => format!("{m} min"),
    };
    let span = if span.is_empty() {
        label.to_owned()
    } else {
        span
    };
    format!("{span}: {:.0}%", window.used_percent)
}

pub(super) fn usage_bar(usage: &Usage) -> Element<'_, Message> {
    let mut line = row![].spacing(14).align_y(iced::Center);
    let summary = usage.summary();
    if !summary.is_empty() {
        line = line.push(text(summary).size(11).color(design::MUTED));
    }
    if let Some(percent) = usage.context_percent() {
        let tint = if percent >= 90.0 {
            design::DANGER
        } else if percent >= 70.0 {
            design::WARNING
        } else {
            design::ACCENT
        };
        line = line.push(styles::tip(
            row![
                text(format!(
                    "Contexto {percent:.0}% ({} de {})",
                    compact_tokens(usage.context_tokens),
                    compact_tokens(usage.context_window)
                ))
                .size(11)
                .color(tint),
                progress_bar(0.0..=100.0, percent as f32)
                    .length(90)
                    .girth(5)
                    .style(move |_: &iced::Theme| iced::widget::progress_bar::Style {
                        background: iced::Background::Color(design::BORDER),
                        bar: iced::Background::Color(tint),
                        border: iced::Border::default().rounded(3.0),
                    }),
            ]
            .spacing(8)
            .align_y(iced::Center),
            "Al llegar al 90 % se libera el contexto solo y la conversación sigue con un resumen.",
            tooltip::Position::Top,
        ));
    }
    for (label, window) in [("5 h", &usage.primary), ("semana", &usage.secondary)] {
        if let Some(window) = window {
            let plan = if usage.plan.is_empty() {
                String::new()
            } else {
                format!(" · {}", usage.plan)
            };
            line = line.push(
                text(format!("{}{plan}", window_label(label, window)))
                    .size(11)
                    .color(design::MUTED),
            );
        }
    }
    container(line)
        .padding([6, 16])
        .width(Fill)
        .style(styles::surface(design::BG_DEEP))
        .into()
}
