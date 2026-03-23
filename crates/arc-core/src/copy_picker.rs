// SPDX-License-Identifier: MIT
//! # /copy Picker — Interactive Code Block Selection
//!
//! Pick code blocks from responses, copy full response, write to file.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub index: usize,
    pub language: String,
    pub content: String,
    pub line_count: usize,
}

/// Extract code blocks from a markdown response.
pub fn extract_code_blocks(response: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lang = String::new();
    let mut current_content = String::new();
    let mut idx = 0;

    for line in response.lines() {
        if line.starts_with("```") && !in_block {
            in_block = true;
            current_lang = line.trim_start_matches('`').trim().to_string();
            current_content.clear();
        } else if line.starts_with("```") && in_block {
            in_block = false;
            let content = current_content.trim_end().to_string();
            let line_count = content.lines().count();
            blocks.push(CodeBlock {
                index: idx,
                language: current_lang.clone(),
                content,
                line_count,
            });
            idx += 1;
        } else if in_block {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }
    blocks
}

#[derive(Debug, Clone, Copy)]
pub enum CopyAction {
    CopyBlock(usize),
    CopyAll,
    WriteToFile(usize),
}

/// Format a picker display of available code blocks.
pub fn format_picker(blocks: &[CodeBlock]) -> String {
    let mut out = String::from("Code blocks available:\n");
    for block in blocks {
        out.push_str(&format!(
            "  [{}] {} ({} lines)\n",
            block.index,
            if block.language.is_empty() {
                "plain"
            } else {
                &block.language
            },
            block.line_count
        ));
    }
    out.push_str("  [a] Copy entire response\n");
    out.push_str("  [w<N>] Write block N to file\n");
    out
}

/// Parse user input into a copy action.
pub fn parse_copy_input(input: &str, max_idx: usize) -> Option<CopyAction> {
    let input = input.trim();
    if input == "a" {
        return Some(CopyAction::CopyAll);
    }
    if let Some(n) = input.strip_prefix('w') {
        if let Ok(idx) = n.parse::<usize>() {
            if idx <= max_idx {
                return Some(CopyAction::WriteToFile(idx));
            }
        }
    }
    if let Ok(idx) = input.parse::<usize>() {
        if idx <= max_idx {
            return Some(CopyAction::CopyBlock(idx));
        }
    }
    None
}
