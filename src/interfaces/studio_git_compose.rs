//! The guide card and the commit box.
use super::super::super::git::Event;
use super::event;
use crate::features::git::guide::Tone;
use crate::features::git::suggest::is_conventional;
use crate::interfaces::studio::*;
use iced::widget::{column, text_editor};

pub(super) fn guide_card(
    guide: &crate::features::git::guide::Guidance,
    open: bool,
) -> Element<'_, Message> {
    let tint = match guide.tone {
        Tone::Info => design::BRAND_CYAN,
        Tone::Warning => design::WARNING,
        Tone::Done => design::SUCCESS,
    };
    // Warnings always show every step; the rest folds away so the file list stays in view.
    let expanded = open || guide.tone == Tone::Warning || guide.steps.len() <= 1;
    let mut steps = column![].spacing(4);
    for (index, step) in guide
        .steps
        .iter()
        .enumerate()
        .take(if expanded { usize::MAX } else { 1 })
    {
        steps = steps.push(
            row![
                text(format!("{}.", index + 1))
                    .size(11)
                    .color(design::FAINT)
                    .width(16),
                text(step).size(11).color(design::MUTED)
            ]
            .spacing(2),
        );
    }
    let mut card = column![
        row![
            icons::icon(guide.icon, tint, 16.0),
            text(&guide.title)
                .size(12)
                .font(design::UI_BOLD)
                .width(Fill),
        ]
        .spacing(8)
        .align_y(iced::Center),
        steps,
    ]
    .spacing(8);
    if guide.tone != Tone::Warning && guide.steps.len() > 1 {
        card = card.push(
            button(
                row![
                    icons::icon(
                        if open {
                            "chevron-down"
                        } else {
                            "chevron-right"
                        },
                        design::MUTED,
                        11.0
                    ),
                    text(if open {
                        "Ocultar pasos"
                    } else {
                        "Ver todos los pasos"
                    })
                    .size(10)
                    .color(design::MUTED),
                ]
                .spacing(4)
                .align_y(iced::Center),
            )
            .padding([2, 4])
            .style(styles::ghost(false))
            .on_press(event(Event::ToggleGuide)),
        );
    }
    container(card)
        .padding(12)
        .width(Fill)
        .style(styles::tinted(tint))
        .into()
}

pub(super) fn commit_box(state: &Dashboard, staged: usize, changed: usize) -> Element<'_, Message> {
    let git = &state.git;
    let first_line = git
        .commit_box
        .line(0)
        .map(|line| line.text.to_string())
        .unwrap_or_default();
    let has_message = !git.commit_box.text().trim().is_empty();
    let all = staged == 0;
    let count = if all { changed } else { staged };
    let ready = has_message && count > 0 && git.busy.is_none();
    let label = if all {
        format!("Guardar todo · {count}")
    } else {
        format!("Guardar · {count}")
    };
    let (hint, tint) = if first_line.chars().count() > 72 {
        (
            "El título supera 72 caracteres: acórtalo y pasa el detalle al cuerpo.".to_owned(),
            design::WARNING,
        )
    } else if has_message && !is_conventional(&first_line) {
        ("Tip: usa «tipo(alcance): descripción», por ejemplo feat(rust): añade Git. Pulsa Sugerir y te lo propongo.".to_owned(), design::MUTED)
    } else if all {
        (
            "No hay nada preparado: se guardarán todos los cambios. Ctrl+Intro guarda.".to_owned(),
            design::FAINT,
        )
    } else {
        (
            "Se guardarán solo los archivos de «Listos para guardar». Ctrl+Intro guarda."
                .to_owned(),
            design::FAINT,
        )
    };
    column![
        text_editor(&git.commit_box)
            .placeholder("Describe qué cambiaste. Ejemplo: fix(login): corrige el error al guardar")
            .on_action(|action| event(Event::Edit(action)))
            .height(84)
            .padding(8)
            .size(12)
            .style(styles::message_box)
            .key_binding(|key| {
                if key.modifiers.command()
                    && matches!(
                        key.key.as_ref(),
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
                    )
                {
                    Some(text_editor::Binding::Custom(event(Event::Commit {
                        all: false,
                    })))
                } else {
                    text_editor::Binding::from_key_press(key)
                }
            }),
        row![
            button(
                row![
                    icons::icon("sparkle", design::ACCENT, 13.0),
                    text("Sugerir").size(12)
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .padding([7, 12])
            .style(styles::secondary)
            .on_press_maybe(
                (git.busy.is_none() && staged + changed > 0).then(|| event(Event::Suggest))
            ),
            button(text(label).size(12).font(design::UI_BOLD).center())
                .width(Fill)
                .padding([7, 10])
                .style(styles::primary)
                .on_press_maybe(ready.then(|| event(Event::Commit { all }))),
        ]
        .spacing(6),
        text(hint).size(10).color(tint),
    ]
    .spacing(6)
    .into()
}
