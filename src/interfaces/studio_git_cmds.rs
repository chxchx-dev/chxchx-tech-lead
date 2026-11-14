//! Git commands started from the sidebar: each runs in the background and refreshes.
use super::ops::{act, refresh, say, trusted};
use super::*;
use crate::features::git::{GitFailure, PullMode, Repository};

type DiffJob = Box<dyn FnOnce(&Repository) -> Result<String, GitFailure> + Send>;

pub(in crate::interfaces::studio) fn init(state: &mut Dashboard) -> Task<Message> {
    let (project, trusted) = (state.root.clone(), trusted(state));
    state.git.busy = Some("Creando el repositorio…");
    jobs::run(
        state,
        move || Repository::init(&project, trusted),
        |state, result| {
            state.git.busy = None;
            match result {
                Ok(_) => say(state, true, "Repositorio Git creado."),
                Err(failure) => say(state, false, failure.message),
            }
            refresh(state)
        },
    )
}

pub(in crate::interfaces::studio) fn stage(
    state: &mut Dashboard,
    paths: Vec<String>,
) -> Task<Message> {
    act(state, "Preparando…", move |repo| {
        repo.stage(&paths)
            .map(|()| "Cambios preparados para guardar.".to_owned())
    })
}

pub(in crate::interfaces::studio) fn unstage(
    state: &mut Dashboard,
    paths: Vec<String>,
) -> Task<Message> {
    let initial = state
        .git
        .status
        .as_ref()
        .is_some_and(|status| status.initial);
    act(state, "Quitando de la lista…", move |repo| {
        repo.unstage(&paths, initial)
            .map(|()| "Quitado de «Listos para guardar».".to_owned())
    })
}

pub(in crate::interfaces::studio) fn commit(state: &mut Dashboard, all: bool) -> Task<Message> {
    // Nothing prepared (for example with Ctrl+Enter): save every change.
    let all = all
        || state
            .git
            .status
            .as_ref()
            .is_some_and(|status| status.staged_entries().is_empty());
    let message = state.git.commit_box.text();
    if message.trim().is_empty() {
        say(state, false, "Escribe un mensaje para describir el cambio.");
        return Task::none();
    }
    let Some(repo) = state.git.repo.clone() else {
        return Task::none();
    };
    if state.git.busy.is_some() {
        return Task::none();
    }
    state.git.busy = Some("Guardando…");
    jobs::run(
        state,
        move || repo.commit(&message, all),
        |state, result| {
            state.git.busy = None;
            match result {
                Ok(text) => {
                    state.git.commit_box = iced::widget::text_editor::Content::new();
                    say(state, true, format!("Guardado: {text}"));
                }
                Err(failure) => say(state, false, failure.message),
            }
            refresh(state)
        },
    )
}

pub(in crate::interfaces::studio) fn fetch(state: &mut Dashboard) -> Task<Message> {
    act(state, "Buscando novedades…", |repo| repo.fetch())
}

pub(in crate::interfaces::studio) fn pull(state: &mut Dashboard, mode: PullMode) -> Task<Message> {
    act(state, "Recibiendo…", move |repo| repo.pull(mode))
}

/// `publish` creates the branch on the remote the first time.
pub(in crate::interfaces::studio) fn push(state: &mut Dashboard, publish: bool) -> Task<Message> {
    if !state.git.busy.is_none() {
        return Task::none();
    }
    let target = state.git.status.as_ref().map(|status| status.head.clone());
    let remote = state
        .git
        .remotes
        .iter()
        .find(|name| *name == "origin")
        .or_else(|| state.git.remotes.first())
        .cloned();
    act(state, "Enviando…", move |repo| {
        match (publish, remote, target) {
            (true, Some(remote), Some(branch)) => repo.push(Some((&remote, &branch))),
            (true, _, _) => Err(GitFailure::other(
                "Este repositorio no tiene un remoto configurado.",
            )),
            _ => repo.push(None),
        }
    })
}

pub(in crate::interfaces::studio) fn switch(
    state: &mut Dashboard,
    name: String,
    remote: bool,
) -> Task<Message> {
    act(state, "Cambiando de rama…", move |repo| {
        repo.switch_branch(&name, remote)
    })
}

pub(in crate::interfaces::studio) fn create_branch(state: &mut Dashboard) -> Task<Message> {
    let name = std::mem::take(&mut state.git.new_branch);
    act(state, "Creando la rama…", move |repo| {
        repo.create_branch(name.trim(), None)
    })
}

