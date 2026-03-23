// SPDX-License-Identifier: MIT
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::instrument;

/// Read-only tool set for the planning subagent.
/// These tools CANNOT write, delete, or execute anything.
/// They exist solely to gather information about the codebase.
pub struct ReadOnlyToolSet {
    /// Root directory the planner is allowed to read from.
    root: PathBuf,
    /// Maximum file size to read (prevents OOM on huge binaries).
    max_file_size: u64,
    /// Glob patterns to exclude (e.g. node_modules, target/).
    exclude_patterns: Vec<glob::Pattern>,
}

impl ReadOnlyToolSet {
    pub fn new(root: PathBuf) -> Self {
        let default_excludes = [
            "node_modules/**",
            "target/**",
            ".git/**",
            "*.lock",
            "*.bin",
            "*.exe",
            "*.so",
            "*.dylib",
            "*.wasm",
        ];

        let exclude_patterns = default_excludes
            .iter()
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();

        Self {
            root,
            max_file_size: 1024 * 1024, // 1 MB
            exclude_patterns,
        }
    }

    /// Validate a path is within the allowed root and not excluded.
    fn validate_path(&self, path: &Path) -> Result<PathBuf> {
        let canonical = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        // Ensure the path doesn't escape the root via symlinks or ../
        let resolved = canonical
            .canonicalize()
            .with_context(|| format!("Path does not exist: {}", canonical.display()))?;

        if !resolved.starts_with(&self.root) {
            anyhow::bail!(
                "Path {} escapes the project root {}",
                resolved.display(),
                self.root.display()
            );
        }

        let relative = resolved.strip_prefix(&self.root).unwrap_or(&resolved);

        for pattern in &self.exclude_patterns {
            if pattern.matches_path(relative) {
                anyhow::bail!(
                    "Path {} matches exclude pattern {}",
                    path.display(),
                    pattern
                );
            }
        }

        Ok(resolved)
    }

    /// Read a file's contents as a string.
    #[instrument(skip(self), fields(path = %path.as_ref().display()))]
    pub async fn read_file(&self, path: impl AsRef<Path> + std::fmt::Debug) -> Result<String> {
        let validated = self.validate_path(path.as_ref())?;

        let metadata = fs::metadata(&validated).await?;
        if metadata.len() > self.max_file_size {
            anyhow::bail!(
                "File {} exceeds maximum size ({} > {} bytes)",
                validated.display(),
                metadata.len(),
                self.max_file_size
            );
        }

        let content = fs::read_to_string(&validated)
            .await
            .with_context(|| format!("Failed to read file: {}", validated.display()))?;

        Ok(content)
    }

    /// Read only the first N lines of a file.
    #[instrument(skip(self))]
    pub async fn read_file_head(
        &self,
        path: impl AsRef<Path> + std::fmt::Debug,
        lines: usize,
    ) -> Result<String> {
        let content = self.read_file(path).await?;
        let head: String = content.lines().take(lines).collect::<Vec<_>>().join("\n");
        Ok(head)
    }

    /// Search for a pattern in files using grep-like semantics.
    #[instrument(skip(self))]
    pub async fn grep(
        &self,
        pattern: &str,
        path: impl AsRef<Path> + std::fmt::Debug,
        max_results: usize,
    ) -> Result<Vec<GrepResult>> {
        let validated = self.validate_path(path.as_ref())?;
        let regex = regex::Regex::new(pattern)
            .with_context(|| format!("Invalid regex pattern: {pattern}"))?;

        let mut results = Vec::new();

        if validated.is_file() {
            self.grep_file(&validated, &regex, &mut results, max_results)
                .await?;
        } else if validated.is_dir() {
            self.grep_directory(&validated, &regex, &mut results, max_results)
                .await?;
        }

        Ok(results)
    }

