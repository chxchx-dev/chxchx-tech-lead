//! Parsers for the machine-readable output of git.
use super::types::{
    EntryKind, GitBranch, GitCommit, GitEntry, GitFileChange, GitStash, GitStatus, GraphRow,
};

/// `git status --porcelain=v2 --branch -z`.
pub fn parse_status(output: &str) -> GitStatus {
    let mut status = GitStatus::default();
    let mut records = output.split('\0');
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        if let Some(oid) = record.strip_prefix("# branch.oid ") {
            status.oid = oid.to_owned();
            status.initial = oid == "(initial)";
        } else if let Some(head) = record.strip_prefix("# branch.head ") {
            status.detached = head == "(detached)";
            status.head = head.to_owned();
        } else if let Some(upstream) = record.strip_prefix("# branch.upstream ") {
            status.upstream = Some(upstream.to_owned());
        } else if let Some(counts) = record.strip_prefix("# branch.ab ") {
            for part in counts.split_whitespace() {
                if let Some(ahead) = part.strip_prefix('+') {
                    status.ahead = ahead.parse().unwrap_or(0);
                } else if let Some(behind) = part.strip_prefix('-') {
                    status.behind = behind.parse().unwrap_or(0);
                }
            }
        } else if let Some(rest) = record.strip_prefix("1 ") {
            // XY sub mH mI mW hH hI path
            let fields: Vec<&str> = rest.splitn(8, ' ').collect();
            if let [xy, .., path] = fields.as_slice() {
                status
                    .entries
                    .push(entry(EntryKind::Ordinary, xy, path, None));
            }
        } else if let Some(rest) = record.strip_prefix("2 ") {
            // XY sub mH mI mW hH hI Xscore path, then the original path as its own record.
            let fields: Vec<&str> = rest.splitn(9, ' ').collect();
            let original = records.next().unwrap_or_default();
            if let [xy, .., path] = fields.as_slice() {
                status.entries.push(entry(
                    EntryKind::Renamed,
                    xy,
                    path,
                    Some(original.to_owned()),
                ));
            }
        } else if let Some(rest) = record.strip_prefix("u ") {
            // XY sub m1 m2 m3 mW h1 h2 h3 path
            let fields: Vec<&str> = rest.splitn(10, ' ').collect();
            if let [xy, .., path] = fields.as_slice() {
                status
                    .entries
                    .push(entry(EntryKind::Unmerged, xy, path, None));
            }
        } else if let Some(path) = record.strip_prefix("? ") {
            status.entries.push(GitEntry {
                kind: EntryKind::Untracked,
                path: path.to_owned(),
                orig_path: None,
                index: '.',
                work: '?',
            });
        }
    }
    if status.detached {
        status.head = status.oid.chars().take(7).collect();
    }
    status
}

fn entry(kind: EntryKind, xy: &str, path: &str, orig_path: Option<String>) -> GitEntry {
    let mut codes = xy.chars();
    GitEntry {
        kind,
        path: path.to_owned(),
        orig_path,
        index: codes.next().unwrap_or('.'),
        work: codes.next().unwrap_or('.'),
    }
}

pub const BRANCH_FORMAT: &str = "%(HEAD)%09%(refname)%09%(upstream:short)%09%(upstream:track,nobracket)%09%(objectname:short)%09%(subject)";

pub fn parse_branches(output: &str) -> Vec<GitBranch> {
    output
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.splitn(6, '\t').collect();
            let [head, name, upstream, track, hash, subject] = fields.as_slice() else {
                return None;
            };
            let (short, remote) = match name.strip_prefix("refs/heads/") {
                Some(short) => (short, false),
                None => (name.strip_prefix("refs/remotes/")?, true),
            };
            // `origin/HEAD` is an alias, not a branch to switch to.
            if remote && short.ends_with("/HEAD") {
                return None;
            }
            Some(GitBranch {
                name: short.to_owned(),
                current: head.trim() == "*",
                remote,
                upstream: (*upstream).to_owned(),
                track: (*track).to_owned(),
                hash: (*hash).to_owned(),
                subject: (*subject).to_owned(),
            })
        })
        .collect()
}

pub const LOG_FORMAT: &str = "%H%x1f%P%x1f%an%x1f%ct%x1f%D%x1f%s%x1e";

pub fn parse_log(output: &str) -> Vec<GitCommit> {
    output
        .split('\x1e')
        .filter_map(|record| {
            let fields: Vec<&str> = record.trim_start_matches('\n').split('\x1f').collect();
            let [hash, parents, author, time, refs, subject] = fields.as_slice() else {
                return None;
            };
            if hash.is_empty() {
                return None;
            }
            Some(GitCommit {
                hash: (*hash).to_owned(),
                parents: parents.split_whitespace().map(str::to_owned).collect(),
                author: (*author).to_owned(),
                time: time.parse().unwrap_or(0),
                refs: refs
                    .split(", ")
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect(),
                subject: (*subject).to_owned(),
                graph: GraphRow::default(),
            })
        })
        .collect()
}

