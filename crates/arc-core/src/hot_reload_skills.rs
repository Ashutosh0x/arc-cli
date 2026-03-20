//! Hot-Reload Skills — fsnotify watcher on skill directories.
//! New or updated skills become available immediately without restarting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub last_modified: std::time::SystemTime,
}

pub struct HotReloadSkillRegistry {
    skills: Arc<RwLock<HashMap<String, SkillEntry>>>,
    watch_dirs: Vec<PathBuf>,
}

impl HotReloadSkillRegistry {
    pub fn new(watch_dirs: Vec<PathBuf>) -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            watch_dirs,
        }
    }

    /// Initial scan of all skill directories
    pub fn initial_scan(&self) -> usize {
        let mut skills = self.skills.write().unwrap_or_else(|e| e.into_inner());
        let mut count = 0;
        for dir in &self.watch_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.join("SKILL.md").exists() {
                        if let Some(skill) = Self::parse_skill(&path) {
                            skills.insert(skill.name.clone(), skill);
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }

    /// Parse a single skill from its directory
    fn parse_skill(skill_dir: &Path) -> Option<SkillEntry> {
        let skill_md = skill_dir.join("SKILL.md");
        let content = std::fs::read_to_string(&skill_md).ok()?;
        let modified = std::fs::metadata(&skill_md).ok()?.modified().ok()?;

        // Parse YAML frontmatter
        let mut name = skill_dir.file_name()?.to_string_lossy().to_string();
        let mut description = String::new();

        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let frontmatter = &content[3..3 + end];
                for line in frontmatter.lines() {
                    let line = line.trim();
                    if let Some(n) = line.strip_prefix("name:") {
                        name = n.trim().trim_matches('"').to_string();
                    } else if let Some(d) = line.strip_prefix("description:") {
                        description = d.trim().trim_matches('"').to_string();
                    }
                }
            }
        }

        Some(SkillEntry {
            name,
            description,
            path: skill_dir.to_path_buf(),
            last_modified: modified,
        })
    }

    /// Handle a file system change event
    pub fn on_change(&self, changed_path: &Path) {
        // Find the skill directory containing this path
        let skill_dir = if changed_path.is_dir() {
            changed_path.to_path_buf()
        } else {
            changed_path.parent().unwrap_or(changed_path).to_path_buf()
        };

        if skill_dir.join("SKILL.md").exists() {
            let mut skills = self.skills.write().unwrap_or_else(|e| e.into_inner());
            if let Some(skill) = Self::parse_skill(&skill_dir) {
                tracing::info!("Hot-reloaded skill: {}", skill.name);
                skills.insert(skill.name.clone(), skill);
            }
        }
    }

    /// Handle a file deletion event
    pub fn on_delete(&self, deleted_path: &Path) {
        let mut skills = self.skills.write().unwrap_or_else(|e| e.into_inner());
        let dir_name = deleted_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string());
        if let Some(name) = dir_name {
            if skills.remove(&name).is_some() {
                tracing::info!("Unloaded skill: {}", name);
            }
        }
    }

    /// Get all currently loaded skills
    pub fn list(&self) -> Vec<SkillEntry> {
        let skills = self.skills.read().unwrap_or_else(|e| e.into_inner());
        skills.values().cloned().collect()
    }

    /// Get a specific skill by name
    pub fn get(&self, name: &str) -> Option<SkillEntry> {
        let skills = self.skills.read().unwrap_or_else(|e| e.into_inner());
        skills.get(name).cloned()
    }

    pub fn count(&self) -> usize {
        let skills = self.skills.read().unwrap_or_else(|e| e.into_inner());
        skills.len()
    }
}
