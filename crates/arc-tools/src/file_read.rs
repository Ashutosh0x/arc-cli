use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use tokio::fs;

use crate::traits::Tool;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &'static str {
        "file_read"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file from the local filesystem. Pass absolute or relative paths."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, anyhow::Error> {
        let path_str = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required string argument 'path'"))?;

        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(format!("Error: File not found at '{path_str}'"));
        }

        if path.is_dir() {
            return Ok(format!("Error: Path '{path_str}' is a directory, not a file"));
        }

        match fs::read_to_string(path).await {
            Ok(content) => {
                // Add line numbers for context chunking later
                let formatted: String = content
                    .lines()
                    .enumerate()
                    .map(|(i, line)| format!("{:4} | {}\n", i + 1, line))
                    .collect();
                Ok(formatted)
            }
            Err(e) => Ok(format!("Error reading file: {e}")),
        }
    }
}
