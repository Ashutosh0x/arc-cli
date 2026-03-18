use crate::events::HookEvent;
use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct HookConfig {
    pub enabled_events: Vec<HookEvent>,
    pub run_format: bool,
    pub run_lint: bool,
    pub auto_doc: bool,
}

pub struct HookExecutor {
    workspace_root: PathBuf,
    config: HookConfig,
}

impl HookExecutor {
    pub fn new(workspace_root: PathBuf, config: HookConfig) -> Self {
        Self {
            workspace_root,
            config,
        }
    }

    /// Install ARC hooks into the local .git/hooks directory.
    pub fn install_git_hooks(&self) -> Result<()> {
        let git_dir = self.workspace_root.join(".git").join("hooks");
        if !git_dir.exists() {
            anyhow::bail!("No .git/hooks directory found in workspace root");
        }

        // Create pre-commit hook template
        let pre_commit_path = git_dir.join("pre-commit");
        let pre_commit_script = r#"#!/bin/sh
# ARC CLI Managed Pre-Commit Hook
arc run-hook pre-commit
"#;
        std::fs::write(&pre_commit_path, pre_commit_script)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&pre_commit_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&pre_commit_path, perms)?;
        }

        info!("Successfully installed ARC git hooks into {:?}", git_dir);
        Ok(())
    }

    /// Run the logic for a specific hook event.
    pub async fn execute(&self, event: HookEvent) -> Result<()> {
        if !self.config.enabled_events.contains(&event) {
            return Ok(());
        }

        match event {
            HookEvent::PreCommit => self.run_pre_commit().await,
            HookEvent::PostCommit => {
                info!("Post-commit hook triggered. Generating summary...");
                // In full implementation, this calls the LLM to write a summary
                Ok(())
            }
            HookEvent::OnSave { ref file } => {
                info!("File saved: {}. Running fast format/lint.", file);
                Ok(())
            }
        }
    }

    async fn run_pre_commit(&self) -> Result<()> {
        info!("Running pre-commit checks...");

        if self.config.run_format {
            let status = Command::new("cargo")
                .arg("fmt")
                .arg("--check")
                .current_dir(&self.workspace_root)
                .status()?;
            
            if !status.success() {
                warn!("Cargo fmt failed, rejecting commit.");
                std::process::exit(1);
            }
        }

        if self.config.run_lint {
            let status = Command::new("cargo")
                .arg("clippy")
                .arg("--")
                .arg("-D")
                .arg("warnings")
                .current_dir(&self.workspace_root)
                .status()?;

            if !status.success() {
                warn!("Cargo clippy failed, rejecting commit.");
                std::process::exit(1);
            }
        }

        info!("Pre-commit checks passed.");
        Ok(())
    }
}
