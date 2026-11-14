//! File rows and section headers of the changes list.
use super::super::super::git::{Confirm, DiffTarget, Event};
use super::event;
use crate::features::git::GitEntry;
use crate::interfaces::studio::*;
use iced::widget::{column, tooltip};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Conflict,
    Staged,
    Changed,
}

fn tint(badge: char) -> iced::Color {
    match badge {
        'M' => design::WARNING,
        'N' => design::SUCCESS,
        'D' | '!' => design::DANGER,
        'R' | 'C' => design::BRAND_CYAN,
        _ => design::MUTED,
    }
}

pub(super) fn badge<'a>(letter: char, color: iced::Color) -> Element<'a, Message> {
    container(text(letter.to_string()).size(10).font(design::UI_BOLD))
        .width(18)
        .height(18)
        .center(18)
        .style(styles::chip(color))
        .into()
}

pub(super) fn small<'a>(
    icon: &'static str,
    hint: &'a str,
    message: Message,
) -> Element<'a, Message> {
    styles::tip(
        button(icons::icon(icon, design::MUTED, 13.0))
            .padding(4)
            .style(styles::ghost(false))
            .on_press(message),
        hint,
        tooltip::Position::Left,
    )
}

pub(super) fn bulk<'a>(
    icon: &'static str,
    hint: &'a str,
    message: Message,
) -> Element<'a, Message> {
    small(icon, hint, message)
}

pub(super) fn row_for(entry: &GitEntry, kind: Kind) -> Element<'_, Message> {
    let letter = if kind == Kind::Staged && entry.index != '.' && entry.index != 'A' {
        entry.index
    } else if kind == Kind::Staged {
        'N'
    } else {
        entry.badge()
    };
    let path = std::path::Path::new(&entry.path);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| entry.path.clone());
    let folder = path
        .parent()
        .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let target = match kind {
        Kind::Staged => DiffTarget::Staged(entry.path.clone()),
        _ if entry.untracked() => DiffTarget::Untracked(entry.path.clone()),
        _ => DiffTarget::Unstaged(entry.path.clone()),
    };
    let mut line = row![
        badge(letter, tint(letter)),
        button(
            row![
                text(name)
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::None),
                text(folder)
                    .size(10)
                    .color(design::FAINT)
                    .wrapping(iced::widget::text::Wrapping::None),
            ]
            .spacing(6)
            .align_y(iced::Center),
        )
        .width(Fill)
        .clip(true)
        .padding([3, 4])
        .style(styles::ghost(false))
        .on_press(event(Event::OpenDiff(target))),
    ]
    .spacing(4)
    .align_y(iced::Center);
    if letter != 'D' {
        line = line.push(small(
            "file",
            "Abrir el archivo",
            event(Event::OpenFile(entry.path.clone())),
        ));
    }
    match kind {
        Kind::Staged => {
            line = line.push(small(
                "minus",
                "Quitar de la lista",
                event(Event::Unstage(vec![entry.path.clone()])),
            ));
        }
        Kind::Changed | Kind::Conflict => {
            line = line
                .push(small(
                    "undo",
                    "Descartar (guarda una copia)",
                    event(Event::Ask(Confirm::Discard(vec![entry.clone()]))),
                ))
                .push(small(
                    "plus",
                    "Preparar para guardar",
                    event(Event::Stage(vec![entry.path.clone()])),
                ));
        }
    }
    line.into()
}

pub(super) fn section<'a>(
    icon: &'static str,
    color: iced::Color,
    title: &'a str,
    count: usize,
    actions: Vec<Element<'a, Message>>,
    rows: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut header = row![
        icons::icon(icon, color, 14.0),
        text(title).size(12).font(design::UI_BOLD),
        text(count.to_string()).size(11).color(design::FAINT),
        Space::new().width(Fill),
    ]
    .spacing(6)
    .align_y(iced::Center);
    for action in actions {
        header = header.push(action);
    }
    column![
        container(Space::new())
            .width(Fill)
            .height(1)
            .style(styles::surface(design::BORDER)),
        header,
        column(rows).spacing(1)
    ]
    .spacing(6)
    .into()
}