pub const STASH_FORMAT: &str = "%gd%x1f%ct%x1f%gs";

pub fn parse_stashes(output: &str) -> Vec<GitStash> {
    output
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.splitn(3, '\x1f').collect();
            let [name, time, message] = fields.as_slice() else {
                return None;
            };
            let index = name
                .strip_prefix("stash@{")?
                .strip_suffix('}')?
                .parse()
                .ok()?;
            Some(GitStash {
                index,
                time: time.parse().unwrap_or(0),
                message: (*message).to_owned(),
            })
        })
        .collect()
}

/// `git diff-tree --name-status -r -M -z`: `M\0path\0` and `R100\0old\0new\0`.
pub fn parse_name_status(output: &str) -> Vec<GitFileChange> {
    let mut tokens = output.split('\0').filter(|token| !token.is_empty());
    let mut changes = Vec::new();
    while let Some(code) = tokens.next() {
        let status: String = code.chars().take(1).collect();
        if matches!(status.as_str(), "R" | "C") {
            let (Some(old), Some(new)) = (tokens.next(), tokens.next()) else {
                break;
            };
            changes.push(GitFileChange {
                status,
                path: new.to_owned(),
                orig_path: Some(old.to_owned()),
            });
        } else if let Some(path) = tokens.next() {
            changes.push(GitFileChange {
                status,
                path: path.to_owned(),
                orig_path: None,
            });
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_reads_branch_counts_renames_conflicts_and_untracked() {
        let text = "# branch.oid 0123456789abcdef\0# branch.head main\0# branch.upstream origin/main\0\
# branch.ab +1 -2\0\
1 M. N... 100644 100644 100644 aaa bbb src/app.py\0\
1 .M N... 100644 100644 100644 aaa bbb docs/with space.md\0\
2 R. N... 100644 100644 100644 aaa bbb R100 new name.py\0old name.py\0\
u UU N... 100644 100644 100644 100644 a b c conflicto.rs\0\
? notas.md\0";
        let status = parse_status(text);
        assert_eq!(
            (status.head.as_str(), status.ahead, status.behind),
            ("main", 1, 2)
        );
        assert_eq!(status.upstream.as_deref(), Some("origin/main"));
        assert_eq!(status.entries.len(), 5);
        assert_eq!(status.entries[1].path, "docs/with space.md");
        assert!(status.entries[1].unstaged() && !status.entries[1].staged());
        assert_eq!(status.entries[2].orig_path.as_deref(), Some("old name.py"));
        assert_eq!(status.conflicts().len(), 1);
        assert_eq!(status.staged_entries().len(), 2);
        assert_eq!(
            status.changed_entries().len(),
            2,
            "edited file plus untracked file"
        );
        assert_eq!(status.entries[4].badge(), 'N');
        assert_eq!(status.entries[0].badge(), 'M');
    }

    #[test]
    fn detached_and_initial_states_are_recognised() {
        let detached = parse_status("# branch.oid abcdef1234567\0# branch.head (detached)\0");
        assert!(detached.detached);
        assert_eq!(detached.head, "abcdef1");
        let fresh = parse_status("# branch.oid (initial)\0# branch.head main\0");
        assert!(fresh.initial && !fresh.detached);
    }

    #[test]
    fn history_branches_stashes_and_name_status_parse() {
        let log = "aaa1111\x1fbbb2222 ccc3333\x1fAna\x1f1700000000\x1fHEAD -> main, tag: v1\x1fMerge\x1e\nbbb2222\x1f\x1fLuis\x1f1690000000\x1f\x1fRaíz\x1e";
        let commits = parse_log(log);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].parents.len(), 2);
        assert_eq!(commits[0].refs, ["HEAD -> main", "tag: v1"]);
        assert_eq!(commits[1].parents.len(), 0);
        assert_eq!(commits[0].short_hash(), "aaa1111");

        let branches = parse_branches(
            "*\trefs/heads/main\torigin/main\tahead 2\tabc1234\tUltimo\n \trefs/remotes/origin/HEAD\t\t\tabc1234\tx\n \trefs/heads/feature\t\t\tdef5678\tOtro\n \trefs/remotes/origin/main\t\t\tabc1234\tUltimo\n",
        );
        assert_eq!(branches.len(), 3);
        assert!(branches[0].current && branches[0].track == "ahead 2");
        assert!(branches[2].remote && branches[2].name == "origin/main");

        let stashes = parse_stashes(
            "stash@{0}\x1f1700000000\x1fOn main: idea\nstash@{1}\x1f1600000000\x1fWIP\n",
        );
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[1].index, 1);

        let files = parse_name_status("M\0src/a.rs\0R100\0old.rs\0new.rs\0D\0gone.rs\0");
        assert_eq!(files.len(), 3);
        assert_eq!(files[1].orig_path.as_deref(), Some("old.rs"));
        assert_eq!(files[2].status, "D");
    }
}
