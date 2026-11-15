//! "Historial" and "Ramas" tabs.
use super::super::git::{Confirm, DiffTarget, Event};
use super::{ago, event, graph};
use crate::interfaces::studio::*;
use iced::widget::{Canvas, column, text_input, tooltip};

pub(super) fn log(state: &Dashboard) -> Element<'_, Message> {
    let git = &state.git;
    if git.log.is_empty() {
        return container(
            text("Todavía no hay commits.")
                .size(12)
                .color(design::MUTED),
        )
        .padding([4, 14])
        .into();
    }
    let lanes = git
        .log
        .iter()
        .map(|commit| commit.graph.lane_count)
        .max()
        .unwrap_or(1);
    let mut list = column![].spacing(0);
    for (index, commit) in git.log.iter().enumerate() {
        let selected = git.selected_commit == Some(index);
        let head = commit.refs.iter().any(|name| name.starts_with("HEAD"));
        // Branch and tag chips come first: a long message must never push them out of view.
        let mut subject = row![].spacing(5).align_y(iced::Center);
        for name in commit.refs.iter().take(2) {
            subject = subject.push(ref_chip(name));
        }
        subject = subject.push(
            text(&commit.subject)
                .size(12)
                .font(if head { design::UI_BOLD } else { design::UI })
                .wrapping(iced::widget::text::Wrapping::None),
        );
        let meta = text(format!(
            "{} · {} · {}",
            commit.short_hash(),
            commit.author,
            ago(commit.time)
        ))
        .size(10)
        .color(design::FAINT)
        .wrapping(iced::widget::text::Wrapping::None);
        let line = styles::tip(
            button(column![subject, meta].spacing(3).width(Fill))
                .width(Fill)
                .height(graph::ROW_HEIGHT)
                .clip(true)
                .padding([5, 6])
                .style(styles::ghost(selected))
                .on_press(event(Event::SelectCommit(index))),
            &commit.subject,
            tooltip::Position::Right,
        );
        let mut body = column![line].width(Fill);
        if selected {
            body = body.push(details(state, &commit.hash));
        }
        // The graph is as tall as the row and its details, so lanes run through them.
        list = list.push(
            row![
                Canvas::new(graph::Graph {
                    row: commit.graph.clone(),
                    lanes,
                    head,
                    selected,
                })
                .width(graph::width(lanes))
                .height(Fill),
                body,
            ]
            .width(Fill),
        );
    }
    styles::vscroll(container(list).padding([0, 6]))
        .height(Fill)
        .into()
}

fn ref_chip(name: &str) -> Element<'_, Message> {
    let (label, tint) = if let Some(head) = name.strip_prefix("HEAD -> ") {
        (head, design::ACCENT)
    } else if let Some(tag) = name.strip_prefix("tag: ") {
        (tag, design::WARNING)
    } else if name.contains('/') {
        (name, design::TINT_PURPLE)
    } else {
        (name, design::BRAND_BLUE)
    };
    container(
        text(label)
            .size(9)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .padding([1, 6])
    .style(styles::chip(tint))
    .into()
}

fn details<'a>(state: &'a Dashboard, hash: &'a str) -> Element<'a, Message> {
    let Some((message, files)) = &state.git.commit_detail else {
        return container(text("Leyendo el commit…").size(11).color(design::MUTED))
            .padding([6, 12])
            .into();
    };
    let mut body = column![text(message).size(11)].spacing(6);
    for file in files {
        let letter = file.status.chars().next().unwrap_or('M');
        let tint = match letter {
            'A' => design::SUCCESS,
            'D' => design::DANGER,
            'R' | 'C' => design::BRAND_CYAN,
            _ => design::WARNING,
        };
        body = body.push(
            button(
                row![
                    container(text(letter.to_string()).size(9).font(design::UI_BOLD))
                        .width(16)
                        .height(16)
                        .center(16)
                        .style(styles::chip(tint)),
                    text(&file.path)
                        .size(11)
                        .wrapping(iced::widget::text::Wrapping::None),
                ]
                .spacing(6)
                .align_y(iced::Center),
            )
            .width(Fill)
            .clip(true)
            .padding([2, 4])
            .style(styles::ghost(false))
            .on_press(event(Event::OpenDiff(DiffTarget::Commit(
                hash.to_owned(),
                file.path.clone(),
            )))),
        );
    }
    container(body)
        .padding(10)
        .width(Fill)
        .style(styles::well)
        .into()
}

