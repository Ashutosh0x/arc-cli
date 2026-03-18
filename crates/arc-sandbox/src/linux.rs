use anyhow::{Result, Context};
use landlock::{
    Access, AccessFs, ABI, Ruleset, RulesetAttr, RulesetError, RulesetStatus,
    PathBeneath,
};
use std::path::{Path, PathBuf};
use tracing::info;

pub struct LinuxSandbox {
    allowed_paths: Vec<PathBuf>,
}

impl LinuxSandbox {
    pub fn new() -> Self {
        Self { allowed_paths: Vec::new() }
    }

    pub fn apply(&self, paths: &[PathBuf]) -> Result<()> {
        info!("Applying Linux Landlock sandbox to {} paths", paths.len());
        
        let abi = ABI::V1;
        let mut ruleset = Ruleset::new()
            .handle_access(AccessFs::from_all(abi))?
            .create()?;

        // Grant read/write access to the allowed paths
        let access = AccessFs::from_all(abi);
        for path in paths {
            if path.exists() {
                let path_beneath = PathBeneath::new(path, access);
                ruleset = ruleset.add_rule(path_beneath)?;
            }
        }

        // Restrict the current process
        let status: RulesetStatus = ruleset.restrict_self()?;
        if status.ruleset == landlock::RulesetStatus::NotEnforced {
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
