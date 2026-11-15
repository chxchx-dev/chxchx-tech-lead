//! One git repository: opening it and reading its state. Changes live in `operations`.
use super::graph::assign_lanes;
use super::parse::{
    BRANCH_FORMAT, LOG_FORMAT, STASH_FORMAT, parse_branches, parse_log, parse_name_status,
    parse_stashes, parse_status,
};
use super::problem::{GitFailure, failure};
use super::types::{GitBranch, GitCommit, GitFileChange, GitStash, GitStatus};
use crate::integrations::git::{self, Output, Run};
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Repository {
    /// Top level of the work tree (the project may be a folder inside it).
    pub root: PathBuf,
    /// Path of the project inside the repository, `""` or `"sub/dir/"`.
    pub prefix: String,
    /// Untrusted projects never run the repository's own hooks.
    pub trusted: bool,
}

impl Repository {
    /// `Ok(None)` when `project` is not inside a repository.
    pub fn open(project: &Path, trusted: bool) -> Result<Option<Self>, GitFailure> {
        let run = Run::new(["rev-parse", "--show-toplevel", "--show-prefix"]).read_only();
        let output = git::run(project, &run).map_err(GitFailure::other)?;
        if !output.ok() {
            return if output.stderr.contains("not a git repository") {
                Ok(None)
            } else {
                Err(failure(&output))
            };
        }
        let text = output.text();
        let mut lines = text.lines();
        let root = lines.next().unwrap_or_default().trim();
        if root.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            root: PathBuf::from(root),
            prefix: lines.next().unwrap_or_default().trim().to_owned(),
            trusted,
        }))
    }

    pub fn init(project: &Path, trusted: bool) -> Result<Self, GitFailure> {
        let output = git::run(project, &Run::new(["init"])).map_err(GitFailure::other)?;
        if !output.ok() {
            return Err(failure(&output));
        }
        Self::open(project, trusted)?
            .ok_or_else(|| GitFailure::other("Git no creó el repositorio."))
    }

    /// Repository path -> project-relative path, or `None` when it lies outside the project.
    pub fn project_path(&self, path: &str) -> Option<String> {
        path.strip_prefix(self.prefix.as_str()).map(str::to_owned)
    }

    fn settings(&self) -> Vec<String> {
        // A repository can name a program to run on every `status`; never allow it.
        let mut settings = vec!["core.fsmonitor=false".to_owned()];
        if !self.trusted {
            let empty = std::env::temp_dir().join("chxchx-no-hooks");
            let _ = fs::create_dir_all(&empty);
            settings.push(format!("core.hooksPath={}", empty.display()));
        }
        settings
    }

    pub(super) fn exec(&self, run: Run) -> Result<Output, GitFailure> {
        let mut run = run.config(self.settings());
        run.args.insert(0, "--literal-pathspecs".to_owned());
        git::run(&self.root, &run).map_err(GitFailure::other)
    }

    pub(super) fn ok(&self, run: Run) -> Result<Output, GitFailure> {
        let output = self.exec(run)?;
        if output.ok() {
            Ok(output)
        } else {
            Err(failure(&output))
        }
    }

    pub fn status(&self) -> Result<GitStatus, GitFailure> {
        let run = Run::new([
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
        ])
        .read_only();
        Ok(parse_status(&self.ok(run)?.text()))
    }

    pub fn branches(&self) -> Result<Vec<GitBranch>, GitFailure> {
        let run = Run::new([
            "for-each-ref".to_owned(),
            format!("--format={BRANCH_FORMAT}"),
            "refs/heads".to_owned(),
            "refs/remotes".to_owned(),
        ])
        .read_only();
        let mut branches = parse_branches(&self.ok(run)?.text());
        branches
            .sort_by(|a, b| (a.remote, !a.current, &a.name).cmp(&(b.remote, !b.current, &b.name)));
        Ok(branches)
    }

    /// Newest first, across every branch, with the graph lanes filled in.
    pub fn log(&self, count: usize) -> Result<Vec<GitCommit>, GitFailure> {
        let run = Run::new([
            "log".to_owned(),
            // A stash is saved work, not history: its two internal commits would show up as noise in the graph.
            "--exclude=refs/stash".to_owned(),
            "--all".to_owned(),
            "--topo-order".to_owned(),
            format!("-n{count}"),
            format!("--format={LOG_FORMAT}"),
        ])
        .read_only();
        let output = self.exec(run)?;
        if !output.ok() {
            if output.stderr.contains("does not have any commits") {
                return Ok(Vec::new());
            }
            return Err(failure(&output));
        }
        let mut commits = parse_log(&output.text());
        assign_lanes(&mut commits);
        Ok(commits)
    }

    pub fn commit_files(&self, hash: &str) -> Result<Vec<GitFileChange>, GitFailure> {
        valid_hash(hash)?;
        let run = Run::new([
            "diff-tree",
            "--no-commit-id",
            "--name-status",
            "-r",
            "-M",
            "-z",
            "--root",
            hash,
        ])
        .read_only();
        Ok(parse_name_status(&self.ok(run)?.text()))
    }

    pub fn commit_message(&self, hash: &str) -> Result<String, GitFailure> {
        valid_hash(hash)?;
        let run = Run::new(["show", "-s", "--format=%B", hash]).read_only();
        Ok(self.ok(run)?.text().trim().to_owned())
    }

    pub fn commit_diff(&self, hash: &str, path: &str) -> Result<String, GitFailure> {
        valid_hash(hash)?;
        safe_path(path)?;
        let run = Run::new(["show", "--no-color", "--format=", "-M", hash, "--", path])
            .read_only()
            .max_output(MAX_DIFF_BYTES);
        self.diff_text(run)
    }

    /// `staged` compares the index with HEAD; otherwise the work tree with the index.
    pub fn file_diff(&self, path: &str, staged: bool) -> Result<String, GitFailure> {
        safe_path(path)?;
        let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
        if staged {
            args.push("--cached");
        }
        args.extend(["--", path]);
        self.diff_text(Run::new(args).read_only().max_output(MAX_DIFF_BYTES))
    }

    /// A file git does not track yet, shown as one big addition.
    pub fn untracked_diff(&self, path: &str) -> Result<String, GitFailure> {
        safe_path(path)?;
        let full = self.root.join(path);
        let metadata = fs::symlink_metadata(&full)
            .map_err(|error| GitFailure::other(format!("No se pudo leer {path}: {error}")))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(GitFailure::other("Solo se muestran archivos normales."));
        }
        if metadata.len() as usize > MAX_DIFF_BYTES / 2 {
            return Ok(format!(
                "El archivo nuevo {path} es demasiado grande para mostrarlo.\n"
            ));
        }
        let bytes = fs::read(&full)
            .map_err(|error| GitFailure::other(format!("No se pudo leer {path}: {error}")))?;
        let Ok(text) = String::from_utf8(bytes) else {
            return Ok(format!("Archivo nuevo binario o no UTF-8: {path}\n"));
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut diff = format!(
            "diff --git a/{path} b/{path}\nnew file\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n",
            lines.len()
        );
        for line in lines {
            diff.push('+');
            diff.push_str(line);
            diff.push('\n');
        }
        Ok(diff)
    }

    fn diff_text(&self, run: Run) -> Result<String, GitFailure> {
        let output = self.exec(run)?;
        if output.truncated {
            return Ok("La diferencia es demasiado grande para mostrarla completa.\n".to_owned());
        }
        if !output.ok() {
            return Err(failure(&output));
        }
        let text = output.text();
        Ok(if text.trim().is_empty() {
            "Sin diferencias que mostrar.\n".to_owned()
        } else {
            text
        })
    }

    pub fn stashes(&self) -> Result<Vec<GitStash>, GitFailure> {
        let run = Run::new([
            "stash".to_owned(),
            "list".to_owned(),
            format!("--format={STASH_FORMAT}"),
        ])
        .read_only();
        Ok(parse_stashes(&self.ok(run)?.text()))
    }

    pub fn remotes(&self) -> Result<Vec<String>, GitFailure> {
        let run = Run::new(["remote"]).read_only();
        Ok(self.ok(run)?.text().lines().map(str::to_owned).collect())
    }

    pub fn merge_in_progress(&self) -> bool {
        let run = Run::new(["rev-parse", "--git-path", "MERGE_HEAD"]).read_only();
        self.ok(run)
            .map(|output| self.root.join(output.text().trim()).exists())
            .unwrap_or(false)
    }

    pub fn git_dir(&self) -> Result<PathBuf, GitFailure> {
        let run = Run::new(["rev-parse", "--absolute-git-dir"]).read_only();
        Ok(PathBuf::from(self.ok(run)?.text().trim()))
    }
}

fn valid_hash(hash: &str) -> Result<(), GitFailure> {
    if (4..=64).contains(&hash.len()) && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(GitFailure::other("Identificador de commit no válido."))
    }
}

/// A repository-relative path that cannot escape it or be read as an option.
pub(super) fn safe_path(path: &str) -> Result<(), GitFailure> {
    let relative = Path::new(path);
    if path.is_empty()
        || path.starts_with('-')
        || path.contains('\0')
        || relative.is_absolute()
        || !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(GitFailure::other(format!("Ruta no válida: {path}")));
    }
    Ok(())
}
