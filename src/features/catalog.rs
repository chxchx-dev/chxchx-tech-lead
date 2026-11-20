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

/// The raw `(id, manifest, _)` entries of the MCP connectors (parsed by `mcp_connectors`).
pub(crate) fn connector_manifests() -> &'static [(&'static str, &'static str, &'static str)] {
    data::CONNECTORS
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
        assert_eq!(skills().len(), 40, "skills bundled");
        assert_eq!(packs().len(), 22, "tech packs bundled");
        assert_eq!(mods().len(), 31, "mods bundled");
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

    fn detection(stacks: &[&str], languages: &[&str]) -> ProjectDetection {
        ProjectDetection {
            stacks: stacks.iter().map(|s| (*s).to_owned()).collect(),
            languages: languages.iter().map(|s| (*s).to_owned()).collect(),
            package_managers: Vec::new(),
            infrastructure: Vec::new(),
            profile: "python".into(),
            profile_rules: Vec::new(),
        }
    }

    fn recommended_skills(found: &ProjectDetection) -> Vec<&'static str> {
        skills()
            .iter()
            .filter(|skill| !skill_reasons(found, skill).is_empty())
            .map(|skill| skill.name.as_str())
            .collect()
    }

    #[test]
    fn a_model_training_project_gets_the_training_skills_and_a_plain_python_project_does_not() {
        let ml = recommended_skills(&detection(&["python", "ml"], &["python"]));
        for wanted in [
            "llm-training-from-scratch",
            "llm-data-pipeline",
            "tokenizer-engineering",
            "llm-fine-tuning",
            "llm-evaluation",
            "llm-inference-serving",
            "pytorch-engineering",
            "gpu-performance",
            "ml-experiment-reproducibility",
            "model-safety-validation",
        ] {
            assert!(
                ml.contains(&wanted),
                "{wanted} should be recommended: {ml:?}"
            );
        }
        let plain = recommended_skills(&detection(&["python"], &["python"]));
        for unwanted in [
            "llm-training-from-scratch",
            "gpu-performance",
            "pytorch-engineering",
            "data-engineering",
        ] {
            assert!(
                !plain.contains(&unwanted),
                "{unwanted} must not follow every Python project"
            );
        }
    }

    #[test]
    fn an_app_that_calls_hosted_models_gets_rag_and_safety_but_not_training() {
        let ai = recommended_skills(&detection(&["python", "ai"], &["python"]));
        assert!(ai.contains(&"rag-engineering") && ai.contains(&"model-safety-validation"));
        assert!(ai.contains(&"llm-evaluation"));
        assert!(!ai.contains(&"llm-training-from-scratch"));
    }

    #[test]
    fn infrastructure_and_language_skills_follow_their_stacks() {
        assert!(recommended_skills(&detection(&["kubernetes"], &[])).contains(&"kubernetes"));
        assert!(recommended_skills(&detection(&["terraform"], &[])).contains(&"terraform"));
        assert!(recommended_skills(&detection(&["rust"], &["rust"])).contains(&"rust-engineering"));
        assert!(recommended_skills(&detection(&["go"], &["go"])).contains(&"go-engineering"));
        assert!(!recommended_skills(&detection(&["rust"], &["rust"])).contains(&"go-engineering"));
    }

    #[test]
    fn the_new_packs_bundle_what_their_goal_needs_and_are_recommended_by_stack() {
        let scratch = pack("llm-from-scratch").unwrap();
        for wanted in [
            "llm-training-from-scratch",
            "tokenizer-engineering",
            "llm-evaluation",
            "gpu-performance",
        ] {
            assert!(scratch.skills.iter().any(|s| s == wanted), "{wanted}");
        }
        assert!(!pack_reasons(&detection(&["ml"], &[]), scratch).is_empty());
        assert!(pack_reasons(&detection(&["python"], &["python"]), scratch).is_empty());
        for name in [
            "llm-fine-tuning",
            "llm-inference",
            "llm-applications",
            "rust-service",
            "go-service",
            "kubernetes-platform",
            "terraform-infrastructure",
            "data-platform",
        ] {
            let found = pack(name).unwrap_or_else(|| panic!("{name} missing"));
            assert!(found.skills.len() >= 4, "{name} bundles too little");
        }
    }

    #[test]
    fn the_new_mods_are_recommended_for_their_stacks_only() {
        let reasons = |name: &str, stacks: &[&str]| {
            mod_reasons(
                &detection(stacks, &[]),
                module(name).unwrap_or_else(|| panic!("{name} missing")),
            )
            .len()
        };
        assert!(reasons("chxchx-llm-training-guard", &["ml"]) > 0);
        assert_eq!(reasons("chxchx-llm-training-guard", &["python"]), 0);
        assert!(reasons("chxchx-llm-eval-gate", &["ai"]) > 0);
        assert!(reasons("chxchx-kubernetes-safety", &["kubernetes"]) > 0);
        assert!(reasons("chxchx-terraform-plan-first", &["terraform"]) > 0);
        assert_eq!(reasons("chxchx-terraform-plan-first", &["kubernetes"]), 0);
    }

    #[test]
    fn every_skill_is_real_guidance_not_a_stub() {
        for skill in skills() {
            assert!(
                skill.description.len() >= 20,
                "{} needs a description",
                skill.name
            );
        }
        // The new LLM and ML skills carry enough substance to act on.
        for name in [
            "llm-training-from-scratch",
            "llm-data-pipeline",
            "tokenizer-engineering",
            "llm-fine-tuning",
            "llm-evaluation",
            "llm-inference-serving",
            "pytorch-engineering",
            "gpu-performance",
            "ml-experiment-reproducibility",
            "model-safety-validation",
            "rag-engineering",
        ] {
            let lines = skill(name).unwrap().instructions.lines().count();
            assert!(lines >= 40, "{name} has only {lines} lines");
        }
    }
}
