//! Bundled catalog of Skills, Tech Packs and Mods, and how it matches a project.
use crate::features::project_detection::ProjectDetection;
use std::sync::OnceLock;

mod data {
    include!(concat!(env!("OUT_DIR"), "/catalog_data.rs"));
}

#[derive(Clone, Debug)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub stacks: Vec<String>,
    pub languages: Vec<String>,
    pub tags: Vec<String>,
    pub always_recommend: bool,
    pub instructions: String,
}

#[derive(Clone, Debug)]
pub struct TechPack {
    pub name: String,
    pub description: String,
    pub stacks: Vec<String>,
    pub languages: Vec<String>,
    pub skills: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Mod {
    pub name: String,
    pub description: String,
    pub stacks: Vec<String>,
    pub languages: Vec<String>,
    pub infrastructure: Vec<String>,
    pub always_recommend: bool,
    pub instructions: String,
}

pub fn skills() -> &'static [Skill] {
    static CACHE: OnceLock<Vec<Skill>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut found: Vec<Skill> = data::SKILLS
            .iter()
            .filter_map(|(_, manifest, instructions)| {
                let values: toml::Table = toml::from_str(manifest).ok()?;
                Some(Skill {
                    name: text(&values, "name")?,
                    description: text(&values, "description")?,
                    stacks: list(&values, "stacks"),
                    languages: list(&values, "languages"),
                    tags: list(&values, "tags"),
                    always_recommend: flag(&values, "always_recommend"),
                    instructions: clean(instructions),
                })
            })
            .collect();
        found.sort_by(|left, right| left.name.cmp(&right.name));
        found
    })
}

pub fn packs() -> &'static [TechPack] {
    static CACHE: OnceLock<Vec<TechPack>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut found: Vec<TechPack> = data::PACKS
            .iter()
            .filter_map(|(_, manifest, _)| {
                let values: toml::Table = toml::from_str(manifest).ok()?;
                Some(TechPack {
                    name: text(&values, "name")?,
                    description: text(&values, "description")?,
                    stacks: list(&values, "stacks"),
                    languages: list(&values, "languages"),
                    skills: list(&values, "skills"),
                })
            })
            .collect();
        found.sort_by(|left, right| left.name.cmp(&right.name));
        found
    })
}

pub fn mods() -> &'static [Mod] {
    static CACHE: OnceLock<Vec<Mod>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut found: Vec<Mod> = data::MODS
            .iter()
            .filter_map(|(_, manifest, instructions)| {
                let values: toml::Table = toml::from_str(manifest).ok()?;
                Some(Mod {
                    name: text(&values, "name")?,
                    description: text(&values, "description")?,
                    stacks: list(&values, "stacks"),
                    languages: list(&values, "languages"),
                    infrastructure: list(&values, "infrastructure"),
                    always_recommend: flag(&values, "always_recommend"),
                    instructions: clean(instructions),
                })
            })
            .collect();
        found.sort_by(|left, right| left.name.cmp(&right.name));
        found
    })
}

pub fn skill(name: &str) -> Option<&'static Skill> {
    skills()
        .iter()
        .find(|skill| skill.name.eq_ignore_ascii_case(name))
}

pub fn pack(name: &str) -> Option<&'static TechPack> {
    packs()
        .iter()
        .find(|pack| pack.name.eq_ignore_ascii_case(name))
}

pub fn module(name: &str) -> Option<&'static Mod> {
    mods()
        .iter()
        .find(|module| module.name.eq_ignore_ascii_case(name))
}

/// Why an item is recommended for a detected project; empty when it is not.
pub fn reasons(
    detection: &ProjectDetection,
    stacks: &[String],
    languages: &[String],
    infrastructure: &[String],
    always: Option<&'static str>,
) -> Vec<String> {
    let mut found = Vec::new();
    found.extend(
        stacks
            .iter()
            .filter(|item| detection.stacks.contains(item))
            .map(|item| format!("stack detectado: {item}")),
    );
    found.extend(
        languages
            .iter()
            .filter(|item| detection.languages.contains(item))
            .map(|item| format!("lenguaje detectado: {item}")),
    );
    found.extend(
        infrastructure
            .iter()
            .filter(|item| detection.infrastructure.contains(item))
            .map(|item| format!("infraestructura detectada: {item}")),
    );
    if let Some(always) = always {
        found.push(always.to_owned());
    }
    found
}

pub fn skill_reasons(detection: &ProjectDetection, skill: &Skill) -> Vec<String> {
    reasons(
        detection,
        &skill.stacks,
        &skill.languages,
        &[],
        skill
            .always_recommend
            .then_some("guía general aplicable a cualquier proyecto"),
    )
}

pub fn pack_reasons(detection: &ProjectDetection, pack: &TechPack) -> Vec<String> {
    reasons(detection, &pack.stacks, &pack.languages, &[], None)
}

pub fn mod_reasons(detection: &ProjectDetection, module: &Mod) -> Vec<String> {
    reasons(
        detection,
        &module.stacks,
        &module.languages,
        &module.infrastructure,
        module
            .always_recommend
            .then_some("flujo general recomendado"),
    )
}

fn text(values: &toml::Table, key: &str) -> Option<String> {
    values.get(key)?.as_str().map(str::to_owned)
}

fn list(values: &toml::Table, key: &str) -> Vec<String> {
    values
        .get(key)
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn flag(values: &toml::Table, key: &str) -> bool {
    values
        .get(key)
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
}

/// Instructions are rendered into project files: normalise line endings and trim.
fn clean(instructions: &str) -> String {
    instructions.replace("\r\n", "\n").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_complete_and_consistent() {
        assert_eq!(skills().len(), 21, "skills bundled");
        assert_eq!(packs().len(), 13, "tech packs bundled");
        assert_eq!(mods().len(), 23, "mods bundled");
        for pack in packs() {
            for name in &pack.skills {
                assert!(
                    skill(name).is_some(),
                    "{} references unknown {name}",
                    pack.name
                );
            }
        }
        for skill in skills() {
            assert!(
                !skill.instructions.is_empty(),
                "{} has no instructions",
                skill.name
            );
            assert!(!skill.instructions.contains('\r'));
        }
        for module in mods() {
            assert!(
                !module.instructions.is_empty(),
                "{} has no instructions",
                module.name
            );
        }
    }
}