pub(super) fn branches(state: &Dashboard) -> Element<'_, Message> {
    let git = &state.git;
    let creating = !git.new_branch.trim().is_empty();
    let mut list = column![
        row![
            text_input("Nombre de la rama nueva…", &git.new_branch)
                .size(11)
                .padding([5, 8])
                .style(styles::input)
                .on_input(|name| event(Event::NewBranch(name)))
                .on_submit(event(Event::CreateBranch)),
            styles::tip(
                button(icons::icon("plus", design::TEXT, 13.0))
                    .padding(6)
                    .style(styles::primary)
                    .on_press_maybe(creating.then(|| event(Event::CreateBranch))),
                "Crear rama y cambiar a ella",
                tooltip::Position::Left,
            ),
        ]
        .spacing(6)
        .align_y(iced::Center),
    ]
    .spacing(6);
    for remote in [false, true] {
        let group: Vec<_> = git
            .branches
            .iter()
            .filter(|branch| branch.remote == remote)
            .collect();
        if group.is_empty() {
            continue;
        }
        list = list.push(
            text(if remote { "REMOTAS" } else { "LOCALES" })
                .size(10)
                .font(design::UI_BOLD)
                .color(design::FAINT),
        );
        for branch in group {
            let mut line = row![
                icons::icon(
                    "branch",
                    if branch.current {
                        design::ACCENT
                    } else {
                        design::MUTED
                    },
                    13.0
                ),
                column![
                    text(&branch.name)
                        .size(12)
                        .font(if branch.current {
                            design::UI_BOLD
                        } else {
                            design::UI
                        })
                        .wrapping(iced::widget::text::Wrapping::None),
                    text(if branch.track.is_empty() {
                        branch.subject.clone()
                    } else {
                        translate(&branch.track)
                    })
                    .size(10)
                    .color(design::FAINT)
                    .wrapping(iced::widget::text::Wrapping::None),
                ]
                .width(Fill),
            ]
            .spacing(8)
            .align_y(iced::Center);
            if branch.current {
                line = line.push(
                    container(text("ACTUAL").size(9).font(design::UI_BOLD))
                        .padding([1, 6])
                        .style(styles::chip(design::ACCENT)),
                );
            } else {
                let target = event(Event::Switch(branch.name.clone(), branch.remote));
                line = line.push(styles::tip(
                    button(text("Cambiar").size(10))
                        .padding([3, 8])
                        .style(styles::secondary)
                        .on_press_maybe(git.busy.is_none().then_some(target)),
                    "Pasar a esta rama",
                    tooltip::Position::Left,
                ));
                if !branch.remote {
                    line = line.push(styles::tip(
                        button(icons::icon("trash", design::MUTED, 13.0))
                            .padding(4)
                            .style(styles::ghost(false))
                            .on_press(event(Event::Ask(Confirm::DeleteBranch(
                                branch.name.clone(),
                            )))),
                        "Eliminar rama",
                        tooltip::Position::Left,
                    ));
                }
            }
            list = list.push(container(line).padding([3, 4]).clip(true));
        }
    }
    styles::vscroll(container(list).padding([0, 10]))
        .height(Fill)
        .into()
}

/// "ahead 2, behind 1" -> "2 por enviar, 1 por recibir".
fn translate(track: &str) -> String {
    track
        .replace("ahead", "por enviar:")
        .replace("behind", "por recibir:")
        .replace("gone", "remota eliminada")
}
