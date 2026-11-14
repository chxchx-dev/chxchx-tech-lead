//! Shared Rust product. Interfaces parse input and render these capabilities.

pub mod features {
    pub mod catalog;
    pub mod catalog_view;
    pub mod doctor;
    pub mod git;
    pub mod mcp_diagnostics;
    pub mod mcp_setup;
    pub mod project_backup;
    pub mod project_catalog;
    pub mod project_changes;
    pub mod project_config;
    pub mod project_config_migrations;
    pub mod project_config_template;
    pub mod project_detection;
    pub mod project_files;
    pub mod project_mod_agents;
    pub mod project_mods;
    pub mod project_overview;
    pub mod project_profiles;
    pub mod project_registry;
    pub mod project_rollback;
    pub mod project_search;
    pub mod project_setup;
    pub mod project_skills;
    pub mod project_sync;
    pub mod project_templates;
    pub mod project_trust;
    pub mod startup_log;
    pub mod studio_preferences;
    #[cfg(feature = "terminal")]
    pub mod workspace_terminal;
}

pub mod integrations;

pub mod interfaces {
    #[cfg(feature = "cli")]
    pub mod cli;

    #[cfg(feature = "studio")]
    pub mod studio;

    #[cfg(feature = "tui")]
    pub mod tui;
}

pub use features::project_overview::{ProjectOverview, inspect_project};
