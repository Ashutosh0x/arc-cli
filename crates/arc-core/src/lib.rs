#![forbid(unsafe_code)]

pub mod config;
pub mod credentials;
pub mod error;
pub mod memory;
pub mod models;
pub mod model_picker;
pub mod security;
pub mod shutdown;
pub mod telemetry;
pub mod setup_wizard;
pub mod auth;

pub mod network;
pub mod budget;
pub mod instance_lock;

// Phase 28: Gemini Parity — Runtime Intelligence & Production Safety
pub mod loop_detection;
pub mod tool_masking;
pub mod jit_context;
pub mod ide_detect;
pub mod billing;
pub mod prompt_registry;

// Phase 29-33: Claude Code Parity — Foundation, Competitive, DX, Enterprise
pub mod hooks;
pub mod permissions;
pub mod compaction;
pub mod sandbox_policy;
pub mod worktree;
pub mod cron;
pub mod plan_mode;
pub mod slash_commands;
pub mod auto_memory;
pub mod agent_defs;
pub mod settings;
pub mod effort;
pub mod feature_flags;
pub mod background_tasks;
pub mod platform;

// Phase 29-33: Remaining gap closures (100% completion)
pub mod plugin_marketplace;
pub mod voice;
pub mod agent_teams;
pub mod skills;
pub mod statusline;
pub mod copy_picker;
pub mod security_review;
pub mod tool_search;
pub mod ralph_loop;
pub mod pr_review;
// Phase 34: Audit gap closures — HTTP hooks, hot-reload, wildcards, extended features
pub mod http_hooks;
pub mod hot_reload_skills;
pub mod wildcard_permissions;
pub mod extended_features;

// Re-export the error for convenience.
pub use error::ArcError;
