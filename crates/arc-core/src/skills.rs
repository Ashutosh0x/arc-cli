//! # Skills System — .arc/skills/*.md with Auto-Discovery
//!
//! SKILL.md frontmatter: name, description. ${ARC_SKILL_DIR} variable.
//! Auto-discovery in subdirectories. Skill deduplication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub instructions: String,
    pub directory: PathBuf,
    pub file_path: PathBuf,
}

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Discover skills recursively from a directory.
    pub fn discover(&mut self, dir: &Path) -> Result<usize, String> {
        if !dir.exists() {
            return Ok(0);
        }
        let mut count = 0;
        self.discover_recursive(dir, &mut count)?;
        Ok(count)
    }

    fn discover_recursive(&mut self, dir: &Path, count: &mut usize) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check for SKILL.md in this subdirectory.
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    if let Ok(skill) = Self::parse_skill(&skill_md, &path) {
                        // Deduplicate by name.
                        if !self.skills.contains_key(&skill.name) {
                            self.skills.insert(skill.name.clone(), skill);
                            *count += 1;
                        }
                    }
                }
                // Recurse into subdirs.
                self.discover_recursive(&path, count)?;
            } else if path.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
                if let Ok(skill) = Self::parse_skill(&path, dir) {
                    if !self.skills.contains_key(&skill.name) {
                        self.skills.insert(skill.name.clone(), skill);
                        *count += 1;
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_skill(path: &Path, directory: &Path) -> Result<Skill, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err("Missing YAML frontmatter".into());
        }

        #[derive(Deserialize)]
        struct FM {
            #[serde(default)]
            name: String,
            #[serde(default)]
            description: String,
        }
        let fm: FM = serde_yaml::from_str(parts[1].trim()).map_err(|e| e.to_string())?;
        let name = if fm.name.is_empty() {
            directory
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("skill")
                .to_string()
        } else {
            fm.name
        };

        Ok(Skill {
            name,
            description: fm.description,
            instructions: parts[2].trim().to_string(),
            directory: directory.to_path_buf(),
            file_path: path.to_path_buf(),
        })
    }

    /// Resolve ${ARC_SKILL_DIR} in instructions text.
    pub fn resolve_variables(&self, skill_name: &str, text: &str) -> String {
        if let Some(skill) = self.skills.get(skill_name) {
            text.replace(
                "${ARC_SKILL_DIR}",
                skill.directory.to_str().unwrap_or_default(),
            )
        } else {
            text.to_string()
        }
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
