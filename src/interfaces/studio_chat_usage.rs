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
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    // A window that already reset says so instead of showing last period's figure.
    if window.resets_at > 0 && window.resets_at <= now {
        return format!("{span}: reiniciado");
    }
    let left = (100.0 - window.used_percent).max(0.0);
    let again = if window.resets_at > now {
        let minutes = (window.resets_at - now) / 60;
        match minutes {
            m if m >= 1440 => format!(" · reinicia en {} d {} h", m / 1440, m % 1440 / 60),
            m if m >= 60 => format!(" · reinicia en {} h {} min", m / 60, m % 60),
            m => format!(" · reinicia en {m} min"),
        }
    } else {
        String::new()
    };
    format!("{span}: quedan {left:.0}%{again}")
}

/// What the running turn has used so far, and with which model when plan mode chose one.
pub(super) struct Running {
    pub input: i64,
    pub output: i64,
    pub label: String,
}

pub(super) fn usage_bar<'a>(
    usage: &'a Usage,
    spend: &[crate::features::model_route::Spent],
    running: Option<Running>,
    refresh: Option<Message>,
) -> Element<'a, Message> {
    let mut line = row![].spacing(14).align_y(iced::Center);
    let summary = usage.summary();
    if !summary.is_empty() {
        line = line.push(text(summary).size(11).color(design::muted()));
    }
    if let Some(now) = running {
        line = line.push(styles::tip(
            text(format!(
                "En curso{}: {} de entrada · {} de salida",
                if now.label.is_empty() { String::new() } else { format!(" ({})", now.label) },
                compact_tokens(now.input),
                compact_tokens(now.output)
            ))
            .size(11)
            .color(design::accent()),
            "Lo que va usando este turno; al terminar se suma a los totales con las cifras finales del agente.",
            tooltip::Position::Top,
        ));
    }
    let by_model = crate::features::model_route::spend_summary(spend);
    if !by_model.is_empty() {
        line = line.push(styles::tip(
            text(by_model).size(11).color(design::accent()),
            "Lo que gastó cada modelo en el modo plan: entrada/salida y, si el agente lo informa, el costo.",
            tooltip::Position::Top,
        ));
    }
    if let Some(percent) = usage.context_percent() {
        let tint = if percent >= 90.0 {
            design::danger()
        } else if percent >= 70.0 {
            design::warning()
        } else {
            design::accent()
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
                        background: iced::Background::Color(design::border()),
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
    let mut any = false;
    for (label, window) in [("5 h", &usage.primary), ("semana", &usage.secondary)] {
        if let Some(window) = window {
            any = true;
            line = line.push(
                text(window_label(label, window))
                    .size(11)
                    .color(design::muted()),
            );
        }
    }
    if any && !usage.plan.is_empty() {
        line = line.push(text(usage.plan.clone()).size(11).color(design::faint()));
    }
    if let Some(message) = refresh {
        let hint = if usage.limits_at > 0 {
            "Vuelve a leer lo que queda de tu plan. Solo lee archivos locales: no gasta tokens ni hace peticiones."
        } else {
            "Lee lo que queda de tu plan (5 h y semana). Solo lee archivos locales y solo cuando lo pides."
        };
        line = line.push(styles::tip(
            button(
                text(if any {
                    "Actualizar límites"
                } else {
                    "Ver límites"
                })
                .size(11),
            )
            .padding([2, 8])
            .style(styles::secondary)
            .on_press(message),
            hint,
            tooltip::Position::Top,
        ));
    }
    container(line.wrap())
        .padding([6, 16])
        .width(Fill)
        .style(styles::surface(design::bg_deep()))
        .into()
}
