//! The commit plan: one card per proposed commit, editable before saving.
use super::super::super::git::Event;
use super::event;
use crate::features::git::suggest::{CommitGroup, CommitType};
use crate::interfaces::studio::*;
use iced::widget::{column, text_input, tooltip};

pub(super) fn type_color(kind: CommitType) -> iced::Color {
    match kind {
        CommitType::Feat => design::SUCCESS,
        CommitType::Fix => design::DANGER,
        CommitType::Docs => design::BRAND_CYAN,
        CommitType::Test => design::TINT_PURPLE,
        CommitType::Build => design::TINT_ORANGE,
        CommitType::Ci => design::TINT_YELLOW,
        CommitType::Refactor => design::BRAND_BLUE,
        CommitType::Perf => design::WARNING,
        CommitType::Style | CommitType::Chore => design::MUTED,
    }
}

pub(super) fn view(state: &Dashboard) -> Option<Element<'_, Message>> {
    let git = &state.git;
    let plan = git.plan.as_ref()?;
    let blocked = git.busy.is_some() || git.merge;
    let mut cards = column![
        row![
            icons::icon("sparkle", design::ACCENT, 15.0),
            text("Plan de commits").size(13).font(design::UI_BOLD),
            text(plan.groups.len().to_string()).size(11).color(design::FAINT),
            Space::new().width(Fill),
            button(text("Guardar todos").size(11).font(design::UI_BOLD))
                .padding([4, 10])
                .style(styles::primary)
                .on_press_maybe((!blocked).then(|| event(Event::PlanCommitAll))),
            styles::tip(
                button(icons::icon("close", design::MUTED, 12.0))
                    .padding(4)
                    .style(styles::ghost(false))
                    .on_press(event(Event::PlanClose)),
                "Cerrar el plan",
                tooltip::Position::Left,
            ),
        ]
        .spacing(6)
        .align_y(iced::Center),
        text("Cada commit cuenta una sola historia. Cambia el tipo con un clic, edita la descripción y guarda.")
            .size(10)
            .color(design::FAINT),
    ]
    .spacing(8);
    for warning in &plan.warnings {
        cards = cards.push(
            row![
                icons::icon("warning", design::WARNING, 13.0),
                text(warning).size(10).color(design::WARNING).width(Fill),
            ]
            .spacing(6),
        );
    }
    for (index, group) in plan.groups.iter().enumerate() {
        cards = cards.push(card(index, group, blocked));
    }
    Some(
        container(cards)
            .padding(12)
            .width(Fill)
            .style(styles::tinted(design::BRAND_BLUE))
            .into(),
    )
}

fn card(index: usize, group: &CommitGroup, blocked: bool) -> Element<'_, Message> {
    let color = type_color(group.kind);
    let kind = styles::tip(
        button(text(group.prefix()).size(11).font(design::UI_BOLD))
            .padding([2, 8])
            .style(move |_: &iced::Theme, status: button::Status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                let mut style = styles::chip(color)(&iced::Theme::Dark);
                if hovered {
                    style.background = Some(iced::Background::Color(design::alpha(color, 0.28)));
                }
                button::Style {
                    background: style.background,
                    text_color: color,
                    border: style.border,
                    ..Default::default()
                }
            })
            .on_press(event(Event::PlanType(index))),
        group.kind.hint(),
        tooltip::Position::Top,
    );
    let names: Vec<String> = group
        .files
        .iter()
        .map(|path| path.rsplit('/').next().unwrap_or(path).to_owned())
        .collect();
    let preview = match names.len() {
        0..=3 => names.join(", "),
        count => format!("{}… +{}", names[..3].join(", "), count - 3),
    };
    let too_long = group.subject().chars().count() > 72;
    container(
        column![
            row![
                kind,
                Space::new().width(Fill),
                text(format!("{} archivo(s)", group.files.len()))
                    .size(10)
                    .color(design::FAINT),
            ]
            .align_y(iced::Center),
            text_input("Describe el cambio", &group.text)
                .size(11)
                .padding([5, 8])
                .style(styles::input)
                .on_input(move |value| event(Event::PlanText(index, value))),
            text(if too_long {
                "El título supera 72 caracteres: acórtalo.".to_owned()
            } else {
                preview
            })
            .size(10)
            .color(if too_long {
                design::WARNING
            } else {
                design::FAINT
            })
            .wrapping(iced::widget::text::Wrapping::None),
            button(text("Guardar este commit").size(11))
                .padding([4, 10])
                .style(styles::secondary)
                .on_press_maybe((!blocked).then(|| event(Event::PlanCommit(index)))),
        ]
        .spacing(6),
    )
    .padding(10)
    .width(Fill)
    .clip(true)
    .style(styles::card)
    .into()
}
