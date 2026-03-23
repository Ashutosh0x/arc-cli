// SPDX-License-Identifier: MIT
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

pub struct SparseCheckout {
    repo_path: PathBuf,
}

impl SparseCheckout {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
        }
    }

    pub fn init(&self) -> Result<()> {
        info!("Initializing sparse-checkout in {:?}", self.repo_path);
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(["sparse-checkout", "init", "--cone"])
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to init sparse checkout: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    pub fn set_paths(&self, paths: &[&str]) -> Result<()> {
        info!("Setting sparse-checkout paths: {:?}", paths);
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.repo_path)
            .args(["sparse-checkout", "set"]);
        for p in paths {
            cmd.arg(p);
        }

        let output = cmd.output()?;
        if !output.status.success() {
            anyhow::bail!(
                "Failed to set sparse paths: {:?}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }
}
