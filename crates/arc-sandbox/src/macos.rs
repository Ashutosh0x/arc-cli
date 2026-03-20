use anyhow::Result;
use std::path::{Path, PathBuf};

/// macOS Sandbox leveraging native `sandbox-exec` profiles or software bounds.
/// For ARC CLI, we track allowed paths similarly to Windows and inject them
/// dynamically into spawned processes configuring the macOS kernel layer.
pub struct MacosSandbox {
    allowed_paths: Vec<PathBuf>,
}

impl MacosSandbox {
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
        tracing::info!("macOS Sandbox boundaries registered locally.");
        Ok(())
    }

    /// Future implementation: Generate a dynamic `.sb` profile string from `allowed_paths`
    /// and invoke `sandbox-exec -p <profile_string> <command>` when spawning processes.
    pub fn generate_sandbox_profile(&self) -> String {
        let mut profile =
            String::from("(version 1)\n(deny default)\n(allow file-read* file-write*\n");
        for path in &self.allowed_paths {
            profile.push_str(&format!("    (subpath \"{}\")\n", path.display()));
        }
        profile.push_str(")\n(allow network*)\n(allow process-exec*)");
        profile
    }
}
