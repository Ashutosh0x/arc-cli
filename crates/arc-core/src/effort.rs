//! # Effort Levels — Low/Medium/High + /effort Command
//!
//! Per-agent effort override. Status bar indicator. Symbols: ○ ◐ ●

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffortLevel { Low, Medium, High }

impl EffortLevel {
    pub fn symbol(&self) -> &'static str {
        match self { Self::Low => "○", Self::Medium => "◐", Self::High => "●" }
    }
    pub fn label(&self) -> &'static str {
        match self { Self::Low => "low", Self::Medium => "medium", Self::High => "high" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() { "low" | "l" => Some(Self::Low), "medium" | "med" | "m" => Some(Self::Medium), "high" | "h" => Some(Self::High), _ => None }
    }
}

impl Default for EffortLevel { fn default() -> Self { Self::Medium } }

pub struct EffortManager {
    current: EffortLevel,
    auto_mode: bool,
}

impl EffortManager {
    pub fn new() -> Self { Self { current: EffortLevel::Medium, auto_mode: true } }
    pub fn set(&mut self, level: EffortLevel) { self.current = level; self.auto_mode = false; }
    pub fn set_auto(&mut self) { self.auto_mode = true; self.current = EffortLevel::Medium; }
    pub fn current(&self) -> EffortLevel { self.current }
    pub fn is_auto(&self) -> bool { self.auto_mode }
    pub fn display(&self) -> String {
        if self.auto_mode { format!("{} auto (→ {})", self.current.symbol(), self.current.label()) }
        else { format!("{} {}", self.current.symbol(), self.current.label()) }
    }
}

impl Default for EffortManager { fn default() -> Self { Self::new() } }
