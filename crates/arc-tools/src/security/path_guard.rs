use anyhow::Result;
use std::path::{Path, PathBuf};

/// Path traversal prevention and file system security
pub struct PathGuard {
    /// The project root directory (all operations scoped to this)
    project_root: PathBuf,
    /// Additional allowed directories
    extra_allowed: Vec<PathBuf>,
    /// File extensions the agent may write
    writable_extensions: Vec<String>,
    /// Maximum file size for writes
    max_file_size: u64,
}

impl PathGuard {
    pub fn new(project_root: &Path) -> Result<Self> {
        let canonical_root = project_root
            .canonicalize()
            .map_err(|_| anyhow::anyhow!("Cannot resolve project root"))?;

        Ok(Self {
            project_root: canonical_root,
            extra_allowed: vec![std::env::temp_dir()],
            writable_extensions: vec![
                "rs", "py", "js", "ts", "jsx", "tsx", "go", "java",
                "c", "cpp", "h", "hpp", "md", "txt", "toml", "yaml",
                "yml", "json", "html", "css", "sh", "bash", "zsh",
                "sql", "graphql", "proto", "dockerfile",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            max_file_size: 10 * 1024 * 1024, // 10MB
        })
    }

    /// Validate a path for reading — prevents path traversal
    pub fn validate_read(&self, path: &Path) -> Result<PathBuf> {
        let resolved = self.resolve_path(path)?;

        // Block symlink attacks: resolve the final target
        if resolved.is_symlink() {
            let target = std::fs::read_link(&resolved)?;
            let target_resolved = self.resolve_path(&target)?;
            self.check_within_bounds(&target_resolved)?;
            return Ok(target_resolved);
        }

        self.check_within_bounds(&resolved)?;
        Ok(resolved)
    }

    /// Validate a path for writing — additional checks
    pub fn validate_write(&self, path: &Path, content_size: u64) -> Result<PathBuf> {
        let resolved = self.resolve_path(path)?;
        self.check_within_bounds(&resolved)?;

        // Size check
        if content_size > self.max_file_size {
            anyhow::bail!(
                "Write size ({} bytes) exceeds maximum ({} bytes)",
                content_size,
                self.max_file_size
            );
        }

        // Extension check
        if let Some(ext) = resolved.extension().and_then(|e| e.to_str()) {
            if !self.writable_extensions.contains(&ext.to_lowercase()) {
                anyhow::bail!(
                    "Agent cannot write files with .{ext} extension. \
                     Allowed: {:?}",
                    self.writable_extensions
                );
            }
        }

        // Block writes to dotfiles/hidden directories
        for component in resolved.components() {
            if let std::path::Component::Normal(c) = component {
                let name = c.to_string_lossy();
                if name.starts_with('.') && name != "." && name != ".." {
                    // Allow .gitignore, .env.example but block .git/*, .ssh/*
                    let blocked_dirs = [".git", ".ssh", ".gnupg", ".aws", ".arc"];
                    if blocked_dirs.iter().any(|d| name.as_ref() == *d) {
                        anyhow::bail!(
                            "Cannot write to protected directory: {}",
                            name
                        );
                    }
                }
            }
        }

        Ok(resolved)
    }

    // ── Private helpers ──────────────────────────────────────

    fn resolve_path(&self, path: &Path) -> Result<PathBuf> {
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(self.project_root.join(path))
        }
    }

    fn check_within_bounds(&self, path: &Path) -> Result<()> {
        // Canonicalize for comparison (resolves .., symlinks, etc.)
        // For new files, check parent directory
        let check_path = if path.exists() {
            path.canonicalize()?
        } else {
            path.parent()
                .unwrap_or(path)
                .canonicalize()
                .unwrap_or_else(|_| path.to_path_buf())
        };

        let in_bounds = check_path.starts_with(&self.project_root)
            || self
                .extra_allowed
                .iter()
                .any(|a| check_path.starts_with(a));

        if !in_bounds {
            anyhow::bail!(
                "Path traversal blocked: {} is outside project root ({})",
                path.display(),
                self.project_root.display()
            );
        }

        Ok(())
    }
}
