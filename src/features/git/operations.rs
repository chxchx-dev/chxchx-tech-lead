//! Changes to a repository. Each one runs a single, explicit git command.
use super::problem::GitFailure;
use super::repository::{Repository, safe_path};
use super::types::{EntryKind, GitEntry};
use crate::integrations::git::Run;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;

const NETWORK_TIMEOUT: Duration = Duration::from_secs(120);
const PATHS_PER_COMMAND: usize = 100;
const KEPT_BACKUPS: usize = 10;
const MAX_BACKUP_BYTES: usize = 16 * 1024 * 1024;
pub const BACKUP_FOLDER: &str = "chxchx-backups";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullMode {
    FastForwardOnly,
    Merge,
}

type Done = Result<String, GitFailure>;

impl Repository {
    pub(super) fn with_paths(&self, base: &[&str], paths: &[String]) -> Result<(), GitFailure> {
        for path in paths {
            safe_path(path)?;
        }
        for chunk in paths.chunks(PATHS_PER_COMMAND) {
            let mut args: Vec<String> = base.iter().map(|arg| (*arg).to_owned()).collect();
            args.push("--".to_owned());
            args.extend(chunk.iter().cloned());
            self.ok(Run::new(args))?;
        }
        Ok(())
    }

    pub fn stage(&self, paths: &[String]) -> Result<(), GitFailure> {
        self.with_paths(&["add"], paths)
    }

    pub fn unstage(&self, paths: &[String], initial_commit: bool) -> Result<(), GitFailure> {
        if initial_commit {
            self.with_paths(&["rm", "--cached", "-r", "-q"], paths)
        } else {
            self.with_paths(&["restore", "--staged"], paths)
        }
    }

    /// Saves a copy of what is about to be lost inside `.git`, then discards.
    /// Returns the backup folder; nothing is discarded when the copy cannot be made.
    pub fn discard(
        &self,
        entries: &[GitEntry],
        initial_commit: bool,
    ) -> Result<Option<PathBuf>, GitFailure> {
        let (mut untracked, mut added, mut restore) = (Vec::new(), Vec::new(), Vec::new());
        for entry in entries {
            match entry.kind {
                EntryKind::Untracked => untracked.push(entry.path.clone()),
                EntryKind::Renamed => {
                    added.push(entry.path.clone());
                    restore.extend(entry.orig_path.clone());
                }
                _ if entry.index == 'A' => added.push(entry.path.clone()),
                _ => restore.push(entry.path.clone()),
            }
        }
        for path in untracked.iter().chain(&added).chain(&restore) {
            safe_path(path)?;
        }
        let backup = self.save_discard_backup(
            &untracked,
            !added.is_empty() || !restore.is_empty(),
            initial_commit,
        )?;
        if !added.is_empty() {
            self.with_paths(&["rm", "-f", "-q"], &added)?;
        }
        if !restore.is_empty() {
            self.with_paths(
                &["restore", "--source=HEAD", "--staged", "--worktree"],
                &restore,
            )?;
        }
        for path in &untracked {
            let _ = fs::remove_file(self.root.join(path));
        }
        Ok(backup)
    }

