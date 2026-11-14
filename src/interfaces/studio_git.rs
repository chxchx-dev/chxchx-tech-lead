//! Git in the left sidebar: state and events. Commands run in `studio_git_ops`.
use super::*;
use crate::features::git::guide::Guidance;
use crate::features::git::suggest::Plan;
use crate::features::git::{
    GitBranch, GitCommit, GitEntry, GitFileChange, GitStash, GitStatus, PullMode, Repository,
};
use iced::widget::text_editor;
use std::collections::{HashMap, HashSet};

#[path = "studio_git_cmds.rs"]
mod cmds;
#[path = "studio_git_ops.rs"]
mod ops;
#[path = "studio_git_suggest.rs"]
mod plan;
pub(super) use ops::{poll, refresh};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tab {
    Changes,
    History,
    Branches,
}

/// Actions that lose work or publish it ask first, inline in the panel.
#[derive(Clone, Debug)]
pub(super) enum Confirm {
    Discard(Vec<GitEntry>),
    Push,
    PullMerge,
    DeleteBranch(String),
    DropStash(usize),
    AbortMerge,
}

#[derive(Clone, Debug)]
pub(super) enum DiffTarget {
    Staged(String),
    Unstaged(String),
    Untracked(String),
    Commit(String, String),
}

pub(super) struct Notice {
    pub ok: bool,
    /// Advice rather than an error: shown in amber.
    pub warn: bool,
    pub text: String,
}

pub(super) struct State {
    pub repo: Option<Repository>,
    /// The first read finished (successfully or not).
    pub loaded: bool,
    /// Git is missing or unusable; polling stops.
    pub unavailable: Option<String>,
    pub status: Option<GitStatus>,
    pub stashes: Vec<GitStash>,
    pub remotes: Vec<String>,
    pub merge: bool,
    pub guidance: Option<Guidance>,
    /// Proposed commits, edited by the user before saving.
    pub plan: Option<Plan>,
    pub guide_open: bool,
    pub branches: Vec<GitBranch>,
    pub log: Vec<GitCommit>,
    pub tab: Tab,
    pub commit_box: text_editor::Content,
    pub new_branch: String,
    pub selected_commit: Option<usize>,
    pub commit_detail: Option<(String, Vec<GitFileChange>)>,
    pub confirm: Option<Confirm>,
    pub busy: Option<&'static str>,
    pub notice: Option<Notice>,
    /// Project-relative path -> badge letter, for the explorer.
    pub badges: HashMap<String, char>,
    pub dirty_dirs: HashSet<String>,
    polling: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            repo: None,
            loaded: false,
            unavailable: None,
            status: None,
            stashes: Vec::new(),
            remotes: Vec::new(),
            merge: false,
            guidance: None,
            plan: None,
            guide_open: false,
            branches: Vec::new(),
            log: Vec::new(),
            tab: Tab::Changes,
            commit_box: text_editor::Content::new(),
            new_branch: String::new(),
            selected_commit: None,
            commit_detail: None,
            confirm: None,
            busy: None,
            notice: None,
            badges: HashMap::new(),
            dirty_dirs: HashSet::new(),
            polling: false,
        }
    }
}

impl State {
    /// Number shown on the rail: files with any change.
    pub fn change_count(&self) -> usize {
        self.status
            .as_ref()
            .map_or(0, |status| status.entries.len())
    }

    pub fn is_repository(&self) -> bool {
        self.repo.is_some()
    }
}

#[derive(Clone, Debug)]
pub(super) enum Event {
    Refresh,
    Poll,
    Tab(Tab),
    Edit(text_editor::Action),
    NewBranch(String),
    Init,
    Stage(Vec<String>),
    Unstage(Vec<String>),
    Ask(Confirm),
    Confirm,
    Cancel,
    Commit { all: bool },
    Fetch,
    Pull,
    Publish,
    Switch(String, bool),
    CreateBranch,
    StashPush,
    StashPop(usize),
    StashApply(usize),
    OpenDiff(DiffTarget),
    OpenFile(String),
    SelectCommit(usize),
    DismissNotice,
    ToggleGuide,
    Suggest,
    MakePlan,
    PlanText(usize, String),
    PlanType(usize),
    PlanCommit(usize),
    PlanCommitAll,
    PlanClose,
}

