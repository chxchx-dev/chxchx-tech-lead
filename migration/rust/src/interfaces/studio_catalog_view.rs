//! Catalog page layout: tabs, filter, previews and the empty state.
use super::pages::{badge, eyebrow};
use super::rows;
use super::*;
use iced::widget::{column, text_input};

pub(super) fn matches(filter: &str, parts: &[&str]) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty()
        || parts
            .iter()
            .any(|part| part.to_lowercase().contains(&filter))
}

pub(super) fn chip<'a>(
    label: impl text::IntoFragment<'a>,
    tint: iced::Color,
) -> Element<'a, Message> {
    container(text(label).size(10).font(design::UI_BOLD))
        .padding([2, 8])
        .style(styles::chip(tint))
        .into()
}

pub(super) fn action<'a>(
    label: &'a str,
    style: fn(&iced::Theme, button::Status) -> button::Style,
    message: Message,
) -> Element<'a, Message> {
    button(text(label).size(12))
        .padding([6, 12])
        .style(style)
        .on_press(message)
        .into()
}

pub(super) fn item<'a>(
    name: &'a str,
    description: &'a str,
    chips: Vec<Element<'a, Message>>,
    notes: Vec<String>,
    button: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut details = column![
        row![
            text(name).size(14).font(design::UI_BOLD),
            row(chips).spacing(6)
        ]
        .spacing(10)
        .align_y(iced::Center),
        text(description).size(12).color(design::MUTED),
    ]
    .spacing(5)
    .width(Fill);
    if !notes.is_empty() {
        details = details.push(text(notes.join(" · ")).size(11).color(design::FAINT));
    }
    container(row![details, button].spacing(14).align_y(iced::Center))
        .padding([12, 16])
        .width(Fill)
        .style(styles::card)
        .into()
}

pub(in crate::interfaces::studio) fn view(state: &Dashboard) -> Element<'_, Message> {
    let catalog = &state.catalog;
    let Some(data) = &catalog.view else {
        let label = if catalog.loading {
            "Leyendo el catálogo…"
        } else {
            "Carga el catálogo del proyecto."
        };
        return container(
            row![
                icons::icon("layers", design::ACCENT, 16.0),
                text(label).size(13).color(design::MUTED),
                action("Cargar", styles::secondary, Message::Catalog(Event::Load)),
            ]
            .spacing(10)
            .align_y(iced::Center),
        )
        .padding(20)
        .width(Fill)
        .style(styles::card)
        .into();
    };
    let tab = |label: String, target: Tab| {
        button(text(label).size(12).font(design::UI_BOLD))
            .padding([7, 14])
            .style(styles::ghost(catalog.tab == target))
            .on_press(Message::Catalog(Event::Select(target)))
    };
    let mut content = column![
        row![
            tab(format!("Skills · {}", data.skills.len()), Tab::Skills),
            tab(format!("Tech Packs · {}", data.packs.len()), Tab::Packs),
            tab(format!("Mods · {}", data.mods.len()), Tab::Mods),
            Space::new().width(Fill),
            text_input("Filtrar…", &catalog.filter)
                .size(12)
                .padding([6, 10])
                .width(220)
                .style(styles::input)
                .on_input(|value| Message::Catalog(Event::Filter(value))),
        ]
        .spacing(4)
        .align_y(iced::Center),
    ]
    .spacing(12);
    if let Some(error) = &data.selection_error {
        content = content.push(pages::banner("warning", design::WARNING, error.clone()));
    }
    if let Some(plan) = &catalog.skills_plan {
        content = content.push(review(
            "Cambios de skills",
            plan.actions.clone(),
            plan.changes_state(),
            catalog.busy,
            Event::ApplySkills,
        ));
    }
    if let Some(plan) = &catalog.mods_plan {
        let lines = match &plan.blocked_reason {
            Some(reason) => vec![format!("No se puede aplicar: {reason}")],
            None => plan.actions.clone(),
        };
        content = content.push(review(
            "Cambios de Mods",
            lines,
            plan.changes_state() && plan.blocked_reason.is_none(),
            catalog.busy,
            Event::ApplyMods,
        ));
    }
    let filter = catalog.filter.as_str();
    let rows = match catalog.tab {
        Tab::Skills => rows::skills(data, filter),
        Tab::Packs => rows::packs(data, filter),
        Tab::Mods => rows::mods(data, filter),
    };
    content.push(rows).into()
}

pub(in crate::interfaces::studio) fn header<'a>() -> Element<'a, Message> {
    row![
        badge("layers", 22.0),
        column![
            text("Catálogo del proyecto").size(22).font(design::UI_BOLD),
            text("Skills, Tech Packs y Mods que guían a tus agentes. Cada cambio se previsualiza y respalda.")
                .size(13)
                .color(design::MUTED)
        ]
        .spacing(4),
    ]
    .spacing(14)
    .align_y(iced::Center)
    .into()
}

pub(super) fn bulk<'a>(
    label: String,
    button_label: &'a str,
    message: Message,
) -> Element<'a, Message> {
    container(
        row![
            icons::icon("sparkle", design::ACCENT, 15.0),
            text(label).size(12),
            Space::new().width(Fill),
            action(button_label, styles::primary, message),
        ]
        .spacing(10)
        .align_y(iced::Center),
    )
    .padding([10, 16])
    .width(Fill)
    .style(styles::hero)
    .into()
}

fn review<'a>(
    title: &'a str,
    actions: Vec<String>,
    can_apply: bool,
    busy: bool,
    apply: Event,
) -> Element<'a, Message> {
    let mut lines = column![
        eyebrow("VISTA PREVIA"),
        text(title).size(15).font(design::UI_BOLD)
    ]
    .spacing(6);
    for line in actions {
        lines = lines.push(
            row![
                icons::icon("check", design::SUCCESS, 13.0),
                text(line).size(12)
            ]
            .spacing(8)
            .align_y(iced::Center),
        );
    }
    if can_apply {
        lines = lines.push(if busy {
            Element::from(text("Aplicando…").size(12).color(design::MUTED))
        } else {
            action(
                "Confirmar y aplicar",
                styles::primary,
                Message::Catalog(apply),
            )
        });
    }
    container(lines.spacing(8))
        .padding(16)
        .width(Fill)
        .style(styles::hero)
        .into()
}
