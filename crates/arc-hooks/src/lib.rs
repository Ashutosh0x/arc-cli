// SPDX-License-Identifier: MIT
//! # arc-hooks
//!
//! Enables automated execution of tasks at specific events, particularly
//! Git lifecycle events (pre-commit, post-checkout) or IDE save events.
//! Useful for auto-generating docs, fixing formatting, or writing PR summaries.

pub mod config;
pub mod events;
pub mod executor;
pub mod git_intel;
pub mod rewrite;

pub use events::HookEvent;
pub use executor::{HookConfig, HookExecutor};
pub use rewrite::HookSystemRewrite;
