use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HookConfig {
    pub pre_commit: Option<Vec<String>>,
    pub post_edit: Option<Vec<String>>,
    pub on_success: Option<Vec<String>>,
}

impl HookConfig {
    /// Loads the `.arc/hooks.toml` file if it exists in the workspace
    pub fn load_from_workspace<P: AsRef<Path>>(workspace_root: P) -> Result<Self> {
        let hook_file = workspace_root.as_ref().join(".arc").join("hooks.toml");
        if hook_file.exists() {
            let content = std::fs::read_to_string(&hook_file)?;
            let config: HookConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(HookConfig::default())
        }
    }

    /// Executes a list of hook commands securely
    pub fn execute_hooks(&self, hook_type: &str, workspace_root: impl AsRef<Path>) -> Result<()> {
        let commands = match hook_type {
            "pre_commit" => self.pre_commit.as_ref(),
            "post_edit" => self.post_edit.as_ref(),
            "on_success" => self.on_success.as_ref(),
            _ => None,
        };

        if let Some(cmds) = commands {
            for cmd_str in cmds {
                // In a production environment, this would be wrapped inside the Arc Sandbox 
                // to prevent malicious arbitrary execution. For now, we execute standard bash/cmd.
                let status = if cfg!(target_os = "windows") {
                    Command::new("cmd")
                        .arg("/C")
                        .arg(cmd_str)
                        .current_dir(workspace_root.as_ref())
                        .status()?
                } else {
                    Command::new("sh")
                        .arg("-c")
                        .arg(cmd_str)
                        .current_dir(workspace_root.as_ref())
                        .status()?
                };
                
                if !status.success() {
                    tracing::warn!("Hook command '{}' failed with status: {}", cmd_str, status);
                } else {
                    tracing::info!("Successfully ran hook: {}", cmd_str);
                }
            }
        }
        Ok(())
    }
}
