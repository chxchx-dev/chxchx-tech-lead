//! Catalog page: skills, Tech Packs and Mods for the open project.
#[path = "studio_catalog_rows.rs"]
mod rows;
#[path = "studio_catalog_view.rs"]
mod ui;
pub(super) use ui::{header, view};

use super::*;
use crate::features::catalog_view::{CatalogView, inspect_catalog};
use crate::features::project_mods::{ModsChange, ModsPlan, apply_mods_plan, plan_mods};
use crate::features::project_skills::{SkillsChange, SkillsPlan, apply_skills, plan_skills};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tab {
    Skills,
    Packs,
    Mods,
}

pub(super) struct State {
    pub tab: Tab,
    pub view: Option<CatalogView>,
    pub loading: bool,
    pub busy: bool,
    pub filter: String,
    pub skills_plan: Option<SkillsPlan>,
    pub mods_plan: Option<ModsPlan>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tab: Tab::Skills,
            view: None,
            loading: false,
            busy: false,
            filter: String::new(),
            skills_plan: None,
            mods_plan: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Load,
    Select(Tab),
    Filter(String),
    Skills(SkillsChange),
    Mods(ModsChange),
    ApplySkills,
    ApplyMods,
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    let root = state.root.clone();
    match event {
        Event::Select(tab) => state.catalog.tab = tab,
        Event::Filter(filter) => state.catalog.filter = filter,
        Event::Load => {
            if state.catalog.loading {
                return Task::none();
            }
            state.catalog.loading = true;
            return jobs::run(
                state,
                move || inspect_catalog(&root),
                |state, result| {
                    state.catalog.loading = false;
                    match result {
                        Ok(view) => state.catalog.view = Some(view),
                        Err(error) => state.message = error,
                    }
                    Task::none()
                },
            );
        }
        Event::Skills(change) => {
            return jobs::run(
                state,
                move || plan_skills(&root, &change),
                |state, result| {
                    match result {
                        Ok(plan) => {
                            plans::clear_previews(state);
                            state.message =
                                "Revisa los cambios de skills antes de aplicarlos.".into();
                            state.catalog.skills_plan = Some(plan);
                        }
                        Err(error) => state.message = error,
                    }
                    Task::none()
                },
            );
        }
        Event::Mods(change) => {
            return jobs::run(
                state,
                move || plan_mods(&root, &change),
                |state, result| {
                    match result {
                        Ok(plan) => {
                            plans::clear_previews(state);
                            state.message = "Revisa los Mods antes de aplicarlos.".into();
                            state.catalog.mods_plan = Some(plan);
                        }
                        Err(error) => state.message = error,
                    }
                    Task::none()
                },
            );
        }
        Event::ApplySkills => {
            let Some(plan) = state
                .catalog
                .skills_plan
                .clone()
                .filter(|_| !state.catalog.busy)
            else {
                return Task::none();
            };
            state.catalog.busy = true;
            return jobs::run(
                state,
                move || apply_skills(&plan),
                |state, result| {
                    state.catalog.busy = false;
                    state.catalog.skills_plan = None;
                    match result {
                        Ok(done) => {
                            state.message = format!("Skills habilitadas: {}.", done.enabled.len());
                            if !done.backups.is_empty() {
                                state
                                    .message
                                    .push_str(&format!(" · Backup: {}", done.backups.join(", ")));
                            }
                        }
                        Err(error) => state.message = error,
                    }
                    update(state, Event::Load)
                },
            );
        }
        Event::ApplyMods => {
            let Some(plan) = state
                .catalog
                .mods_plan
                .clone()
                .filter(|_| !state.catalog.busy)
            else {
                return Task::none();
            };
            state.catalog.busy = true;
            return jobs::run(
                state,
                move || apply_mods_plan(&plan),
                |state, result| {
                    state.catalog.busy = false;
                    state.catalog.mods_plan = None;
                    match result {
                        Ok(done) => {
                            state.message = done.completed.join(" · ");
                            if !done.backups.is_empty() {
                                state
                                    .message
                                    .push_str(&format!(" · Backups: {}", done.backups.join(", ")));
                            }
                        }
                        Err(error) => state.message = error,
                    }
                    update(state, Event::Load)
                },
            );
        }
    }
    Task::none()
}
