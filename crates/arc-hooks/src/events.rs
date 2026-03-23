// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookEvent {
    /// Triggered by standard git pre-commit hook
    PreCommit,
    /// Triggered by standard git post-commit hook
    PostCommit,
    /// Triggered artificially by an IDE or watcher when a file is saved
    OnSave { file: String },
}
