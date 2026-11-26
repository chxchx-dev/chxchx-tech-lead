//! The Kanban of Colmena in the main area: one column per status, a detail strip for the card
//! selected. It only emits events; the rules live in `features::hive::board`.
use super::*;
use iced::widget::{column, text_input};

/// Whether the board takes the place of the editor group.
pub(in crate::interfaces) fn takes_center(state: &Dashboard) -> bool {
    super::visible(state) && state.hive.wide && state.shell.side == shell::Side::Hive
}

fn tile<'a>(item: &'a board::Card, selected: bool) -> Element<'a, Message> {
    let mut body = column![text(format!("{} · {}", item.key, item.title)).size(12)].spacing(3);
    if !item.blocker.is_empty() {
        body = body.push(text(&item.blocker).size(11).color(design::danger()));
    } else if !item.waits_for.is_empty() || !item.depends_on.is_empty() {
        let mut parts = Vec::new();
        if !item.depends_on.is_empty() {
            parts.push(format!("depende de {}", item.depends_on.join(", ")));
        }
        if !item.waits_for.is_empty() {
            parts.push(format!("espera {}", item.waits_for.join(", ")));
        }
        body = body.push(text(parts.join(" · ")).size(11).color(design::muted()));
    }
    let target = if selected {
        None
    } else {
        Some(item.key.clone())
    };
    button(body)
        .width(Fill)
        .padding([6, 8])
        .style(if selected {
            styles::primary
        } else {
            styles::secondary
        })
        .on_press(Message::Hive(Event::Select(target)))
        .into()
}

fn detail<'a>(hive: &'a State, item: &'a board::Card) -> Element<'a, Message> {
    let at = board::STATUSES
        .iter()
        .position(|s| *s == item.status)
        .unwrap_or(0);
    let mut moves = row![].spacing(4);
    for (index, status) in board::STATUSES.iter().enumerate() {
        if index != at && *status != "blocked" {
            moves = moves.push(small(
                status_name(status),
                Message::Hive(Event::Move(item.key.clone(), (*status).into())),
            ));
        }
    }
    let mut waits = row![].spacing(4);
    for d in hive.decisions.iter().filter(|d| d.status == "proposed") {
        if !item.waits_for.contains(&d.key) {
            waits = waits.push(small(&d.key, Message::Hive(Event::WaitFor(d.key.clone()))));
        }
    }
    container(
        column![
            row![
                text(format!("{} · {}", item.key, item.title))
                    .size(13)
                    .font(design::UI_BOLD)
                    .width(Fill),
                small("Cerrar", Message::Hive(Event::Select(None))),
            ],
            text("Mover a").size(11).color(design::faint()),
            moves.wrap(),
            row![
                text_input("Depende de (C-002)", &hive.dependency)
                    .size(12)
                    .on_input(|v| Message::Hive(Event::DependencyInput(v)))
                    .on_submit(Message::Hive(Event::AddDependency)),
                small("Añadir dependencia", Message::Hive(Event::AddDependency)),
            ]
            .spacing(6),
            row![
                text_input("Motivo del bloqueo", &hive.reason)
                    .size(12)
                    .on_input(|v| Message::Hive(Event::ReasonInput(v)))
                    .on_submit(Message::Hive(Event::Block)),
                small("Bloquear", Message::Hive(Event::Block)),
            ]
            .spacing(6),
            text("Esperar una decisión").size(11).color(design::faint()),
            waits.wrap(),
        ]
        .spacing(6),
    )
    .padding(10)
    .width(Fill)
    .style(styles::card)
    .into()
}

pub(in crate::interfaces) fn board_view(state: &Dashboard) -> Element<'_, Message> {
    let hive = &state.hive;
    let mut columns = row![].spacing(10);
    for status in board::STATUSES {
        let cards: Vec<&board::Card> = hive.cards.iter().filter(|c| c.status == status).collect();
        let mut lane = column![label(status_name(status), cards.len())].spacing(6);
        for item in cards {
            let on = hive.selected.as_deref() == Some(item.key.as_str());
            lane = lane.push(tile(item, on));
        }
        columns = columns.push(
            container(styles::vscroll(lane).height(Fill))
                .width(230)
                .height(Fill)
                .padding(8)
                .style(styles::surface(design::bg_panel())),
        );
    }
    let mut top = column![
        row![
            text("Colmena · tablero").size(15).font(design::UI_BOLD),
            Space::new().width(Fill),
            small("Exportar .ai/board.json", Message::Hive(Event::Export)),
            small("Cerrar tablero", Message::Hive(Event::Wide(false))),
        ]
        .spacing(8)
        .align_y(iced::Center)
    ]
    .spacing(8);
    if !hive.error.is_empty() {
        top = top.push(text(&hive.error).size(11).color(design::danger()));
    }
    if !hive.released.is_empty() {
        top = top.push(
            text(format!("Liberó: {}", hive.released.join(", ")))
                .size(11)
                .color(design::muted()),
        );
    }
    for item in hive.decisions.iter().filter(|d| d.status == "proposed") {
        top = top.push(decision_card(item));
    }
    if let Some(item) = hive
        .selected
        .as_deref()
        .and_then(|key| hive.cards.iter().find(|c| c.key == key))
    {
        top = top.push(detail(hive, item));
    }
    container(
        column![
            styles::vscroll(top).height(iced::Shrink),
            styles::hscroll(columns).height(Fill)
        ]
        .spacing(10),
    )
    .padding(12)
    .width(Fill)
    .height(Fill)
    .style(styles::surface(design::bg_deep()))
    .into()
}
