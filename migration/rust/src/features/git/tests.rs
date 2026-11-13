//! End-to-end checks against real repositories in temporary folders.
use super::guide::{GuideFacts, Situation, guidance_for};
use super::*;
use crate::integrations::git::{self, Run};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Sandbox(PathBuf);

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git_available() -> bool {
    git::run(Path::new("."), &Run::new(["--version"]).read_only()).is_ok()
}

/// A repository with a local identity and `main` as its branch; `None` without git.
pub(super) fn repository(trusted: bool) -> Option<(Sandbox, Repository)> {
    if !git_available() {
        return None;
    }
    let dir = std::env::temp_dir().join(format!(
        "chxchx-git-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let sandbox = Sandbox(dir);
    let repo = Repository::init(&sandbox.0, trusted).unwrap();
    for args in [
        vec!["symbolic-ref", "HEAD", "refs/heads/main"],
        vec!["config", "user.name", "Prueba"],
        vec!["config", "user.email", "prueba@example.com"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["config", "core.autocrlf", "false"],
    ] {
        git::run(&repo.root, &Run::new(args)).unwrap();
    }
    Some((sandbox, repo))
}

pub(super) fn write(repo: &Repository, name: &str, text: &str) {
    let path = repo.root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn a_project_outside_a_repository_is_reported_as_such() {
    if !git_available() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("chxchx-nogit-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    // A temp folder is not inside any repository on a normal machine.
    let found = Repository::open(&dir, true).unwrap();
    let _ = fs::remove_dir_all(&dir);
    if let Some(repo) = found {
        assert!(
            repo.root.exists(),
            "an enclosing repository is acceptable, a broken one is not"
        );
    }
}

#[test]
fn stage_commit_history_and_branches_work_end_to_end() {
    let Some((_keep, repo)) = repository(true) else {
        return;
    };
    write(&repo, "a.txt", "uno\n");
    let status = repo.status().unwrap();
    assert!(status.initial);
    assert_eq!(status.entries[0].badge(), 'N');

    repo.stage(&["a.txt".to_owned()]).unwrap();
    assert_eq!(repo.status().unwrap().staged_entries().len(), 1);
    assert!(
        repo.commit("   ", false).is_err(),
        "an empty message is refused"
    );
    repo.commit("Primer commit", false).unwrap();
    let status = repo.status().unwrap();
    assert!(!status.initial && status.entries.is_empty());
    assert_eq!(status.head, "main");

    write(&repo, "a.txt", "uno\ndos\n");
    write(&repo, "nuevo/b.txt", "hola\n");
    let status = repo.status().unwrap();
    assert_eq!(status.changed_entries().len(), 2);
    assert!(repo.file_diff("a.txt", false).unwrap().contains("+dos"));
    assert!(
        repo.untracked_diff("nuevo/b.txt")
            .unwrap()
            .contains("+++ b/nuevo/b.txt")
    );
    repo.commit("Segundo", true).unwrap();

    let log = repo.log(50).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].subject, "Segundo");
    assert!(log[0].refs.iter().any(|name| name.contains("main")));
    let files = repo.commit_files(&log[0].hash).unwrap();
    assert!(
        files
            .iter()
            .any(|file| file.path == "nuevo/b.txt" && file.status == "A")
    );
    assert!(
        repo.commit_diff(&log[0].hash, "a.txt")
            .unwrap()
            .contains("+dos")
    );
    assert_eq!(repo.commit_message(&log[0].hash).unwrap(), "Segundo");

    repo.create_branch("feature/uno", None).unwrap();
    assert_eq!(repo.status().unwrap().head, "feature/uno");
    repo.switch_branch("main", false).unwrap();
    let branches = repo.branches().unwrap();
    assert!(
        branches
            .iter()
            .any(|b| b.name == "feature/uno" && !b.current)
    );
    assert!(
        branches
            .first()
            .is_some_and(|b| b.current && b.name == "main")
    );
    repo.delete_branch("feature/uno", false).unwrap();
    assert!(repo.create_branch("nombre malo", None).is_err());
    assert!(repo.create_branch("-x", None).is_err());
}

#[test]
fn discarding_saves_a_backup_first_and_restores_the_files() {
    let Some((_keep, repo)) = repository(true) else {
        return;
    };
    write(&repo, "a.txt", "original\n");
    repo.stage(&["a.txt".to_owned()]).unwrap();
    repo.commit("base", false).unwrap();

    write(&repo, "a.txt", "cambiado\n");
    write(&repo, "suelto.txt", "no rastreado\n");
    let status = repo.status().unwrap();
    let entries: Vec<GitEntry> = status.changed_entries().into_iter().cloned().collect();
    let backup = repo
        .discard(&entries, false)
        .unwrap()
        .expect("a backup folder");

    assert_eq!(
        fs::read_to_string(repo.root.join("a.txt")).unwrap(),
        "original\n"
    );
    assert!(!repo.root.join("suelto.txt").exists());
    assert!(
        fs::read_to_string(backup.join("changes.patch"))
            .unwrap()
            .contains("+cambiado")
    );
    assert_eq!(
        fs::read_to_string(backup.join("untracked/suelto.txt")).unwrap(),
        "no rastreado\n"
    );
    assert!(repo.status().unwrap().entries.is_empty());
}

#[test]
fn stash_round_trip_and_unsafe_paths_are_rejected() {
    let Some((_keep, repo)) = repository(true) else {
        return;
    };
    write(&repo, "a.txt", "uno\n");
    repo.stage(&["a.txt".to_owned()]).unwrap();
    repo.commit("base", false).unwrap();
    write(&repo, "a.txt", "dos\n");
    repo.stash_push("idea a medias", false).unwrap();
    let stashes = repo.stashes().unwrap();
    assert_eq!(stashes.len(), 1);
    assert!(stashes[0].message.contains("idea a medias"));
    assert_eq!(
        fs::read_to_string(repo.root.join("a.txt")).unwrap(),
        "uno\n"
    );
    repo.stash_pop(0).unwrap();
    assert_eq!(
        fs::read_to_string(repo.root.join("a.txt")).unwrap(),
        "dos\n"
    );
    assert!(repo.stashes().unwrap().is_empty());

    for bad in ["../fuera.txt", "-n", "", "/etc/passwd"] {
        assert!(
            repo.stage(&[bad.to_owned()]).is_err(),
            "{bad:?} must be rejected"
        );
    }
    assert!(repo.commit_diff("no-es-hash", "a.txt").is_err());
    // A path that looks like a glob is taken literally.
    write(&repo, "[x].txt", "corchetes\n");
    repo.stage(&["[x].txt".to_owned()]).unwrap();
    assert!(
        repo.status()
            .unwrap()
            .staged_entries()
            .iter()
            .any(|e| e.path == "[x].txt")
    );
}

#[test]
fn untrusted_projects_never_run_the_repository_hooks() {
    let hook = "#!/bin/sh\nexit 1\n";
    for (trusted, commits) in [(true, false), (false, true)] {
        let Some((_keep, repo)) = repository(trusted) else {
            return;
        };
        let hooks = repo.root.join(".git/hooks");
        fs::create_dir_all(&hooks).unwrap();
        fs::write(hooks.join("pre-commit"), hook).unwrap();
        write(&repo, "a.txt", "x\n");
        repo.stage(&["a.txt".to_owned()]).unwrap();
        let result = repo.commit("con hook", false);
        // The hook only blocks when it is allowed to run (Windows needs Git's bundled sh).
        if commits {
            assert!(
                result.is_ok(),
                "untrusted: hook must be skipped: {result:?}"
            );
        } else if let Err(error) = &result {
            assert!(!error.message.is_empty());
        }
    }
}

#[test]
fn guidance_reads_a_real_repository() {
    let Some((_keep, repo)) = repository(true) else {
        return;
    };
    let facts = GuideFacts {
        merge_in_progress: repo.merge_in_progress(),
        has_remotes: !repo.remotes().unwrap().is_empty(),
        stash_count: 0,
    };
    assert!(!facts.merge_in_progress && !facts.has_remotes);
    write(&repo, "a.txt", "x\n");
    assert_eq!(
        guidance_for(&repo.status().unwrap(), &facts).situation,
        Situation::Changes
    );
    repo.stage(&["a.txt".to_owned()]).unwrap();
    assert_eq!(
        guidance_for(&repo.status().unwrap(), &facts).situation,
        Situation::ReadyToCommit
    );
    repo.commit("base", false).unwrap();
    assert_eq!(
        guidance_for(&repo.status().unwrap(), &facts).situation,
        Situation::Clean
    );
}
