use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use tokio::fs;

use crate::traits::Tool;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &'static str {
        "file_edit"
    }

    fn description(&self) -> &'static str {
        "Edit a file by replacing a specific block of text. For Phase 1, it expects exact string matching search/replace."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to edit"
                },
                "search": {
                    "type": "string",
                    "description": "The exact multi-line string to find in the file"
                },
                "replace": {
                    "type": "string",
                    "description": "The exact replacement string"
                }
            },
            "required": ["path", "search", "replace"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, anyhow::Error> {
        let path_str = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let search = args.get("search").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("Missing 'search'"))?;
        let replace = args.get("replace").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("Missing 'replace'"))?;

        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(format!("Error: File not found at '{path_str}'"));
        }

        let content = fs::read_to_string(path).await?;
        
        // AST-aware diffing and formatting would go here via tree-sitter.
        // For MVP Phase 1 (and robust edit reliability), we just run string replacement.
        if !content.contains(search) {
            return Ok("Error: Search block not found in file. Ensure exact whitespace and line-endings match.".into());
        }

        let new_content = content.replace(search, replace);
        fs::write(path, new_content).await?;

        Ok(format!("Successfully edited file '{path_str}'"))
    }
}
