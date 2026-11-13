//! Git commands for the sidebar. Every command runs in the background; the panel
//! refreshes itself afterwards.
use super::*;
use crate::features::git::guide::{GuideFacts, guidance_for};
use crate::features::git::{
    GitBranch, GitCommit, GitFailure, GitStash, GitStatus, Problem, Repository,
};

struct Loaded {
    repo: Repository,
    status: GitStatus,
    stashes: Vec<GitStash>,
    remotes: Vec<String>,
    merge: bool,
    history: Option<(Vec<GitBranch>, Vec<GitCommit>)>,
}

fn load(
    project: PathBuf,
    trusted: bool,
    known: Option<Repository>,
    history: bool,
) -> Result<Option<Loaded>, GitFailure> {
    let repo = match known {
        Some(mut repo) => {
            repo.trusted = trusted;
            repo
        }
        None => match Repository::open(&project, trusted)? {
            Some(repo) => repo,
            None => return Ok(None),
        },
    };
    let status = repo.status()?;
    let history = if history {
        Some((repo.branches()?, repo.log(150)?))
    } else {
        None
    };
    Ok(Some(Loaded {
        stashes: repo.stashes().unwrap_or_default(),
        remotes: repo.remotes().unwrap_or_default(),
        merge: repo.merge_in_progress(),
        status,
        history,
        repo,
    }))
}

pub(super) fn trusted(state: &Dashboard) -> bool {
    state
        .overview
        .as_ref()
        .is_ok_and(|overview| overview.workspace.trusted)
}

fn apply(state: &mut Dashboard, result: Result<Option<Loaded>, GitFailure>) {
    let git = &mut state.git;
    git.loaded = true;
    match result {
        Err(failure) => {
            if failure.message.contains("No se encontró Git") {
                git.unavailable = Some(failure.message);
            } else {
                git.notice = Some(Notice {
                    ok: false,
                    warn: false,
                    text: failure.message,
                });
            }
        }
        Ok(None) => {
            git.repo = None;
            git.status = None;
            git.guidance = None;
            git.badges.clear();
            git.dirty_dirs.clear();
        }
        Ok(Some(loaded)) => {
            git.unavailable = None;
            let facts = GuideFacts {
                merge_in_progress: loaded.merge,
                has_remotes: !loaded.remotes.is_empty(),
                stash_count: loaded.stashes.len(),
            };
            git.guidance = Some(guidance_for(&loaded.status, &facts));
            git.badges.clear();
            git.dirty_dirs.clear();
            for entry in &loaded.status.entries {
                let Some(path) = loaded.repo.project_path(&entry.path) else {
                    continue;
                };
                let mut folder = path.as_str();
                while let Some((parent, _)) = folder.rsplit_once('/') {
                    git.dirty_dirs.insert(parent.to_owned());
                    folder = parent;
                }
                git.badges.insert(path, entry.badge());
            }
            if let Some((branches, log)) = loaded.history {
                // Keep the open commit when it is still in the history.
                let open = git
                    .selected_commit
                    .and_then(|index| git.log.get(index))
                    .map(|commit| commit.hash.clone());
                git.selected_commit = open
                    .as_ref()
                    .and_then(|hash| log.iter().position(|commit| commit.hash == *hash));
                if git.selected_commit.is_none() {
                    git.commit_detail = None;
                }
                git.branches = branches;
                git.log = log;
            }
            git.stashes = loaded.stashes;
            git.remotes = loaded.remotes;
            git.merge = loaded.merge;
            git.status = Some(loaded.status);
            git.repo = Some(loaded.repo);
            super::plan::reconcile(state);
        }
    }
}

fn request(state: &Dashboard, history: bool) -> (PathBuf, bool, Option<Repository>, bool) {
    (
        state.root.clone(),
        trusted(state),
        state.git.repo.clone(),
        history,
    )
}

/// Reads the repository again, showing the working indicator.
pub(in crate::interfaces::studio) fn refresh(state: &mut Dashboard) -> Task<Message> {
    if state.git.unavailable.is_some() {
        return Task::none();
    }
    let history = state.git.tab != git::Tab::Changes;
    let (project, trusted, known, history) = request(state, history);
    jobs::run(
        state,
        move || load(project, trusted, known, history),
        |state, result| {
            apply(state, result);
            Task::none()
        },
    )
}

/// Background refresh for the timer and after saving a file: no indicator, no overlap.
pub(in crate::interfaces::studio) fn poll(state: &mut Dashboard) -> Task<Message> {
    if state.git.unavailable.is_some() || state.git.polling || state.git.busy.is_some() {
        return Task::none();
    }
    state.git.polling = true;
    // Only the status is read; the history reloads when the repository moved.
    let before = state.git.status.as_ref().map(|status| status.oid.clone());
    let (project, trusted, known, history) = request(state, false);
    jobs::quiet(
        move || load(project, trusted, known, history),
        move |state, result| {
            state.git.polling = false;
            if state.git.busy.is_some() {
                return Task::none();
            }
            apply(state, result);
            let moved = state.git.status.as_ref().map(|status| status.oid.clone()) != before;
            if moved && state.git.tab != git::Tab::Changes {
                return refresh(state);
            }
            Task::none()
        },
    )
}

pub(super) fn say(state: &mut Dashboard, ok: bool, text: impl Into<String>) {
    state.git.notice = Some(Notice {
        ok,
        warn: false,
        text: text.into(),
    });
}

/// An advisory notice (amber): worth reading, not an error.
pub(super) fn advise(state: &mut Dashboard, text: impl Into<String>) {
    state.git.notice = Some(Notice {
        ok: true,
        warn: true,
        text: text.into(),
    });
}

/// Runs one change on the repository, reports the outcome and refreshes.
pub(super) fn act(
    state: &mut Dashboard,
    label: &'static str,
    job: impl FnOnce(Repository) -> Result<String, GitFailure> + Send + 'static,
) -> Task<Message> {
    let Some(repo) = state.git.repo.clone() else {
        return Task::none();
    };
    if state.git.busy.is_some() {
        return Task::none();
    }
    state.git.busy = Some(label);
    state.git.confirm = None;
    jobs::run(
        state,
        move || job(repo),
        |state, result| {
            state.git.busy = None;
            match result {
                Ok(text) => say(state, true, text),
                Err(failure) => {
                    if failure.problem == Problem::Diverged {
                        state.git.confirm = Some(git::Confirm::PullMerge);
                    }
                    say(state, false, failure.message);
                }
            }
            refresh(state)
        },
    )
}
