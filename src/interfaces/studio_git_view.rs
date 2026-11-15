//! Git sidebar frame: branch header, sync buttons, tabs, notices and confirmations.
use super::git::{Confirm, Event, Tab};
use super::*;
use iced::widget::{column, tooltip};

#[path = "studio_git_changes.rs"]
mod changes;
#[path = "studio_git_graph.rs"]
mod graph;
#[path = "studio_git_history.rs"]
mod history;
#[path = "studio_git_prompts.rs"]
mod prompts;
pub(in crate::interfaces::studio) use prompts::ago;
use prompts::{confirm_card, notice_card};

pub(super) fn event(event: Event) -> Message {
    Message::Git(event)
}

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let git = &state.git;
    let body: Element<'_, Message> = if let Some(reason) = &git.unavailable {
        empty(
            "warning",
            design::WARNING,
            "Git no está disponible",
            reason,
            None,
        )
    } else if !git.loaded {
        container(text("Leyendo Git…").size(12).color(design::MUTED))
            .padding([8, 14])
            .into()
    } else if !git.is_repository() {
        empty(
            "branch",
            design::BRAND_BLUE,
            "Este proyecto no es un repositorio Git",
            "Inicialízalo para guardar versiones, crear ramas y ver el historial.",
            Some(("Inicializar repositorio", event(Event::Init))),
        )
    } else {
        repository(state)
    };
    container(
        column![
            container(
                text("GIT")
                    .size(10)
                    .font(design::UI_BOLD)
                    .color(design::FAINT)
            )
            .padding([0, 12]),
            body,
        ]
        .spacing(8),
    )
    .padding([10, 0])
    .width(state.shell.explorer_width)
    .height(Fill)
    .style(styles::surface(design::BG_PANEL))
    .into()
}

fn empty<'a>(
    icon: &'static str,
    tint: iced::Color,
    title: &'a str,
    detail: &'a str,
    action: Option<(&'a str, Message)>,
) -> Element<'a, Message> {
    let mut card = column![
        row![
            icons::icon(icon, tint, 16.0),
            text(title).size(13).font(design::UI_BOLD)
        ]
        .spacing(8)
        .align_y(iced::Center),
        text(detail).size(12).color(design::MUTED),
    ]
    .spacing(10);
    if let Some((label, message)) = action {
        card = card.push(
            button(text(label).size(12).font(design::UI_BOLD))
                .padding([7, 14])
                .style(styles::primary)
                .on_press(message),
        );
    }
    container(container(card).padding(14).width(Fill).style(styles::card))
        .padding([0, 10])
        .into()
}

fn repository(state: &Dashboard) -> Element<'_, Message> {
    let git = &state.git;
    let mut content = column![header(state), tabs(state)].spacing(8);
    if let Some(label) = git.busy {
        content = content.push(
            container(
                row![
                    icons::icon("refresh", design::ACCENT, 12.0),
                    text(label).size(11).color(design::MUTED)
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .padding([0, 12]),
        );
    }
    if let Some(notice) = &git.notice {
        content = content.push(notice_card(notice.ok, notice.warn, &notice.text));
    }
    if let Some(confirm) = &git.confirm {
        content = content.push(confirm_card(state, confirm));
    }
    content = content.push(match git.tab {
        Tab::Changes => changes::view(state),
        Tab::History => history::log(state),
        Tab::Branches => history::branches(state),
    });
    content.height(Fill).into()
}

fn header(state: &Dashboard) -> Element<'_, Message> {
    let git = &state.git;
    let Some(status) = &git.status else {
        return Space::new().into();
    };
    let busy = git.busy.is_some();
    let mut branch = row![
        icons::icon("branch", design::BRAND_CYAN, 15.0),
        text(&status.head).size(13).font(design::UI_BOLD),
    ]
    .spacing(7)
    .align_y(iced::Center);
    if status.detached {
        branch = branch.push(text("(sin rama)").size(11).color(design::WARNING));
    }
    for (symbol, count, tint) in [
        ("↑", status.ahead, design::ACCENT),
        ("↓", status.behind, design::WARNING),
    ] {
        if count > 0 {
            branch = branch.push(
                container(
                    text(format!("{symbol}{count}"))
                        .size(10)
                        .font(design::UI_BOLD),
                )
                .padding([1, 6])
                .style(styles::chip(tint)),
            );
        }
    }
    let publish =
        status.upstream.is_none() && !git.remotes.is_empty() && !status.initial && !status.detached;
    let receive = sync_button(
        "arrow-down",
        &if status.behind > 0 {
            format!("Recibir {}", status.behind)
        } else {
            "Recibir".to_owned()
        },
        (!busy && status.upstream.is_some()).then(|| event(Event::Pull)),
        status.behind > 0,
    );
    let send = sync_button(
        "arrow-up",
        &if publish {
            "Publicar rama".to_owned()
        } else if status.ahead > 0 {
            format!("Enviar {}", status.ahead)
        } else {
            "Enviar".to_owned()
        },
        (!busy && (publish || status.ahead > 0)).then(|| {
            if publish {
                event(Event::Publish)
            } else {
                event(Event::Ask(Confirm::Push))
            }
        }),
        status.ahead > 0 || publish,
    );
    let refresh = styles::tip(
        button(icons::icon("refresh", design::MUTED, 14.0))
            .padding(6)
            .style(styles::ghost(false))
            .on_press_maybe((!busy).then(|| event(Event::Fetch))),
        "Buscar novedades en el remoto",
        tooltip::Position::Bottom,
    );
    column![
        container(branch).padding([0, 12]),
        container(row![receive, send, Space::new().width(Fill), refresh].spacing(6))
            .padding([0, 10]),
    ]
    .spacing(8)
    .into()
}

fn sync_button<'a>(
    icon: &'static str,
    label: &str,
    message: Option<Message>,
    highlight: bool,
) -> Element<'a, Message> {
    let tint = match (message.is_some(), highlight) {
        (false, _) => design::FAINT,
        (true, true) => design::ACCENT,
        (true, false) => design::MUTED,
    };
    button(
        row![
            icons::icon(icon, tint, 13.0),
            text(label.to_owned()).size(11)
        ]
        .spacing(6)
        .align_y(iced::Center),
    )
    .padding([5, 9])
    .style(styles::secondary)
    .on_press_maybe(message)
    .into()
}

fn tabs(state: &Dashboard) -> Element<'_, Message> {
    let git = &state.git;
    let tab = |label: String, target: Tab| {
        button(text(label).size(11).font(design::UI_BOLD))
            .padding([5, 10])
            .style(styles::ghost(git.tab == target))
            .on_press(event(Event::Tab(target)))
    };
    container(
        row![
            tab(format!("Cambios · {}", git.change_count()), Tab::Changes),
            tab("Historial".to_owned(), Tab::History),
            tab("Ramas".to_owned(), Tab::Branches),
        ]
        .spacing(2),
    )
    .padding([0, 10])
    .into()
}
