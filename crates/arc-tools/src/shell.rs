// SPDX-License-Identifier: MIT
use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;

use crate::traits::Tool;

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command. Use this to run tests, examine standard output, or check system states."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, anyhow::Error> {
        let command_str = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required string argument 'command'"))?;

        // For cross-platform MVP execution, use sh on Unix, cmd on Windows
        // Since user is on Windows, we'll use cmd.exe /c
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg(command_str)
                .output()
                .await?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command_str)
                .output()
                .await?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str("STDOUT:\n");
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n");
            }
            result.push_str("STDERR:\n");
            result.push_str(&stderr);
        }

        if result.is_empty() {
            result.push_str(&format!(
                "Command completed successfully with code {}",
                output.status
            ));
        }

        Ok(result)
    }
}
