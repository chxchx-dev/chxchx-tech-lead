//! Shared Rust product. Interfaces parse input and render these capabilities.

pub mod features {
    pub mod doctor;
    pub mod mcp_diagnostics;
    pub mod mcp_setup;
    pub mod project_backup;
    pub mod project_catalog;
    pub mod project_config;
    pub mod project_config_migrations;
    pub mod project_config_template;
    pub mod project_detection;
    pub mod project_overview;
    pub mod project_profiles;
    pub mod project_registry;
    pub mod project_rollback;
    pub mod project_setup;
    pub mod project_sync;
    pub mod project_templates;
    pub mod project_trust;
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
