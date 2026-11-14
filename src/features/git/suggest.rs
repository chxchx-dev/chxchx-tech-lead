//! Commit suggestions: classifies the pending files and groups them into commits that
//! follow Conventional Commits (`feat`, `fix`, `docs`…). Entirely local: nothing is sent
//! anywhere, and files that look like secrets are never read.
use std::collections::BTreeMap;

#[path = "suggest_rules.rs"]
mod rules;
use rules::{Class, area, body_for, classify, describe, kind_for_source, names};
#[path = "suggest_checks.rs"]
mod checks;
#[path = "suggest_split.rs"]
mod split;
pub use checks::{fix_words, is_conventional, is_sensitive};
use split::split_bucket;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommitType {
    Feat,
    Fix,
    Refactor,
    Perf,
    Docs,
    Test,
    Build,
    Ci,
    Style,
    Chore,
}

impl CommitType {
    pub const ALL: [CommitType; 10] = [
        Self::Feat,
        Self::Fix,
        Self::Refactor,
        Self::Perf,
        Self::Docs,
        Self::Test,
        Self::Build,
        Self::Ci,
        Self::Style,
        Self::Chore,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Feat => "feat",
            Self::Fix => "fix",
            Self::Refactor => "refactor",
            Self::Perf => "perf",
            Self::Docs => "docs",
            Self::Test => "test",
            Self::Build => "build",
            Self::Ci => "ci",
            Self::Style => "style",
            Self::Chore => "chore",
        }
    }

    /// What the type means, for tooltips.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Feat => "Una función nueva o una mejora visible para quien usa el producto.",
            Self::Fix => "La corrección de un error.",
            Self::Refactor => "Reorganizar código sin cambiar lo que hace.",
            Self::Perf => "Un cambio que hace algo más rápido o ligero.",
            Self::Docs => "Solo documentación.",
            Self::Test => "Añadir o corregir pruebas.",
            Self::Build => "Compilación, dependencias o empaquetado.",
            Self::Ci => "Integración continua y despliegue.",
            Self::Style => "Formato que no cambia el comportamiento.",
            Self::Chore => "Mantenimiento: configuración y archivos del proyecto.",
        }
    }

    /// The next type in the list; used to cycle when the suggestion is not right.
    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|kind| *kind == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

/// One pending file with what is known about its change.
#[derive(Clone, Debug)]
pub struct ChangeInfo {
    pub path: String,
    pub orig_path: Option<String>,
    /// M modified, N new, D deleted, R renamed.
    pub badge: char,
    pub additions: u32,
    pub deletions: u32,
    /// Added lines that talk about fixing something.
    pub fix_hints: u32,
    pub sensitive: bool,
}

#[derive(Clone, Debug)]
pub struct CommitGroup {
    pub kind: CommitType,
    pub scope: Option<String>,
    /// The description after `type(scope): `; editable.
    pub text: String,
    /// Repository paths to commit together (renames include both names).
    pub files: Vec<String>,
    pub body: Vec<String>,
}

impl CommitGroup {
    pub fn prefix(&self) -> String {
        match &self.scope {
            Some(scope) => format!("{}({scope})", self.kind.label()),
            None => self.kind.label().to_owned(),
        }
    }

    pub fn subject(&self) -> String {
        format!("{}: {}", self.prefix(), self.text.trim())
    }

