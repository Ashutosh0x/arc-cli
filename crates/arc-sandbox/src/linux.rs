// SPDX-License-Identifier: MIT
use anyhow::{Context, Result};
use landlock::{
    ABI, Access, AccessFs, PathBeneath, PathFd, RestrictionStatus, Ruleset, RulesetAttr,
    RulesetCreatedAttr,
};
use std::path::PathBuf;
use tracing::info;

pub struct LinuxSandbox {
    allowed_paths: Vec<PathBuf>,
}

impl LinuxSandbox {
    pub fn new() -> Self {
        Self {
            allowed_paths: Vec::new(),
        }
    }

    pub fn apply(&self, paths: &[PathBuf]) -> Result<()> {
        info!(
            "Applying Linux Landlock sandbox to {} paths",
            paths.len() + self.allowed_paths.len()
        );

        let abi = ABI::V3;
        let access = AccessFs::from_all(abi);
        let mut ruleset = Ruleset::default().handle_access(access)?.create()?;

        for path in paths.iter().chain(self.allowed_paths.iter()) {
            if path.exists() {
                let path_fd = PathFd::new(path)
                    .with_context(|| format!("failed to open path for landlock: {path:?}"))?;
                let rule = PathBeneath::new(path_fd, access);
                ruleset = ruleset.add_rule(rule)?;
            }
        }

        let status: RestrictionStatus = ruleset.restrict_self()?;
        if matches!(status.ruleset, landlock::RulesetStatus::NotEnforced) {
            tracing::warn!("Landlock is not supported by the current kernel");
        }

        Ok(())
    }
}

impl Default for LinuxSandbox {
    fn default() -> Self {
        Self::new()
    }
}
