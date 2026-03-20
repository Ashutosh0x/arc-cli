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

// Re-export the error for convenience.
pub use error::ArcError;