pub(super) fn update(state: &mut Dashboard, event: Event) -> Task<Message> {
    match event {
        Event::Refresh => return refresh(state),
        Event::Poll => return poll(state),
        Event::Tab(tab) => {
            state.git.tab = tab;
            if tab != Tab::Changes {
                return refresh(state);
            }
        }
        Event::Edit(action) => state.git.commit_box.perform(action),
        Event::NewBranch(name) => state.git.new_branch = name,
        Event::Ask(confirm) => state.git.confirm = Some(confirm),
        Event::Cancel => state.git.confirm = None,
        Event::DismissNotice => state.git.notice = None,
        Event::ToggleGuide => state.git.guide_open = !state.git.guide_open,
        Event::Suggest => return plan::suggest_message(state),
        Event::MakePlan => return plan::make_plan(state),
        Event::PlanText(index, text) => plan::edit_text(state, index, text),
        Event::PlanType(index) => plan::cycle_type(state, index),
        Event::PlanCommit(index) => return plan::commit_group(state, index),
        Event::PlanCommitAll => return plan::commit_all(state),
        Event::PlanClose => state.git.plan = None,
        Event::Init => return cmds::init(state),
        Event::Stage(paths) => return cmds::stage(state, paths),
        Event::Unstage(paths) => return cmds::unstage(state, paths),
        Event::Commit { all } => return cmds::commit(state, all),
        Event::Fetch => return cmds::fetch(state),
        Event::Pull => return cmds::pull(state, PullMode::FastForwardOnly),
        Event::Publish => return cmds::push(state, true),
        Event::Switch(name, remote) => return cmds::switch(state, name, remote),
        Event::CreateBranch => return cmds::create_branch(state),
        Event::StashPush => return cmds::stash_push(state),
        Event::StashPop(index) => return cmds::stash_pop(state, index),
        Event::StashApply(index) => return cmds::stash_apply(state, index),
        Event::Confirm => return cmds::confirmed(state),
        Event::OpenDiff(target) => return cmds::open_diff(state, target),
        Event::OpenFile(path) => {
            let project = state
                .git
                .repo
                .as_ref()
                .and_then(|repo| repo.project_path(&path));
            return match project {
                Some(relative) => Task::done(Message::Editor(editor::Event::Open(PathBuf::from(
                    relative,
                )))),
                None => {
                    state.git.notice = Some(Notice {
                        ok: false,
                        warn: false,
                        text: "Ese archivo está fuera de la carpeta del proyecto.".into(),
                    });
                    Task::none()
                }
            };
        }
        Event::SelectCommit(index) => return cmds::select_commit(state, index),
    }
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::git::parse::parse_status;

    fn with_status(text: &str) -> Dashboard {
        let mut state = editor::fixture();
        state.git.status = Some(parse_status(text));
        state
    }

    #[test]
    fn the_rail_counts_every_changed_file() {
        let state = with_status(
            "# branch.oid abc\0# branch.head main\x001 M. N... 1 1 1 a b x.rs\0? y.md\0",
        );
        assert_eq!(state.git.change_count(), 2);
        assert!(
            !state.git.is_repository(),
            "no repository handle until loaded"
        );
    }

    #[test]
    fn confirmations_open_and_close_without_running_anything() {
        let mut state = editor::fixture();
        let _ = update(&mut state, Event::Ask(Confirm::Push));
        assert!(matches!(state.git.confirm, Some(Confirm::Push)));
        let _ = update(&mut state, Event::Cancel);
        assert!(state.git.confirm.is_none());
        // Confirming with nothing pending is a no-op, never a command.
        let _ = update(&mut state, Event::Confirm);
        assert!(state.git.busy.is_none());
    }

    #[test]
    fn commands_without_a_repository_do_nothing() {
        let mut state = editor::fixture();
        for event in [
            Event::Stage(vec!["a.rs".into()]),
            Event::Fetch,
            Event::Pull,
            Event::StashPush,
            Event::SelectCommit(0),
        ] {
            let _ = update(&mut state, event);
            assert!(state.git.busy.is_none() && state.git.notice.is_none());
        }
        let _ = update(&mut state, Event::Commit { all: false });
        assert!(
            state.git.notice.as_ref().is_some_and(|notice| !notice.ok),
            "an empty message is explained, not sent to git"
        );
    }

    #[test]
    fn explorer_badges_follow_the_status_and_ignore_other_folders() {
        use crate::features::git::Repository;
        let mut state = with_status(
            "# branch.oid abc\0# branch.head main\x001 .M N... 1 1 1 a b app/src/main.rs\x001 M. N... 1 1 1 a b other/x.rs\0",
        );
        state.git.repo = Some(Repository {
            root: PathBuf::from("."),
            prefix: "app/".to_owned(),
            trusted: true,
        });
        let repo = state.git.repo.as_ref().unwrap();
        assert_eq!(
            repo.project_path("app/src/main.rs").as_deref(),
            Some("src/main.rs")
        );
        assert_eq!(repo.project_path("other/x.rs"), None);
    }
}
