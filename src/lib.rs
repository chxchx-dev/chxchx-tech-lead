//! Shared Rust product. Interfaces parse input and render these capabilities.

pub mod features {
    pub mod agent_launch;
    pub mod agents;
    pub mod catalog;
    pub mod catalog_view;
    pub mod chat_handoff;
    pub mod chat_journal;
    pub mod chat_model;
    pub mod chat_state;
    pub mod connection;
    pub mod context_compaction;
    pub mod data_home;
    pub mod doctor;
    pub mod error_cache;
    pub mod explicit_memory;
    pub mod explicit_memory_prune;
    pub mod git;
    pub mod identity;
    pub mod jobs;
    pub mod manager;
    pub mod mcp_connectors;
    pub mod mcp_connectors_plan;
    pub mod mcp_diagnostics;
    pub mod mcp_setup;
    pub mod node_audit;
    pub mod node_devices;
    pub mod node_state;
    pub mod project_backup;
    pub mod project_catalog;
    pub mod project_changes;
    pub mod project_config;
    pub mod project_config_migrations;
    pub mod project_config_template;
    pub mod project_detection;
    pub mod project_files;
    pub mod project_fs;
    pub mod project_mod_agents;
    pub mod project_mods;
    pub mod project_overview;
    pub mod project_profiles;
    pub mod project_registry;
    pub mod project_rollback;
    pub mod project_search;
    pub mod project_setup;
    pub mod project_skills;
    pub mod project_switch;
    pub mod project_sync;
    pub mod project_templates;
    pub mod project_trust;
    pub mod release_manifest;
    #[cfg(feature = "node")]
    pub mod remote_access;
    pub mod startup_log;
    pub mod studio_preferences;
    pub mod supervision;
    pub mod tools_install;
    pub mod workspace_lifecycle;
    pub mod workspace_processes;
    pub mod workspace_resources;
    #[cfg(feature = "terminal")]
    pub mod workspace_terminal;
}

pub mod integrations;

pub mod interfaces {
    #[cfg(feature = "cli")]
    pub mod cli;

    #[cfg(feature = "studio")]
    pub mod studio;

    #[cfg(feature = "visual-installer")]
    pub mod installer;

    #[cfg(feature = "node")]
    pub mod node;

    #[cfg(feature = "tui")]
    pub mod tui;
}

pub use features::project_overview::{ProjectOverview, inspect_project};
