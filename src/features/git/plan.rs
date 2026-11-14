//! Reading what changed (for suggestions) and committing one group of files.
use super::problem::GitFailure;
use super::repository::{Repository, safe_path};
use super::suggest::{ChangeInfo, fix_words, is_sensitive};
use super::types::GitEntry;
use crate::integrations::git::Run;
use std::collections::HashMap;
use std::fs;

const MAX_DIFF_BYTES: usize = 1024 * 1024;

impl Repository {
    /// Size and nature of each pending change. Files that look like secrets are never read.
    pub fn analyse(
        &self,
        entries: &[GitEntry],
        initial: bool,
    ) -> Result<Vec<ChangeInfo>, GitFailure> {
        let tracked = entries
            .iter()
            .any(|entry| !entry.untracked() && !entry.conflicted());
        let (mut counts, mut hints) = (HashMap::new(), HashMap::<String, u32>::new());
        if tracked {
            let base: &[&str] = if initial {
                &["diff", "--cached"]
            } else {
                &["diff", "HEAD"]
            };
            let numstat = Run::new(
                base.iter()
                    .copied()
                    .chain(["--numstat", "-z", "--no-color"]),
            )
            .read_only();
            counts = parse_numstat(&self.ok(numstat)?.text());
            let patch = Run::new(base.iter().copied().chain([
                "--unified=0",
                "--no-color",
                "--no-ext-diff",
            ]))
            .read_only()
            .max_output(MAX_DIFF_BYTES);
            // A truncated patch only weakens the hints; the counts above stay exact.
            let output = self.exec(patch)?;
            hints = count_hints(&String::from_utf8_lossy(&output.stdout));
        }
        let mut changes = Vec::new();
        for entry in entries.iter().filter(|entry| !entry.conflicted()) {
            let sensitive = is_sensitive(&entry.path);
            let (mut additions, deletions) = counts.get(&entry.path).copied().unwrap_or((0, 0));
            let mut fix_hints = if sensitive {
                0
            } else {
                hints.get(&entry.path).copied().unwrap_or(0)
            };
            if entry.untracked()
                && !sensitive
                && safe_path(&entry.path).is_ok()
                && let Ok(text) = fs::read_to_string(self.root.join(&entry.path))
                && text.len() <= MAX_DIFF_BYTES
            {
                additions = text.lines().count() as u32;
                fix_hints = text.lines().map(fix_words).sum();
            }
            changes.push(ChangeInfo {
                path: entry.path.clone(),
                orig_path: entry.orig_path.clone(),
                badge: entry.badge(),
                additions,
                deletions,
                fix_hints,
                sensitive,
            });
        }
        Ok(changes)
    }

    /// Commits exactly `paths`, leaving everything else (even if prepared) as it was.
    pub fn commit_paths(
        &self,
        message: &str,
        paths: &[String],
        initial: bool,
    ) -> Result<String, GitFailure> {
        if message.trim().is_empty() {
            return Err(GitFailure::other(
                "Escribe un mensaje para describir el cambio.",
            ));
        }
        if paths.is_empty() {
            return Err(GitFailure::other("Este commit no tiene archivos."));
        }
        for path in paths {
            safe_path(path)?;
        }
        // Stages additions, edits and deletions of just these paths.
        self.with_paths(&["add"], paths)?;
        let mut args = vec!["commit".to_owned(), "-F".to_owned(), "-".to_owned()];
        if !initial {
            args.push("--".to_owned());
            args.extend(paths.iter().cloned());
        }
        let output = self.ok(Run::new(args).input(message.trim().as_bytes().to_vec()))?;
        Ok(output
            .text()
            .lines()
            .next()
            .unwrap_or("Cambios guardados.")
            .to_owned())
    }
}

/// `git diff --numstat -z`: `A\tD\tpath\0`, or `A\tD\t\0old\0new\0` for a rename.
fn parse_numstat(output: &str) -> HashMap<String, (u32, u32)> {
    let mut counts = HashMap::new();
    let mut tokens = output.split('\0').filter(|token| !token.is_empty());
    while let Some(token) = tokens.next() {
        let mut fields = token.splitn(3, '\t');
        let (Some(added), Some(removed), path) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let numbers = (added.parse().unwrap_or(0), removed.parse().unwrap_or(0));
        match path {
            Some(path) if !path.is_empty() => {
                counts.insert(path.to_owned(), numbers);
            }
            _ => {
                let (_old, new) = (tokens.next(), tokens.next());
                if let Some(new) = new {
                    counts.insert(new.to_owned(), numbers);
                }
            }
        }
    }
    counts
}

/// Per file, how many added lines talk about fixing something.
fn count_hints(patch: &str) -> HashMap<String, u32> {
    let mut hints = HashMap::new();
    let mut current: Option<String> = None;
    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            let path = path.trim_end_matches('\t').to_owned();
            current = (!is_sensitive(&path)).then_some(path);
        } else if line.starts_with("+++ ") {
            current = None;
        } else if let (Some(added), Some(path)) = (line.strip_prefix('+'), &current) {
            let count = fix_words(added);
            if count > 0 {
                *hints.entry(path.clone()).or_insert(0) += count;
            }
        }
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numstat_and_patch_hints_parse() {
        let counts = parse_numstat("3\t1\tsrc/a.rs\0-\t-\timg.png\u{0}5\t0\t\0old.rs\0new.rs\0");
        assert_eq!(counts["src/a.rs"], (3, 1));
        assert_eq!(counts["img.png"], (0, 0));
        assert_eq!(counts["new.rs"], (5, 0));
        let hints = count_hints(
            "+++ b/src/a.rs\n+// fix: arregla el fallo\n+let x = 1;\n+++ b/.env\n+TOKEN=fix\n",
        );
        assert_eq!(hints.get("src/a.rs"), Some(&2));
        assert!(!hints.contains_key(".env"), "secrets are not scanned");
    }
}
