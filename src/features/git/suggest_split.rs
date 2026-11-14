//! Splits a module that grew too big into smaller stories.
use super::ChangeInfo;
use super::rules::{name_prefix, sub_area};
use std::collections::BTreeMap;

/// A module with more files than this is split by sub-module and by name prefix.
const SPLIT_ABOVE: usize = 12;

pub(super) type Part<'a> = (Option<String>, Option<String>, Vec<&'a ChangeInfo>);

/// Splits a big module into sub-modules and, inside those, into families of files that
/// share a name prefix. Returns (scope, label, files) parts.
pub(super) fn split_bucket<'a>(files: Vec<&'a ChangeInfo>, area_name: &str) -> Vec<Part<'a>> {
    if files.len() <= SPLIT_ABOVE {
        return vec![(None, None, files)];
    }
    let mut by_sub: BTreeMap<String, Vec<&ChangeInfo>> = BTreeMap::new();
    for file in files {
        by_sub
            .entry(sub_area(&file.path, area_name).unwrap_or_default())
            .or_default()
            .push(file);
    }
    let mut parts: Vec<Part> = Vec::new();
    for (sub, members) in by_sub {
        let scope = sub
            .rsplit('/')
            .next()
            .filter(|last| !last.is_empty())
            .map(str::to_owned);
        if members.len() <= SPLIT_ABOVE {
            parts.push((scope, None, members));
            continue;
        }
        let mut families: BTreeMap<String, Vec<&ChangeInfo>> = BTreeMap::new();
        for file in members {
            families
                .entry(name_prefix(&file.path))
                .or_default()
                .push(file);
        }
        let mut leftovers: Vec<&ChangeInfo> = Vec::new();
        for (prefix, family) in families {
            if family.len() >= 2 {
                parts.push((scope.clone(), Some(prefix), family));
            } else {
                leftovers.extend(family);
            }
        }
        if !leftovers.is_empty() {
            parts.push((scope, None, leftovers));
        }
    }
    parts
}
