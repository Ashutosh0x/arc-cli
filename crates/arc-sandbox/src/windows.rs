use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

/// A software-level directory boundary sandbox for Windows.
/// Unlike Linux Landlock, this does not hook into the NT Kernel natively
/// for this CLI, but rather enforces strict path validation logic
/// prior to command execution to prevent directory traversal escapes.
pub struct WindowsSandbox {
    allowed_paths: Vec<PathBuf>,
}

impl WindowsSandbox {
    pub fn new() -> Self {
        Self {
            allowed_paths: Vec::new(),
        }
    }

    pub fn apply(&mut self, paths: &[PathBuf]) -> Result<()> {
        let mut allowed = Vec::new();
        for p in paths {
            if let Ok(canon) = p.canonicalize() {
                allowed.push(canon);
            } else {
                allowed.push(p.clone());
            }
        }

        self.allowed_paths = allowed;
        tracing::warn!(
            "Windows Kernel Sandboxing is unavailable on this OS wrapper. Utilizing software-level strict directory boundaries."
        );
        Ok(())
    }

    /// Verifies if a given target path is strictly inside the allowed workspace path.
    pub fn check_access<P: AsRef<Path>>(&self, target: P) -> Result<()> {
        let target_path = target
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| target.as_ref().to_path_buf());

        for allowed in &self.allowed_paths {
            if target_path.starts_with(allowed) {
                return Ok(());
            }
        }

        Err(anyhow!(
            "Sandbox Violation: Agent attempted to mutate path {:?} outside of allowed boundaries",
            target.as_ref()
        ))
    }
}
