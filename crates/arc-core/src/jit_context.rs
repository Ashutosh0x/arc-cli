// SPDX-License-Identifier: MIT
//! JIT (Just-In-Time) Context Loading
//!
//! Dynamically discovers and loads ARC.md context files when the agent
//! accesses new directories via high-intent tools (read_file, write_file, etc.).

use std::path::{Path, PathBuf};

pub const CONTEXT_FILE_NAME: &str = "ARC.md";
pub const JIT_CONTEXT_PREFIX: &str = "\n\n--- Newly Discovered Project Context ---\n";
pub const JIT_CONTEXT_SUFFIX: &str = "\n--- End Project Context ---";

/// High-intent tools that trigger JIT context discovery.
pub const HIGH_INTENT_TOOLS: &[&str] = &[
    "read_file",
    "write_file",
    "file_edit",
    "file_read",
    "list_directory",
    "replace",
    "read_many_files",
    "shell",
];

/// Discover JIT context for a given file or directory path.
/// Walks up from the accessed path looking for ARC.md files in directories
/// that haven't been loaded yet.
pub fn discover_jit_context(
    accessed_path: &Path,
    trusted_roots: &[PathBuf],
    already_loaded: &[PathBuf],
) -> Vec<JitContextEntry> {
    let mut results = Vec::new();
    let dir = if accessed_path.is_file() {
        accessed_path.parent().unwrap_or(accessed_path)
    } else {
        accessed_path
    };

    // Walk up from the accessed directory, collecting ARC.md files
    let mut current = Some(dir);
    while let Some(d) = current {
        // Stop if we've reached a trusted root boundary
        let at_root = trusted_roots.iter().any(|r| d == r.as_path());

        let context_file = d.join(CONTEXT_FILE_NAME);
        if context_file.exists() && !already_loaded.contains(&context_file) {
            if let Ok(content) = std::fs::read_to_string(&context_file) {
                if !content.trim().is_empty() {
                    results.push(JitContextEntry {
                        path: context_file,
                        content,
                        directory: d.to_path_buf(),
                    });
                }
            }
        }

        if at_root {
            break;
        }
        current = d.parent();
    }

    // Also check subdirectories of the accessed path for ARC.md
    if accessed_path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(accessed_path) {
            for entry in entries.flatten() {
                if entry.file_type().map_or(false, |t| t.is_dir()) {
                    let sub_context = entry.path().join(CONTEXT_FILE_NAME);
                    if sub_context.exists() && !already_loaded.contains(&sub_context) {
                        if let Ok(content) = std::fs::read_to_string(&sub_context) {
                            if !content.trim().is_empty() {
                                results.push(JitContextEntry {
                                    path: sub_context,
                                    content,
                                    directory: entry.path(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

#[derive(Debug, Clone)]
pub struct JitContextEntry {
    pub path: PathBuf,
    pub content: String,
    pub directory: PathBuf,
}

/// Append JIT context to tool output if any was discovered.
pub fn append_jit_context(tool_output: &str, jit_context: &str) -> String {
    if jit_context.is_empty() {
        return tool_output.to_string();
    }
    format!("{tool_output}{JIT_CONTEXT_PREFIX}{jit_context}{JIT_CONTEXT_SUFFIX}")
}

/// Format multiple JIT context entries into a single string.
pub fn format_jit_entries(entries: &[JitContextEntry]) -> String {
    entries
        .iter()
        .map(|e| format!("## Context from {}\n\n{}", e.directory.display(), e.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}
