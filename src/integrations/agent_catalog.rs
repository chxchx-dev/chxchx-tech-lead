//! Model and effort catalogs exposed by installed agent providers.
//!
//! These reads only inspect provider configuration/cache files. OpenCode's documented `models`
//! command is used for models that its local config does not enumerate.
use crate::features::agents::{Agent, Provider};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelOption {
    pub id: String,
    pub label: String,
    pub efforts: Vec<String>,
    pub default_effort: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentOptions {
    pub available: bool,
    pub install_hint: String,
    pub default_model: String,
    pub default_effort: String,
    pub models: Vec<ModelOption>,
}

const CODEX_FALLBACK_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const CLAUDE_MODELS: &[&str] = &[
    "sonnet",
    "opus",
    "haiku",
    "fable",
    "opusplan",
    "sonnet[1m]",
    "opus[1m]",
];
const CLAUDE_VERSIONS: &[&str] = &[
    "claude-fable-5-2",
    "claude-fable-5-1",
    "claude-opus-5-5",
    "claude-sonnet-5-5",
    "claude-haiku-5-5",
    "claude-opus-4-5",
    "claude-sonnet-4-5",
    "claude-haiku-4-5",
    "claude-opus-4-1",
];
const CLAUDE_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max", "ultracode"];
const TRANSCRIPT_PREFIX_BYTES: u64 = 2_000_000;
const TRANSCRIPT_COUNT: usize = 25;

pub fn options(agent: &Agent, project: &Path) -> AgentOptions {
    let codex_home = codex_home();
    let claude_config_dir = claude_config_dir();
    let opencode_config_home = opencode_config_home();
    options_in(
        agent,
        project,
        &codex_home,
        &claude_config_dir,
        &opencode_config_home,
    )
}

/// Builds the catalog from explicit provider directories. Studio uses [`options`]; explicit roots
/// make provider fixtures deterministic without mutating process-wide environment variables.
pub fn options_in(
    agent: &Agent,
    project: &Path,
    codex_home: &Path,
    claude_config_dir: &Path,
    opencode_config_home: &Path,
) -> AgentOptions {
    let available = agent.executable.as_ref().is_some_and(|path| path.is_file());
    let mut options = AgentOptions {
        available,
        install_hint: install_hint(agent.provider).to_owned(),
        ..Default::default()
    };
    match agent.provider {
        Provider::Codex => apply_codex(&mut options, codex_home),
        Provider::Claude => apply_claude(&mut options, claude_config_dir),
        Provider::OpenCode => apply_opencode(
            &mut options,
            opencode_config_home,
            agent.executable.as_deref(),
            project,
        ),
        Provider::Other => {}
    }
    options
}

fn install_hint(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "curl -fsSL https://claude.ai/install.sh | bash",
        Provider::Codex => "npm install -g @openai/codex",
        Provider::OpenCode => "npm install -g opencode-ai",
        Provider::Other => "",
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn env_path(name: &str, fallback: PathBuf) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or(fallback)
}

fn codex_home() -> PathBuf {
    env_path("CODEX_HOME", home().join(".codex"))
}

fn claude_config_dir() -> PathBuf {
    env_path("CLAUDE_CONFIG_DIR", home().join(".claude"))
}

fn opencode_config_home() -> PathBuf {
    env_path("XDG_CONFIG_HOME", home().join(".config"))
}

fn apply_codex(options: &mut AgentOptions, home: &Path) {
    let config = fs::read_to_string(home.join("config.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .unwrap_or_default();
    options.default_model = config
        .get("model")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    options.default_effort = config
        .get("model_reasoning_effort")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let cache: Value = match fs::read(home.join("models_cache.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(cache) => cache,
        None => return,
    };
    for model in cache
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if model.get("visibility").and_then(Value::as_str) != Some("list") {
            continue;
        }
        let Some(id) = model.get("slug").and_then(Value::as_str) else {
            continue;
        };
        let efforts: Vec<String> = model
            .get("supported_reasoning_levels")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|level| {
                level
                    .get("effort")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect();
        options.models.push(ModelOption {
            id: id.to_owned(),
            label: model
                .get("display_name")
                .and_then(Value::as_str)
                .filter(|label| !label.is_empty())
                .unwrap_or(id)
                .to_owned(),
            efforts: if efforts.is_empty() {
                CODEX_FALLBACK_EFFORTS
                    .iter()
                    .map(|effort| (*effort).to_owned())
                    .collect()
            } else {
                efforts
            },
            default_effort: model
                .get("default_reasoning_level")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        });
    }
}

fn apply_claude(options: &mut AgentOptions, config_dir: &Path) {
    let settings: Value = fs::read(config_dir.join("settings.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);
    let settings = settings.as_object();
    options.default_model = settings
        .and_then(|settings| settings.get("model"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    options.default_effort = settings
        .and_then(|settings| settings.get("effortLevel"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let mut model_ids = Vec::new();
    for model in CLAUDE_MODELS.iter().chain(CLAUDE_VERSIONS) {
        append_unique(&mut model_ids, model);
    }
    let mut extra_models = BTreeSet::new();
    if let Some(available) = settings
        .and_then(|settings| settings.get("availableModels"))
        .and_then(Value::as_array)
    {
        extra_models.extend(
            available
                .iter()
                .filter_map(Value::as_str)
                .filter(|model| !model.is_empty())
                .map(str::to_owned),
        );
    }
    extra_models.extend(anthropic_environment_models());
    extra_models.extend(transcript_models(config_dir));
    for model in extra_models {
        append_unique(&mut model_ids, &model);
    }
    if !options.default_model.is_empty() {
        append_unique(&mut model_ids, &options.default_model);
    }
    options.models = model_ids
        .into_iter()
        .map(|id| ModelOption {
            label: id.clone(),
            id,
            efforts: CLAUDE_EFFORTS
                .iter()
                .map(|effort| (*effort).to_owned())
                .collect(),
            default_effort: String::new(),
        })
        .collect();
}

fn anthropic_environment_models() -> BTreeSet<String> {
    std::env::vars_os()
        .filter_map(|(key, value)| {
            let key = key.to_string_lossy();
            if !key.starts_with("ANTHROPIC_DEFAULT_") || !key.ends_with("_MODEL") {
                return None;
            }
            let value = value.to_string_lossy().trim().to_owned();
            (!value.is_empty()).then_some(value)
        })
        .collect()
}

fn transcript_models(config_dir: &Path) -> BTreeSet<String> {
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    let Ok(projects) = fs::read_dir(config_dir.join("projects")) else {
        return BTreeSet::new();
    };
    for project in projects.flatten() {
        let Ok(entries) = fs::read_dir(project.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "jsonl")
                && path.is_file()
                && let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified())
            {
                files.push((path, modified));
            }
        }
    }
    files.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    files
        .into_iter()
        .take(TRANSCRIPT_COUNT)
        .flat_map(|(path, _)| model_ids_from_transcript(&path))
        .collect()
}

fn model_ids_from_transcript(path: &Path) -> BTreeSet<String> {
    let Ok(file) = fs::File::open(path) else {
        return BTreeSet::new();
    };
    let mut bytes = Vec::new();
    if file
        .take(TRANSCRIPT_PREFIX_BYTES)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return BTreeSet::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut found = BTreeSet::new();
    let mut rest = text.as_ref();
    while let Some(index) = rest.find("\"model\"") {
        rest = &rest[index + "\"model\"".len()..];
        let Some(colon) = rest.find(':') else { break };
        rest = rest[colon + 1..].trim_start();
        let Some(value) = rest.strip_prefix('"') else {
            continue;
        };
        let mut length = value
            .bytes()
            .take_while(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
            })
            .count();
        if value[length..].starts_with("[1m]") {
            length += "[1m]".len();
        }
        let id = &value[..length];
        if id.starts_with("claude-") {
            found.insert(id.to_owned());
        }
        rest = &value[length.min(value.len())..];
    }
    found
}

fn apply_opencode(
    options: &mut AgentOptions,
    config_home: &Path,
    executable: Option<&Path>,
    project: &Path,
) {
    let config = read_opencode_config(config_home);
    options.default_model = config
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut models: Vec<String> = Vec::new();
    if let Some(executable) = executable {
        let command = vec![executable.display().to_string(), "models".to_owned()];
        if let Ok(output) = crate::integrations::mcp_clients::run(&command, project)
            && output.status == 0
        {
            models.extend(
                output
                    .stdout
                    .lines()
                    .map(str::trim)
                    .filter(|line| line.contains('/') && !line.contains(' '))
                    .map(str::to_owned),
            );
        }
    }
    if let Some(providers) = config.get("provider").and_then(Value::as_object) {
        for (provider, spec) in providers {
            if let Some(entries) = spec.get("models").and_then(Value::as_object) {
                models.extend(entries.keys().map(|model| format!("{provider}/{model}")));
            }
        }
    }
    append_unique(&mut models, &options.default_model);
    let mut unique = BTreeSet::new();
    options.models = models
        .into_iter()
        .filter(|id| id.contains('/') && unique.insert(id.clone()))
        .map(|id| ModelOption {
            label: id.clone(),
            id,
            ..Default::default()
        })
        .collect();
}

fn read_opencode_config(config_home: &Path) -> Value {
    for name in ["opencode.json", "opencode.jsonc"] {
        if let Ok(bytes) = fs::read(config_home.join("opencode").join(name))
            && let Ok(config) = serde_json::from_slice::<Value>(&bytes)
            && config.is_object()
        {
            return config;
        }
    }
    Value::Null
}

fn append_unique(models: &mut Vec<String>, model: &str) {
    if !model.is_empty() && !models.contains(&model.to_owned()) {
        models.push(model.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "chxchx-agent-catalog-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn codex_uses_its_config_and_only_visible_cached_models() {
        let home = scratch("codex");
        fs::write(
            home.join("config.toml"),
            "model='gpt-x'\nmodel_reasoning_effort='high'\n",
        )
        .unwrap();
        fs::write(
            home.join("models_cache.json"),
            r#"{"models":[{"slug":"gpt-x","display_name":"GPT X","visibility":"list","supported_reasoning_levels":[{"effort":"low"},{"effort":"high"}],"default_reasoning_level":"low"},{"slug":"hidden","visibility":"hidden"},{"slug":"fallback","visibility":"list"}]}"#,
        )
        .unwrap();

        let mut options = AgentOptions::default();
        apply_codex(&mut options, &home);

        assert_eq!(options.default_model, "gpt-x");
        assert_eq!(options.default_effort, "high");
        assert_eq!(options.models.len(), 2);
        assert_eq!(options.models[0].id, "gpt-x");
        assert_eq!(options.models[0].efforts, ["low", "high"]);
        assert_eq!(options.models[1].efforts, CODEX_FALLBACK_EFFORTS);
    }

    #[test]
    fn claude_includes_aliases_defaults_available_models_and_recent_transcripts() {
        let config = scratch("claude");
        fs::write(
            config.join("settings.json"),
            r#"{"model":"custom-default","effortLevel":"high","availableModels":["custom-a","opus"]}"#,
        )
        .unwrap();
        let transcript_dir = config.join("projects/project-a");
        fs::create_dir_all(&transcript_dir).unwrap();
        fs::write(
            transcript_dir.join("chat.jsonl"),
            r#"{"message":{"model":"claude-custom-5-3"}}{"message":{"model":"claude-sonnet-4-5[1m]"}}"#,
        )
        .unwrap();
        let mut options = AgentOptions::default();
        apply_claude(&mut options, &config);

        let ids: BTreeSet<_> = options
            .models
            .iter()
            .map(|model| model.id.as_str())
            .collect();
        assert_eq!(options.default_model, "custom-default");
        assert_eq!(options.default_effort, "high");
        assert_eq!(options.models.first().unwrap().id, "sonnet");
        assert_eq!(options.models.last().unwrap().id, "custom-default");
        assert!(ids.contains("sonnet") && ids.contains("claude-opus-4-1"));
        assert!(ids.contains("custom-a") && ids.contains("custom-default"));
        assert!(ids.contains("claude-custom-5-3"));
        assert!(ids.contains("claude-sonnet-4-5[1m]"));
        assert!(
            options
                .models
                .iter()
                .all(|model| model.efforts.len() == CLAUDE_EFFORTS.len())
        );
    }

    #[test]
    fn opencode_merges_provider_models_cli_models_and_default_without_duplicates() {
        let config = serde_json::from_str::<Value>(
            r#"{"model":"local/one","provider":{"local":{"models":{"one":{},"two":{}}}}}"#,
        )
        .unwrap();
        let mut models = vec!["cloud/three".to_owned(), "local/one".to_owned()];
        for (provider, spec) in config["provider"].as_object().unwrap() {
            models.extend(
                spec["models"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(|model| format!("{provider}/{model}")),
            );
        }
        append_unique(&mut models, config["model"].as_str().unwrap());
        let mut unique = BTreeSet::new();
        let result: Vec<_> = models
            .into_iter()
            .filter(|id| id.contains('/') && unique.insert(id.clone()))
            .collect();
        assert_eq!(result, ["cloud/three", "local/one", "local/two"]);
    }
}