pub(in crate::interfaces::studio) fn stash_push(state: &mut Dashboard) -> Task<Message> {
    act(state, "Apartando cambios…", |repo| {
        repo.stash_push("", true).map(|text| {
            if text.is_empty() {
                "Cambios apartados (stash).".to_owned()
            } else {
                text
            }
        })
    })
}

pub(in crate::interfaces::studio) fn stash_pop(
    state: &mut Dashboard,
    index: usize,
) -> Task<Message> {
    act(state, "Recuperando…", move |repo| repo.stash_pop(index))
}

pub(in crate::interfaces::studio) fn stash_apply(
    state: &mut Dashboard,
    index: usize,
) -> Task<Message> {
    act(state, "Aplicando…", move |repo| repo.stash_apply(index))
}

/// Runs the action the user just confirmed.
pub(in crate::interfaces::studio) fn confirmed(state: &mut Dashboard) -> Task<Message> {
    let Some(confirm) = state.git.confirm.take() else {
        return Task::none();
    };
    let initial = state
        .git
        .status
        .as_ref()
        .is_some_and(|status| status.initial);
    match confirm {
        git::Confirm::Discard(entries) => act(state, "Descartando…", move |repo| {
            repo.discard(&entries, initial).map(|backup| match backup {
                Some(path) => format!(
                    "Cambios descartados. Copia de seguridad en {}",
                    path.display()
                ),
                None => "Cambios descartados.".to_owned(),
            })
        }),
        git::Confirm::Push => push(state, false),
        git::Confirm::PullMerge => pull(state, PullMode::Merge),
        git::Confirm::DeleteBranch(name) => act(state, "Eliminando la rama…", move |repo| {
            // An unmerged branch needs an explicit decision; never force it silently.
            repo.delete_branch(&name, false).map_err(|failure| {
                GitFailure::other(format!(
                    "{} Si estás seguro, elimínala desde la terminal con git branch -D.",
                    failure.message
                ))
            })
        }),
        git::Confirm::DropStash(index) => act(state, "Eliminando el apartado…", move |repo| {
            repo.stash_drop(index)
        }),
        git::Confirm::AbortMerge => {
            act(state, "Cancelando la fusión…", |repo| repo.abort_merge())
        }
    }
}

pub(in crate::interfaces::studio) fn open_diff(
    state: &mut Dashboard,
    target: git::DiffTarget,
) -> Task<Message> {
    let Some(repo) = state.git.repo.clone() else {
        return Task::none();
    };
    let (label, job): (String, DiffJob) = match target {
        git::DiffTarget::Staged(path) => (
            format!("staged/{path}"),
            Box::new(move |repo| repo.file_diff(&path, true)),
        ),
        git::DiffTarget::Unstaged(path) => (
            path.clone(),
            Box::new(move |repo| repo.file_diff(&path, false)),
        ),
        git::DiffTarget::Untracked(path) => (
            path.clone(),
            Box::new(move |repo| repo.untracked_diff(&path)),
        ),
        git::DiffTarget::Commit(hash, path) => (
            format!("{}/{path}", &hash[..hash.len().min(7)]),
            Box::new(move |repo| repo.commit_diff(&hash, &path)),
        ),
    };
    jobs::run(
        state,
        move || job(&repo),
        move |state, result| match result {
            Ok(text) => Task::done(Message::Editor(editor::Event::OpenText(
                PathBuf::from(format!("diff/{label}.diff")),
                text,
            ))),
            Err(failure) => {
                say(state, false, failure.message);
                Task::none()
            }
        },
    )
}

pub(in crate::interfaces::studio) fn select_commit(
    state: &mut Dashboard,
    index: usize,
) -> Task<Message> {
    let (Some(repo), Some(commit)) = (state.git.repo.clone(), state.git.log.get(index)) else {
        return Task::none();
    };
    if state.git.selected_commit == Some(index) {
        state.git.selected_commit = None;
        state.git.commit_detail = None;
        return Task::none();
    }
    state.git.selected_commit = Some(index);
    state.git.commit_detail = None;
    let hash = commit.hash.clone();
    jobs::run(
        state,
        move || {
            let message = repo.commit_message(&hash)?;
            let files = repo.commit_files(&hash)?;
            Ok::<_, GitFailure>((message, files))
        },
        move |state, result| {
            // Ignore the answer when the user already moved to another commit.
            if state.git.selected_commit == Some(index) {
                match result {
                    Ok(detail) => state.git.commit_detail = Some(detail),
                    Err(failure) => say(state, false, failure.message),
                }
            }
            Task::none()
        },
    )
}