    fn save_discard_backup(
        &self,
        untracked: &[String],
        tracked: bool,
        initial_commit: bool,
    ) -> Result<Option<PathBuf>, GitFailure> {
        if untracked.is_empty() && !tracked {
            return Ok(None);
        }
        let folder = self.git_dir()?.join(BACKUP_FOLDER);
        let backup = folder.join(OffsetDateTime::now_utc().unix_timestamp_nanos().to_string());
        let fail = |what: &str| {
            GitFailure::other(format!(
                "No pude guardar una copia de seguridad de los cambios ({what}), así que no descarté nada."
            ))
        };
        fs::create_dir_all(&backup).map_err(|_| fail("carpeta"))?;
        for path in untracked {
            let target = backup.join("untracked").join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|_| fail("carpeta"))?;
            }
            fs::copy(self.root.join(path), &target).map_err(|_| fail("archivo nuevo"))?;
        }
        if tracked {
            let args: &[&str] = if initial_commit {
                &["diff", "--cached", "--binary", "--no-color"]
            } else {
                &["diff", "HEAD", "--binary", "--no-color"]
            };
            let output = self.exec(
                Run::new(args.iter().copied())
                    .read_only()
                    .max_output(MAX_BACKUP_BYTES),
            )?;
            if !output.ok() {
                return Err(fail("diferencias"));
            }
            fs::write(backup.join("changes.patch"), &output.stdout).map_err(|_| fail("parche"))?;
        }
        prune_backups(&folder);
        Ok(Some(backup))
    }

    /// Commits what is prepared; with `stage_all`, prepares everything first.
    pub fn commit(&self, message: &str, stage_all: bool) -> Done {
        let message = message.trim();
        if message.is_empty() {
            return Err(GitFailure::other(
                "Escribe un mensaje para describir el cambio.",
            ));
        }
        if stage_all {
            self.ok(Run::new(["add", "-A"]))?;
        }
        let output = self.ok(Run::new(["commit", "-F", "-"]).input(message.as_bytes().to_vec()))?;
        Ok(output
            .text()
            .lines()
            .next()
            .unwrap_or("Cambios guardados.")
            .to_owned())
    }

    fn check_branch_name(&self, name: &str) -> Result<(), GitFailure> {
        if name.is_empty() || name.starts_with('-') || name.contains(char::is_whitespace) {
            return Err(GitFailure::other("Nombre de rama no válido."));
        }
        self.ok(Run::new(["check-ref-format", "--branch", name]).read_only())
            .map(|_| ())
            .map_err(|_| GitFailure::other(format!("«{name}» no es un nombre de rama válido.")))
    }

    pub fn switch_branch(&self, name: &str, remote: bool) -> Done {
        self.check_branch_name(name)?;
        let args: Vec<&str> = if remote {
            vec!["switch", "--track", name]
        } else {
            vec!["switch", name]
        };
        self.ok(Run::new(args))
            .map(|_| format!("Ahora estás en «{name}»."))
    }

    pub fn create_branch(&self, name: &str, start: Option<&str>) -> Done {
        self.check_branch_name(name)?;
        let mut args = vec!["switch", "-c", name];
        args.extend(start);
        self.ok(Run::new(args))
            .map(|_| format!("Rama «{name}» creada y activa."))
    }

    pub fn delete_branch(&self, name: &str, force: bool) -> Done {
        self.check_branch_name(name)?;
        let flag = if force { "-D" } else { "-d" };
        self.ok(Run::new(["branch", flag, name]))
            .map(|_| format!("Rama «{name}» eliminada."))
    }

    pub fn fetch(&self) -> Done {
        self.ok(Run::new(["fetch", "--prune"]).timeout(NETWORK_TIMEOUT))
            .map(|_| "Novedades del remoto descargadas.".to_owned())
    }

    pub fn pull(&self, mode: PullMode) -> Done {
        let args: &[&str] = match mode {
            PullMode::FastForwardOnly => &["pull", "--ff-only"],
            PullMode::Merge => &["pull", "--no-rebase", "--no-edit"],
        };
        let output = self.ok(Run::new(args.iter().copied()).timeout(NETWORK_TIMEOUT))?;
        Ok(output
            .text()
            .lines()
            .last()
            .unwrap_or("Cambios recibidos.")
            .to_owned())
    }

    /// Plain `push`, or with `publish` the branch is created on the remote and tracked.
    pub fn push(&self, publish: Option<(&str, &str)>) -> Done {
        let mut args = vec!["push"];
        if let Some((remote, branch)) = publish {
            self.check_branch_name(branch)?;
            self.check_branch_name(remote)?;
            args.extend(["-u", remote, branch]);
        }
        self.ok(Run::new(args).timeout(NETWORK_TIMEOUT))
            .map(|_| "Cambios enviados.".to_owned())
    }

    pub fn abort_merge(&self) -> Done {
        self.ok(Run::new(["merge", "--abort"]))
            .map(|_| "Fusión cancelada.".to_owned())
    }

    pub fn stash_push(&self, message: &str, include_untracked: bool) -> Done {
        let mut args = vec!["stash".to_owned(), "push".to_owned()];
        if include_untracked {
            args.push("-u".to_owned());
        }
        if !message.trim().is_empty() {
            args.push("-m".to_owned());
            args.push(message.trim().to_owned());
        }
        self.ok(Run::new(args))
            .map(|output| output.text().trim().to_owned())
    }

    pub fn stash_apply(&self, index: usize) -> Done {
        self.ok(Run::new([
            "stash".to_owned(),
            "apply".to_owned(),
            format!("stash@{{{index}}}"),
        ]))
        .map(|_| "Apartado aplicado.".to_owned())
    }

    pub fn stash_pop(&self, index: usize) -> Done {
        self.ok(Run::new([
            "stash".to_owned(),
            "pop".to_owned(),
            format!("stash@{{{index}}}"),
        ]))
        .map(|_| "Apartado recuperado.".to_owned())
    }

    pub fn stash_drop(&self, index: usize) -> Done {
        self.ok(Run::new([
            "stash".to_owned(),
            "drop".to_owned(),
            format!("stash@{{{index}}}"),
        ]))
        .map(|_| "Apartado eliminado.".to_owned())
    }
}

/// Keeps only the newest backups.
fn prune_backups(folder: &std::path::Path) {
    let Ok(read) = fs::read_dir(folder) else {
        return;
    };
    let mut names: Vec<PathBuf> = read
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    names.sort_by_key(|path| {
        path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<i128>().ok())
    });
    let excess = names.len().saturating_sub(KEPT_BACKUPS);
    for old in names.into_iter().take(excess) {
        let _ = fs::remove_dir_all(old);
    }
}
