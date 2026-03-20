//! # Statusline Scripts — Custom Status Bar
//!
//! Rate limits, workspace info, model effort level, worktree.
//! Customizable via settings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatuslineConfig {
    pub enabled: bool,
    #[serde(default)]
    pub show_rate_limits: bool,
    #[serde(default)]
    pub show_effort_level: bool,
    #[serde(default)]
    pub show_model: bool,
    #[serde(default)]
    pub show_worktree: bool,
    #[serde(default)]
    pub show_context_usage: bool,
    #[serde(default)]
    pub custom_scripts: Vec<StatuslineScript>,
}

impl Default for StatuslineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_rate_limits: true,
            show_effort_level: true,
            show_model: true,
            show_worktree: true,
            show_context_usage: true,
            custom_scripts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatuslineScript {
    pub name: String,
    pub command: String,
    pub position: StatuslinePosition,
    pub refresh_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatuslinePosition {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Default)]
pub struct StatuslineData {
    pub model_name: String,
    pub effort_symbol: String,
    pub context_percent: f64,
    pub rate_limit_remaining: Option<u32>,
    pub worktree_name: Option<String>,
    pub workspace_dir: String,
    pub custom_segments: Vec<(StatuslinePosition, String)>,
}

impl StatuslineData {
    pub fn render(&self, config: &StatuslineConfig) -> String {
        let mut parts = Vec::new();
        if config.show_model && !self.model_name.is_empty() {
            parts.push(format!("⚙ {}", self.model_name));
        }
        if config.show_effort_level && !self.effort_symbol.is_empty() {
            parts.push(self.effort_symbol.clone());
        }
        if config.show_context_usage {
            parts.push(format!("ctx:{:.0}%", self.context_percent));
        }
        if config.show_rate_limits {
            if let Some(remaining) = self.rate_limit_remaining {
                parts.push(format!("rl:{remaining}"));
            }
        }
        if config.show_worktree {
            if let Some(ref wt) = self.worktree_name {
                parts.push(format!("wt:{wt}"));
            }
        }
        parts.join(" │ ")
    }
}
