// SPDX-License-Identifier: MIT
//! # arc-shadow
//!
//! Shadow Workspace implementation. Clones the current project state into
//! a hidden temporary directory, allowing the LLM to run builds and test
//! mutations without destroying the user's working copy.

pub mod workspace;

pub use workspace::{ShadowOptions, ShadowWorkspace};
