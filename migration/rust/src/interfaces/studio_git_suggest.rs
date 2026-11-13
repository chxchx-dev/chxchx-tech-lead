//! Suggested commit message and the commit plan. Analysis runs locally in the background.
use super::ops::{advise, refresh, say};
use super::*;
use crate::features::git::suggest::{CommitGroup, Plan, plan};
use crate::features::git::{GitEntry, GitFailure, Repository};

type Analysis = Result<(Repository, Vec<GitEntry>, bool), String>;

/// Everything pending that can be described: prepared and edited files, not conflicts.
fn pending(state: &Dashboard, only_prepared: bool) -> Analysis {
    let (Some(repo), Some(status)) = (state.git.repo.clone(), state.git.status.as_ref()) else {
        return Err("Git todavía no está listo.".to_owned());
    };
    if state.git.busy.is_some() {
        return Err("Espera a que termine la acción en curso.".to_owned());
    }
    let mut entries: Vec<GitEntry> = Vec::new();
    for entry in status
        .staged_entries()
        .into_iter()
        .chain(status.changed_entries())
    {
        if !entries.iter().any(|known| known.path == entry.path) {
            entries.push(entry.clone());
        }
    }
    if only_prepared {
        let prepared: Vec<GitEntry> = status.staged_entries().into_iter().cloned().collect();
        if !prepared.is_empty() {
            entries = prepared;
        }
    }
    if entries.is_empty() {
        return Err("No hay cambios que describir.".to_owned());
    }
    Ok((repo, entries, status.initial))
}

/// Proposes what to save. When the pending files belong to several topics it distributes
/// them into related commits (the plan) instead of one mixed message; with a single topic
/// it fills the commit box with a Conventional Commits message.
pub(in crate::interfaces::studio) fn suggest_message(state: &mut Dashboard) -> Task<Message> {
    let (repo, entries, initial) = match pending(state, true) {
        Ok(found) => found,
        Err(text) => {
            say(state, false, text);
            return Task::none();
        }
    };
    state.git.busy = Some("Pensando cómo repartir los cambios…");
    jobs::run(
        state,
        move || -> Result<Plan, GitFailure> { Ok(plan(&repo.analyse(&entries, initial)?)) },
        |state, result| {
            state.git.busy = None;
            match result {
                Ok(plan) if plan.groups.len() > 1 => {
                    let count = plan.groups.len();
                    state.git.plan = Some(plan);
                    advise(
                        state,
                        format!(
                            "Hay varios temas mezclados: los repartí en {count} commits relacionados. Revisa cada uno, cambia el tipo o la descripción y guárdalos."
                        ),
                    );
                }
                Ok(mut plan) => match plan.groups.pop() {
                    Some(group) => {
                        state.git.commit_box =
                            iced::widget::text_editor::Content::with_text(&group.message());
                        let mut text =
                            "Mensaje sugerido. Revísalo y edítalo antes de guardar.".to_owned();
                        for warning in plan.warnings {
                            text.push_str(&format!(" {warning}"));
                        }
                        say(state, true, text);
                    }
                    None => say(state, false, "No hay cambios que describir."),
                },
                Err(failure) => say(state, false, failure.message),
            }
            Task::none()
        },
    )
}

/// Groups every pending file into commits.
pub(in crate::interfaces::studio) fn make_plan(state: &mut Dashboard) -> Task<Message> {
    let (repo, entries, initial) = match pending(state, false) {
        Ok(found) => found,
        Err(text) => {
            say(state, false, text);
            return Task::none();
        }
    };
    state.git.busy = Some("Organizando los cambios…");
    jobs::run(
        state,
        move || -> Result<Plan, GitFailure> { Ok(plan(&repo.analyse(&entries, initial)?)) },
        |state, result| {
            state.git.busy = None;
            match result {
                Ok(plan) if plan.groups.is_empty() => {
                    say(state, false, "No hay cambios que organizar.")
                }
                Ok(plan) => {
                    state.git.notice = None;
                    state.git.plan = Some(plan);
                }
                Err(failure) => say(state, false, failure.message),
            }
            Task::none()
        },
    )
}

pub(in crate::interfaces::studio) fn edit_text(state: &mut Dashboard, index: usize, text: String) {
    if let Some(group) = state
        .git
        .plan
        .as_mut()
        .and_then(|plan| plan.groups.get_mut(index))
    {
        group.text = text;
    }
}

pub(in crate::interfaces::studio) fn cycle_type(state: &mut Dashboard, index: usize) {
    if let Some(group) = state
        .git
        .plan
        .as_mut()
        .and_then(|plan| plan.groups.get_mut(index))
    {
        group.kind = group.kind.next();
    }
}

/// Saves the first `count` groups in order, stopping at the first failure.
fn save(state: &mut Dashboard, groups: Vec<CommitGroup>, label: &'static str) -> Task<Message> {
    let Some(repo) = state.git.repo.clone() else {
        return Task::none();
    };
    if state.git.busy.is_some() {
        return Task::none();
    }
    if state.git.merge {
        say(
            state,
            false,
            "Termina o cancela la fusión antes de guardar por partes.",
        );
        return Task::none();
    }
    let initial = state
        .git
        .status
        .as_ref()
        .is_some_and(|status| status.initial);
    state.git.busy = Some(label);
    jobs::run(
        state,
        move || {
            let mut saved = 0;
            for group in &groups {
                if let Err(failure) = repo.commit_paths(&group.message(), &group.files, initial) {
                    return (saved, Some((group.subject(), failure)));
                }
                saved += 1;
            }
            (saved, None)
        },
        |state, (saved, failed)| {
            state.git.busy = None;
            // Saved groups leave the plan; the rest stays so the user can fix and retry.
            if failed.is_none()
                && state
                    .git
                    .plan
                    .as_ref()
                    .is_some_and(|plan| saved >= plan.groups.len())
            {
                state.git.plan = None;
            }
            match failed {
                None => say(state, true, format!("{saved} commit(s) guardados.")),
                Some((subject, failure)) => say(
                    state,
                    false,
                    format!(
                        "Se guardaron {saved}. Falló «{subject}»: {}",
                        failure.message
                    ),
                ),
            }
            refresh(state)
        },
    )
}

pub(in crate::interfaces::studio) fn commit_group(
    state: &mut Dashboard,
    index: usize,
) -> Task<Message> {
    let Some(group) = state
        .git
        .plan
        .as_ref()
        .and_then(|plan| plan.groups.get(index))
        .cloned()
    else {
        return Task::none();
    };
    save(state, vec![group], "Guardando el commit…")
}

pub(in crate::interfaces::studio) fn commit_all(state: &mut Dashboard) -> Task<Message> {
    let Some(groups) = state.git.plan.as_ref().map(|plan| plan.groups.clone()) else {
        return Task::none();
    };
    save(state, groups, "Guardando los commits en orden…")
}

/// Drops from the plan what is no longer pending (committed, discarded or reverted).
pub(super) fn reconcile(state: &mut Dashboard) {
    let (Some(plan), Some(status)) = (state.git.plan.as_mut(), state.git.status.as_ref()) else {
        return;
    };
    let pending: std::collections::HashSet<&str> = status
        .entries
        .iter()
        .flat_map(|entry| [Some(entry.path.as_str()), entry.orig_path.as_deref()])
        .flatten()
        .collect();
    for group in &mut plan.groups {
        group.files.retain(|path| pending.contains(path.as_str()));
    }
    plan.groups.retain(|group| !group.files.is_empty());
    if plan.groups.is_empty() {
        state.git.plan = None;
    }
}