    async fn grep_file(
        &self,
        path: &Path,
        regex: &regex::Regex,
        results: &mut Vec<GrepResult>,
        max_results: usize,
    ) -> Result<()> {
        if results.len() >= max_results {
            return Ok(());
        }

        let content = match fs::read_to_string(path).await {
            Ok(c) => c,
            Err(_) => return Ok(()), // Skip binary / unreadable files
        };

        for (line_no, line) in content.lines().enumerate() {
            if regex.is_match(line) {
                results.push(GrepResult {
                    file: path.strip_prefix(&self.root).unwrap_or(path).to_path_buf(),
                    line_number: line_no + 1,
                    content: line.to_string(),
                });
                if results.len() >= max_results {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn grep_directory(
        &self,
        dir: &Path,
        regex: &regex::Regex,
        results: &mut Vec<GrepResult>,
        max_results: usize,
    ) -> Result<()> {
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();
            let relative = path.strip_prefix(&self.root).unwrap_or(&path);

            // Check excludes
            let excluded = self
                .exclude_patterns
                .iter()
                .any(|p: &glob::Pattern| p.matches_path(relative));
            if excluded {
                continue;
            }

            let ft = entry.file_type().await?;
            if ft.is_file() {
                self.grep_file(&path, regex, results, max_results).await?;
            } else if ft.is_dir() {
                Box::pin(self.grep_directory(&path, regex, results, max_results)).await?;
            }
        }

        Ok(())
    }

    /// Glob for files matching a pattern.
    #[instrument(skip(self))]
    pub async fn glob_files(&self, pattern: &str) -> Result<Vec<PathBuf>> {
        let full_pattern = format!("{}/{}", self.root.display(), pattern);
        let paths: Vec<PathBuf> = glob::glob(&full_pattern)?
            .filter_map(|entry: Result<PathBuf, glob::GlobError>| entry.ok())
            .filter(|path: &PathBuf| {
                let relative = path.strip_prefix(&self.root).unwrap_or(path);
                !self
                    .exclude_patterns
                    .iter()
                    .any(|p: &glob::Pattern| p.matches_path(relative))
            })
            .map(|path: PathBuf| path.strip_prefix(&self.root).unwrap_or(&path).to_path_buf())
            .collect();

        Ok(paths)
    }

    /// List the directory tree up to a given depth.
    #[instrument(skip(self))]
    pub async fn list_tree(
        &self,
        path: impl AsRef<Path> + std::fmt::Debug,
        max_depth: usize,
    ) -> Result<Vec<TreeEntry>> {
        let validated = self.validate_path(path.as_ref())?;
        let mut entries = Vec::new();
        self.walk_tree(&validated, 0, max_depth, &mut entries)
            .await?;
        Ok(entries)
    }

    #[async_recursion::async_recursion]
    async fn walk_tree(
        &self,
        dir: &Path,
        depth: usize,
        max_depth: usize,
        entries: &mut Vec<TreeEntry>,
    ) -> Result<()> {
        if depth > max_depth {
            return Ok(());
        }

        let mut read_dir = fs::read_dir(dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            let relative = path.strip_prefix(&self.root).unwrap_or(&path);

            let excluded = self
                .exclude_patterns
                .iter()
                .any(|p: &glob::Pattern| p.matches_path(relative));
            if excluded {
                continue;
            }

            let ft = entry.file_type().await?;
            let metadata = entry.metadata().await?;

            entries.push(TreeEntry {
                path: relative.to_path_buf(),
                is_dir: ft.is_dir(),
                size: metadata.len(),
                depth,
            });

            if ft.is_dir() {
                self.walk_tree(&path, depth + 1, max_depth, entries).await?;
            }
        }

        Ok(())
    }

    /// Analyze imports/exports in a Rust file to build dependency information.
    #[instrument(skip(self))]
    pub async fn analyze_rust_deps(
        &self,
        path: impl AsRef<Path> + std::fmt::Debug,
    ) -> Result<FileDependencies> {
        let content = self.read_file(path.as_ref()).await?;

        let use_regex = regex::Regex::new(r"use\s+([\w:]+(?:::\{[^}]+\})?)")?;
        let mod_regex = regex::Regex::new(r"mod\s+(\w+)\s*[;{]")?;
        let pub_fn_regex = regex::Regex::new(r"pub\s+(?:async\s+)?fn\s+(\w+)")?;
        let pub_struct_regex = regex::Regex::new(r"pub\s+struct\s+(\w+)")?;
        let pub_trait_regex = regex::Regex::new(r"pub\s+trait\s+(\w+)")?;
        let impl_regex = regex::Regex::new(r"impl(?:<[^>]+>)?\s+(\w+)")?;

        let imports: Vec<String> = use_regex
            .captures_iter(&content)
            .filter_map(|c: regex::Captures| c.get(1).map(|m: regex::Match| m.as_str().to_string()))
            .collect();

        let modules: Vec<String> = mod_regex
            .captures_iter(&content)
            .filter_map(|c: regex::Captures| c.get(1).map(|m: regex::Match| m.as_str().to_string()))
            .collect();

        let pub_functions: Vec<String> = pub_fn_regex
            .captures_iter(&content)
            .filter_map(|c: regex::Captures| c.get(1).map(|m: regex::Match| m.as_str().to_string()))
            .collect();

        let pub_structs: Vec<String> = pub_struct_regex
            .captures_iter(&content)
            .filter_map(|c: regex::Captures| c.get(1).map(|m: regex::Match| m.as_str().to_string()))
            .collect();

        let pub_traits: Vec<String> = pub_trait_regex
            .captures_iter(&content)
            .filter_map(|c: regex::Captures| c.get(1).map(|m: regex::Match| m.as_str().to_string()))
            .collect();

        let impls: Vec<String> = impl_regex
            .captures_iter(&content)
            .filter_map(|c: regex::Captures| c.get(1).map(|m: regex::Match| m.as_str().to_string()))
            .collect();

        let loc = content.lines().count() as u32;

        Ok(FileDependencies {
            path: path.as_ref().to_path_buf(),
            imports,
            modules,
            pub_functions,
            pub_structs,
            pub_traits,
            impls,
            loc,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResult {
    pub file: PathBuf,
    pub line_number: usize,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDependencies {
    pub path: PathBuf,
    pub imports: Vec<String>,
    pub modules: Vec<String>,
    pub pub_functions: Vec<String>,
    pub pub_structs: Vec<String>,
    pub pub_traits: Vec<String>,
    pub impls: Vec<String>,
    pub loc: u32,
}

use serde::{Deserialize, Serialize};
