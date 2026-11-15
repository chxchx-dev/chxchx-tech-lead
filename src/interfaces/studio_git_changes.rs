//! "Cambios" tab: the guide, the commit box and the lists of files.
use super::super::git::{Confirm, Event};
#[path = "studio_git_compose.rs"]
mod compose;
#[path = "studio_git_plan_view.rs"]
mod plan_view;
#[path = "studio_git_rows.rs"]
mod rows;
use super::{ago, event};
use crate::features::git::GitEntry;
use crate::interfaces::studio::*;
use compose::{commit_box, guide_card};
use iced::widget::column;
use rows::{Kind, badge, bulk, row_for, section, small};

/// Id of the scrolling list, so tests and shortcuts can move it.
pub(in crate::interfaces::studio) const SCROLL: &str = "git-changes";

pub(super) fn view(state: &Dashboard) -> Element<'_, Message> {
    let git = &state.git;
    let Some(status) = &git.status else {
        return Space::new().into();
    };
    let staged = status.staged_entries();
    let changed = status.changed_entries();
    let conflicts = status.conflicts();
    let mut list = column![].spacing(10);
    if let Some(guide) = &git.guidance {
        list = list.push(guide_card(guide, git.guide_open));
    }
    if git.merge {
        list = list.push(
            button(text("Cancelar la fusión").size(11))
                .padding([6, 10])
                .style(styles::danger)
                .on_press(event(Event::Ask(Confirm::AbortMerge))),
        );
    }
    if let Some(plan) = plan_view::view(state) {
        list = list.push(plan);
    } else if staged.len() + changed.len() >= 2 && !git.merge {
        list = list.push(
            button(
                row![
                    icons::icon("sparkle", design::ACCENT, 14.0),
                    column![
                        text("Organizar en commits").size(12).font(design::UI_BOLD),
                        text("Agrupa por tipo (feat, fix, docs…) y módulo")
                            .size(10)
                            .color(design::FAINT),
                    ]
                    .spacing(1),
                ]
                .spacing(10)
                .align_y(iced::Center),
            )
            .width(Fill)
            .padding([8, 12])
            .style(styles::secondary)
            .on_press_maybe(git.busy.is_none().then(|| event(Event::MakePlan))),
        );
    }
    if !conflicts.is_empty() {
        list = list.push(section(
            "warning",
            design::DANGER,
            "En conflicto",
            conflicts.len(),
            Vec::new(),
            conflicts
                .iter()
                .map(|entry| row_for(entry, Kind::Conflict))
                .collect(),
        ));
    }
    if !staged.is_empty() {
        let paths: Vec<String> = staged.iter().map(|entry| entry.path.clone()).collect();
        list = list.push(section(
            "git-commit",
            design::BRAND_CYAN,
            "Listos para guardar",
            staged.len(),
            vec![bulk(
                "minus",
                "Quitar todos de la lista",
                event(Event::Unstage(paths)),
            )],
            staged
                .iter()
                .map(|entry| row_for(entry, Kind::Staged))
                .collect(),
        ));
    }
    if !changed.is_empty() {
        let paths: Vec<String> = changed.iter().map(|entry| entry.path.clone()).collect();
        let entries: Vec<GitEntry> = changed.iter().map(|entry| (*entry).clone()).collect();
        list = list.push(section(
            "file-text",
            design::MUTED,
            "Cambios",
            changed.len(),
            vec![
                bulk(
                    "archive",
                    "Apartar cambios (stash)",
                    event(Event::StashPush),
                ),
                bulk(
                    "undo",
                    "Descartar todos",
                    event(Event::Ask(Confirm::Discard(entries))),
                ),
                bulk("plus", "Preparar todos", event(Event::Stage(paths))),
            ],
            changed
                .iter()
                .map(|entry| row_for(entry, Kind::Changed))
                .collect(),
        ));
    }
    if !git.stashes.is_empty() {
        let rows = git
            .stashes
            .iter()
            .map(|stash| {
                let index = stash.index;
                container(
                    row![
                        badge('S', design::TINT_PURPLE),
                        column![
                            text(&stash.message)
                                .size(11)
                                .wrapping(iced::widget::text::Wrapping::None),
                            text(ago(stash.time)).size(10).color(design::FAINT),
                        ]
                        .width(Fill),
                        small(
                            "arrow-up",
                            "Recuperar (quita el apartado)",
                            event(Event::StashPop(index))
                        ),
                        small(
                            "plus",
                            "Aplicar (conserva el apartado)",
                            event(Event::StashApply(index))
                        ),
                        small(
                            "trash",
                            "Eliminar",
                            event(Event::Ask(Confirm::DropStash(index)))
                        ),
                    ]
                    .spacing(4)
                    .align_y(iced::Center),
                )
                .padding([3, 4])
                .clip(true)
                .into()
            })
            .collect();
        list = list.push(section(
            "archive",
            design::TINT_PURPLE,
            "Apartados (stash)",
            git.stashes.len(),
            Vec::new(),
            rows,
        ));
    }
    // The message box stays outside the scrolling area: a text editor drawn inside a
    // scrollable is not clipped correctly by the software renderer and shows garbled text.
    column![
        container(commit_box(state, staged.len(), changed.len())).padding([0, 10]),
        styles::vscroll(container(list).padding([0, 10]))
            .id(SCROLL)
            .height(Fill),
    ]
    .spacing(8)
    .height(Fill)
    .into()
}
