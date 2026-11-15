//! Notices and inline confirmations of the Git panel, and relative times.
use super::super::git::{Confirm, Event};
use super::event;
use crate::interfaces::studio::*;
use iced::widget::column;

pub(super) fn notice_card<'a>(ok: bool, warn: bool, message: &'a str) -> Element<'a, Message> {
    let tint = if warn {
        design::WARNING
    } else if ok {
        design::SUCCESS
    } else {
        design::DANGER
    };
    container(
        row![
            icons::icon(if ok && !warn { "check" } else { "warning" }, tint, 14.0),
            text(message).size(11).width(Fill),
            button(icons::icon("close", design::MUTED, 11.0))
                .padding(3)
                .style(styles::ghost(false))
                .on_press(event(Event::DismissNotice)),
        ]
        .spacing(8)
        .align_y(iced::Center),
    )
    .padding([7, 10])
    .width(Fill)
    .style(styles::tinted(tint))
    .into()
}

pub(super) fn confirm_card<'a>(state: &'a Dashboard, confirm: &'a Confirm) -> Element<'a, Message> {
    let (question, label) = match confirm {
        Confirm::Discard(entries) => (
            format!(
                "¿Descartar {} archivo(s)? Se guarda una copia de seguridad dentro de .git antes de borrar.",
                entries.len()
            ),
            "Descartar",
        ),
        Confirm::Push => (
            format!(
                "¿Enviar {} commit(s) a «{}»? Quedarán visibles para el equipo.",
                state.git.status.as_ref().map_or(0, |status| status.ahead),
                state
                    .git
                    .status
                    .as_ref()
                    .and_then(|status| status.upstream.clone())
                    .unwrap_or_default()
            ),
            "Enviar",
        ),
        Confirm::PullMerge => (
            "Tu rama y el remoto se separaron. ¿Fusionarlos (merge)? Si hay conflictos te guiaré."
                .to_owned(),
            "Fusionar",
        ),
        Confirm::DeleteBranch(name) => (format!("¿Eliminar la rama «{name}»?"), "Eliminar"),
        Confirm::DropStash(index) => (
            format!("¿Eliminar el apartado {index}? No se puede deshacer."),
            "Eliminar",
        ),
        Confirm::AbortMerge => (
            "¿Cancelar la fusión? Vuelves a como estabas antes de recibir.".to_owned(),
            "Cancelar fusión",
        ),
    };
    let danger = matches!(
        confirm,
        Confirm::Discard(_) | Confirm::DeleteBranch(_) | Confirm::DropStash(_)
    );
    container(
        column![
            text(question).size(11),
            row![
                button(text("Volver").size(11))
                    .padding([5, 10])
                    .style(styles::secondary)
                    .on_press(event(Event::Cancel)),
                button(text(label).size(11).font(design::UI_BOLD))
                    .padding([5, 10])
                    .style(if danger {
                        styles::danger
                    } else {
                        styles::primary
                    })
                    .on_press(event(Event::Confirm)),
            ]
            .spacing(6),
        ]
        .spacing(8),
    )
    .padding(10)
    .width(Fill)
    .style(styles::tinted(design::WARNING))
    .into()
}

/// "hace 4 h" from a Unix timestamp.
pub(in crate::interfaces::studio) fn ago(seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    let delta = (now - seconds).max(0);
    match delta {
        0..=59 => "ahora".to_owned(),
        60..=3599 => format!("hace {} min", delta / 60),
        3600..=86_399 => format!("hace {} h", delta / 3600),
        86_400..=2_591_999 => format!("hace {} d", delta / 86_400),
        _ => format!("hace {} meses", delta / 2_592_000),
    }
}