    pub fn message(&self) -> String {
        if self.body.is_empty() {
            self.subject()
        } else {
            format!("{}\n\n{}", self.subject(), self.body.join("\n"))
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub groups: Vec<CommitGroup>,
    pub warnings: Vec<String>,
}

const MAX_GROUPS: usize = 12;
const MAX_SUBJECT: usize = 72;

fn group_from(
    class: Class,
    scope: Option<String>,
    label: Option<String>,
    files: Vec<&ChangeInfo>,
) -> CommitGroup {
    let kind = match class {
        Class::Source => kind_for_source(&files),
        Class::Test => CommitType::Test,
        Class::Docs => CommitType::Docs,
        Class::Build => CommitType::Build,
        Class::Ci => CommitType::Ci,
        Class::Chore => CommitType::Chore,
    };
    let mut text = describe(kind, class, &files);
    if let Some(label) = label.filter(|_| files.len() > 3) {
        // A cluster is named by what its files share, not by listing them.
        text = text.replacen(
            &names(&files),
            &format!("{label} y archivos relacionados"),
            1,
        );
    }
    let room = MAX_SUBJECT
        .saturating_sub(kind.label().len() + scope.as_ref().map_or(0, |s| s.len() + 2) + 2);
    if text.chars().count() > room {
        text = if class == Class::Test {
            format!("actualiza pruebas ({} archivos)", files.len())
        } else {
            format!("actualiza {} archivos", files.len())
        };
    }
    let mut paths = Vec::new();
    for file in &files {
        paths.push(file.path.clone());
        paths.extend(file.orig_path.clone());
    }
    CommitGroup {
        kind,
        scope,
        text,
        files: paths,
        body: body_for(&files),
    }
}

/// Splits the pending files into commits that each tell one story.
/// One future commit while the plan is being built: class, scope, family label and files.
type Piece<'a> = (Class, Option<String>, Option<String>, Vec<&'a ChangeInfo>);

pub fn plan(changes: &[ChangeInfo]) -> Plan {
    let mut warnings = Vec::new();
    // Source and tests split by module; everything else is one commit per class.
    let mut buckets: BTreeMap<(u8, String), (Class, Vec<&ChangeInfo>)> = BTreeMap::new();
    for change in changes {
        if change.sensitive {
            warnings.push(format!(
                "«{}» parece contener secretos o claves: revisa antes de guardarlo. No se analizó su contenido.",
                change.path
            ));
        }
        let class = classify(&change.path);
        let key = match class {
            Class::Source | Class::Test => (class as u8, area(&change.path).unwrap_or_default()),
            _ => (class as u8, String::new()),
        };
        buckets
            .entry(key)
            .or_insert_with(|| (class, Vec::new()))
            .1
            .push(change);
    }
    // Every part becomes one commit: (class, scope, family label, files).
    let mut parts: Vec<Piece> = Vec::new();
    for ((_, folder), (class, files)) in buckets {
        if folder.is_empty() {
            // A class with a single module still names it.
            let mut areas: Vec<Option<String>> = files.iter().map(|f| area(&f.path)).collect();
            areas.dedup();
            // CI and similar classes are already named by their type.
            let scope = (areas.len() == 1 && class != Class::Ci)
                .then(|| areas.remove(0))
                .flatten();
            parts.push((class, scope, None, files));
            continue;
        }
        for (sub_scope, label, part) in split_bucket(files, &folder) {
            parts.push((
                class,
                sub_scope.or_else(|| Some(folder.clone())),
                label,
                part,
            ));
        }
    }
    // Too many commits are as unhelpful as one giant commit: fold the two smallest parts of
    // the same class together, and stop when only unrelated parts are left.
    while parts.len() > MAX_GROUPS {
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by_key(|index| parts[*index].3.len());
        let pair = order.iter().enumerate().find_map(|(position, first)| {
            order[position + 1..]
                .iter()
                .find(|second| parts[**second].0 == parts[*first].0)
                .map(|second| (*first, *second))
        });
        let Some((first, second)) = pair else {
            break;
        };
        let (class, scope, _, mut files) = parts.remove(second.max(first));
        let keep = second.min(first);
        let merged_scope = if parts[keep].1 == scope { scope } else { None };
        parts[keep].1 = merged_scope;
        parts[keep].2 = None;
        parts[keep].3.append(&mut files);
        parts[keep].0 = class;
    }
    let mut groups: Vec<CommitGroup> = parts
        .into_iter()
        .map(|(class, scope, label, files)| group_from(class, scope, label, files))
        .collect();
    groups.sort_by_key(|group| (group.kind != CommitType::Build, group.kind));
    for group in &groups {
        if group.files.len() > 25 {
            warnings.push(format!(
                "«{}» reúne {} archivos: considera dividirlo para que el historial quede claro.",
                group.subject(),
                group.files.len()
            ));
        }
    }
    Plan { groups, warnings }
}

/// One message for everything in `changes`, plus whether it mixes several topics.
pub fn suggest(changes: &[ChangeInfo]) -> Option<(CommitGroup, bool)> {
    let plan = plan(changes);
    match plan.groups.len() {
        0 => None,
        1 => plan.groups.into_iter().next().map(|group| (group, false)),
        _ => {
            let mut groups = plan.groups;
            groups.sort_by_key(|group| std::cmp::Reverse(group.files.len()));
            let dominant = groups[0].clone();
            let mut areas: Vec<String> = groups
                .iter()
                .filter_map(|group| group.scope.clone())
                .collect();
            areas.sort();
            areas.dedup();
            let text = if areas.len() > 1 {
                format!(
                    "actualiza varios módulos ({})",
                    areas.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )
            } else {
                "actualiza varios archivos".to_owned()
            };
            let all: Vec<String> = groups
                .iter()
                .flat_map(|group| group.files.clone())
                .collect();
            let body: Vec<String> = groups
                .iter()
                .flat_map(|group| group.body.clone())
                .take(12)
                .collect();
            Some((
                CommitGroup {
                    kind: dominant.kind,
                    scope: None,
                    text,
                    files: all,
                    body,
                },
                true,
            ))
        }
    }
}

#[cfg(test)]
#[path = "suggest_tests.rs"]
mod tests;
