//! Plain data produced by parsing `git` output. No processes here, so every shape
//! can be tested from literal text.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Ordinary,
    Renamed,
    Unmerged,
    Untracked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitEntry {
    pub kind: EntryKind,
    pub path: String,
    /// Renames only.
    pub orig_path: Option<String>,
    /// X of the porcelain status: change already prepared for commit.
    pub index: char,
    /// Y: change in the working tree.
    pub work: char,
}

impl GitEntry {
    pub fn conflicted(&self) -> bool {
        self.kind == EntryKind::Unmerged
    }

    pub fn untracked(&self) -> bool {
        self.kind == EntryKind::Untracked
    }

    pub fn staged(&self) -> bool {
        matches!(self.kind, EntryKind::Ordinary | EntryKind::Renamed) && self.index != '.'
    }

    pub fn unstaged(&self) -> bool {
        matches!(self.kind, EntryKind::Ordinary | EntryKind::Renamed) && self.work != '.'
    }

    /// Letter shown next to the file: M modified, N new, D deleted, R renamed, ! conflict.
    pub fn badge(&self) -> char {
        if self.conflicted() {
            return '!';
        }
        if self.untracked() {
            return 'N';
        }
        let code = if self.work != '.' {
            self.work
        } else {
            self.index
        };
        match code {
            'A' => 'N',
            other => other,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GitStatus {
    /// Branch name, or the short oid when detached.
    pub head: String,
    pub oid: String,
    pub detached: bool,
    /// No commits yet.
    pub initial: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<GitEntry>,
}

impl GitStatus {
    pub fn conflicts(&self) -> Vec<&GitEntry> {
        self.entries.iter().filter(|e| e.conflicted()).collect()
    }

    pub fn staged_entries(&self) -> Vec<&GitEntry> {
        self.entries.iter().filter(|e| e.staged()).collect()
    }

    /// Edited tracked files plus files git does not track yet.
    pub fn changed_entries(&self) -> Vec<&GitEntry> {
        self.entries
            .iter()
            .filter(|e| e.unstaged() || e.untracked())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitBranch {
    /// `main` or `origin/main`.
    pub name: String,
    pub current: bool,
    pub remote: bool,
    pub upstream: String,
    /// "ahead 2, behind 1".
    pub track: String,
    pub hash: String,
    pub subject: String,
}

/// How one history row is drawn. Lanes are columns; the node sits on `lane`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphRow {
    pub lane: usize,
    /// A line arrives at the node from above.
    pub incoming: bool,
    /// Lanes whose line passes straight through the row.
    pub through: Vec<usize>,
    /// Other lanes that curve into the node from above.
    pub merge_in: Vec<usize>,
    /// Lane each parent continues on, in parent order.
    pub parent_lanes: Vec<usize>,
    pub lane_count: usize,
}

#[derive(Clone, Debug)]
pub struct GitCommit {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
    /// "HEAD -> main", "origin/main", "tag: v1".
    pub refs: Vec<String>,
    pub subject: String,
    pub graph: GraphRow,
}

impl GitCommit {
    pub fn short_hash(&self) -> &str {
        &self.hash[..self.hash.len().min(7)]
    }
}

#[derive(Clone, Debug)]
pub struct GitStash {
    pub index: usize,
    pub time: i64,
    /// "On main: arreglo a medias".
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct GitFileChange {
    /// M A D R C T.
    pub status: String,
    pub path: String,
    pub orig_path: Option<String>,
}
