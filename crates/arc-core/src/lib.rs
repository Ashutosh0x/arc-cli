#![forbid(unsafe_code)]

pub mod auth;
pub mod config;
pub mod credentials;
pub mod error;
pub mod memory;
pub mod model_picker;
pub mod models;
pub mod security;
pub mod setup_wizard;
pub mod shutdown;
pub mod telemetry;

pub mod budget;
pub mod instance_lock;
pub mod network;

// Phase 28: Gemini Parity — Runtime Intelligence & Production Safety
pub mod billing;
pub mod ide_detect;
pub mod jit_context;
pub mod loop_detection;
pub mod prompt_registry;
pub mod tool_masking;

// Phase 29-33: Claude Code Parity — Foundation, Competitive, DX, Enterprise
pub mod agent_defs;
pub mod auto_memory;
pub mod background_tasks;
pub mod compaction;
pub mod cron;
pub mod effort;
pub mod feature_flags;
pub mod hooks;
pub mod permissions;
pub mod plan_mode;
pub mod platform;
pub mod sandbox_policy;
pub mod settings;
pub mod slash_commands;
pub mod worktree;

// Phase 29-33: Remaining gap closures (100% completion)
pub mod agent_teams;
pub mod copy_picker;
pub mod plugin_marketplace;
pub mod pr_review;
pub mod ralph_loop;
pub mod security_review;
pub mod skills;
pub mod statusline;
pub mod tool_search;
pub mod voice;
// Phase 34: Audit gap closures — HTTP hooks, hot-reload, wildcards, extended features
pub mod extended_features;
pub mod hot_reload_skills;
pub mod http_hooks;
pub mod wildcard_permissions;

// Re-export the error for convenience.
pub use error::ArcError;
