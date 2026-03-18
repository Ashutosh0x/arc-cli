//! # arc-hooks
//!
//! Enables automated execution of tasks at specific events, particularly
//! Git lifecycle events (pre-commit, post-checkout) or IDE save events.
//! Useful for auto-generating docs, fixing formatting, or writing PR summaries.

pub mod executor;
pub mod events;
pub mod executor;
pub mod rewrite;
pub mod config;

pub use executor::{HookExecutor, HookConfig};
pub use events::HookEvent;

pub mod rewrite;
pub use rewrite::HookSystemRewrite;
