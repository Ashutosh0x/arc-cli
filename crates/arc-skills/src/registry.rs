use crate::skill::Skill;
use dashmap::DashMap;
use std::sync::Arc;
use anyhow::{anyhow, Result};

#[derive(Default)]
pub struct SkillRegistry {
    skills: DashMap<String, Arc<dyn Skill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, skill: Arc<dyn Skill>) {
        self.skills.insert(skill.name().to_string(), skill);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.get(name).map(|s| s.clone())
    }

    pub fn list(&self) -> Vec<Arc<dyn Skill>> {
        self.skills.iter().map(|s| s.clone()).collect()
    }

    pub fn unregister(&self, name: &str) -> Result<()> {
        self.skills.remove(name).map(|_| ()).ok_or_else(|| anyhow!("Skill not found"))
    }
}
