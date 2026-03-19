//! # arc-hooks
//!
//! Enables automated execution of tasks at specific events, particularly
//! Git lifecycle events (pre-commit, post-checkout) or IDE save events.
//! Useful for auto-generating docs, fixing formatting, or writing PR summaries.

pub mod executor;
pub mod events;
pub mod rewrite;
pub mod config;
pub mod git_intel;

pub use executor::{HookExecutor, HookConfig};
pub use events::HookEvent;
pub use rewrite::HookSystemRewrite;
