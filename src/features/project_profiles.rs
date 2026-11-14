use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const BUILTIN_PROFILES: &str = include_str!("../../assets/profiles.toml");

#[derive(Clone, Debug)]
pub struct ResolvedProfile {
    pub name: String,
    pub rules: Vec<String>,
}

#[derive(Clone, Debug)]
struct Profile {
    stacks: Vec<String>,
    priority: i64,
    rules: Vec<String>,
}

pub fn resolve_profile(stacks: &[String]) -> ResolvedProfile {
    let available = load_profiles();
    let detected: BTreeSet<&str> = stacks.iter().map(String::as_str).collect();
    let selected = available
        .iter()
        .filter(|(_, profile)| {
            profile
                .stacks
                .iter()
                .all(|stack| detected.contains(stack.as_str()))
        })
        .max_by(|(left_name, left), (right_name, right)| {
            (left.priority, left.stacks.len(), left_name.as_str()).cmp(&(
                right.priority,
                right.stacks.len(),
                right_name.as_str(),
            ))
        });

    match selected {
        Some((name, profile)) => ResolvedProfile {
            name: name.clone(),
            rules: profile.rules.clone(),
        },
        None => ResolvedProfile {
            name: "generic".to_owned(),
            rules: Vec::new(),
        },
    }
}

fn load_profiles() -> BTreeMap<String, Profile> {
    let mut profiles = read_profiles(BUILTIN_PROFILES);
    let Some(directory) = application_home().map(|home| home.join("profiles")) else {
        return profiles;
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return profiles;
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let Ok(contents) = fs::read_to_string(path) else {
            continue;
        };
        profiles.extend(read_profiles(&contents));
    }
    profiles
}

fn read_profiles(contents: &str) -> BTreeMap<String, Profile> {
    let Ok(document) = toml::from_str::<toml::Value>(contents) else {
        return BTreeMap::new();
    };
    let Some(values) = document.get("profiles").and_then(toml::Value::as_table) else {
        return BTreeMap::new();
    };
    values
        .iter()
        .filter_map(|(name, value)| {
            let profile = value.as_table()?;
            let stacks = string_array(profile.get("stacks"));
            let rules = string_array(profile.get("rules"));
            let priority = profile
                .get("priority")
                .and_then(toml::Value::as_integer)
                .unwrap_or(stacks.len() as i64);
            Some((
                name.clone(),
                Profile {
                    stacks,
                    priority,
                    rules,
                },
            ))
        })
        .collect()
}

fn string_array(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub fn application_home() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("CHXCHX_TECH_HOME") {
        let path = PathBuf::from(value);
        if path == Path::new("~") {
            return std::env::home_dir();
        }
        let value = path.to_string_lossy();
        if let Some(rest) = value
            .strip_prefix("~/")
            .or_else(|| value.strip_prefix("~\\"))
        {
            return std::env::home_dir().map(|home| home.join(rest));
        }
        return Some(path);
    }
    std::env::home_dir().map(|home| home.join(".chxchx-tech-lead"))
}
