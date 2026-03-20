//! # Slash Command System — .md Commands with YAML Frontmatter
//!
//! Plugin-shipped `.md` commands with description, argument-hint,
//! allowed-tools, effort frontmatter. Dynamic args via $ARGUMENTS.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub argument_hint: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub effort: Option<String>,
    pub body: String,
    pub source: CommandSource,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandSource { Builtin, Plugin(String), User }

pub struct SlashCommandRegistry {
    commands: HashMap<String, SlashCommand>,
}

impl SlashCommandRegistry {
    pub fn new() -> Self { Self { commands: HashMap::new() } }

    pub fn register(&mut self, cmd: SlashCommand) { self.commands.insert(cmd.name.clone(), cmd); }

    pub fn discover(&mut self, dir: &Path) -> Result<usize, String> {
        if !dir.exists() { return Ok(0); }
        let mut count = 0;
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(cmd) = Self::parse_command(&path) { self.register(cmd); count += 1; }
            }
        }
        Ok(count)
    }

    fn parse_command(path: &Path) -> Result<SlashCommand, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 { return Err("Missing YAML frontmatter".into()); }

        #[derive(Deserialize)]
        struct Frontmatter {
            #[serde(default)] description: String,
            #[serde(default, rename = "argument-hint")] argument_hint: String,
            #[serde(default, rename = "allowed-tools")] allowed_tools: Vec<String>,
            #[serde(default)] effort: Option<String>,
        }

        let fm: Frontmatter = serde_yaml::from_str(parts[1].trim()).map_err(|e| e.to_string())?;
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        Ok(SlashCommand { name, description: fm.description, argument_hint: fm.argument_hint, allowed_tools: fm.allowed_tools, effort: fm.effort, body: parts[2].trim().to_string(), source: CommandSource::User, file_path: path.to_path_buf() })
    }

    pub fn execute(&self, name: &str, args: &str) -> Option<String> {
        self.commands.get(name).map(|cmd| cmd.body.replace("$ARGUMENTS", args))
    }

    pub fn list(&self) -> Vec<&SlashCommand> { self.commands.values().collect() }
    pub fn get(&self, name: &str) -> Option<&SlashCommand> { self.commands.get(name) }
}

impl Default for SlashCommandRegistry { fn default() -> Self { Self::new() } }
