// SPDX-License-Identifier: MIT
//! Folder Trust Discovery Service
//!
//! Scans a workspace directory for local configurations before granting trust.
//! Discovers commands, MCPs, hooks, skills, agents, and collects security warnings.

use std::path::Path;

const ARC_DIR: &str = ".arc";

#[derive(Debug, Clone, Default)]
pub struct FolderDiscoveryResults {
    pub commands: Vec<String>,
    pub mcps: Vec<String>,
    pub hooks: Vec<String>,
    pub skills: Vec<String>,
    pub agents: Vec<String>,
    pub settings: Vec<String>,
    pub security_warnings: Vec<String>,
    pub discovery_errors: Vec<String>,
}

impl FolderDiscoveryResults {
    pub fn has_configurations(&self) -> bool {
        !self.commands.is_empty()
            || !self.mcps.is_empty()
            || !self.hooks.is_empty()
            || !self.skills.is_empty()
            || !self.agents.is_empty()
            || !self.settings.is_empty()
    }

    pub fn has_security_warnings(&self) -> bool {
        !self.security_warnings.is_empty()
    }
}

/// Discover configurations in the given workspace directory before trusting it.
pub fn discover(workspace_dir: &Path) -> FolderDiscoveryResults {
    let mut results = FolderDiscoveryResults::default();
    let arc_dir = workspace_dir.join(ARC_DIR);

    if !arc_dir.exists() {
        return results;
    }

    discover_commands(&arc_dir, &mut results);
    discover_skills(&arc_dir, &mut results);
    discover_agents(&arc_dir, &mut results);
    discover_settings(&arc_dir, &mut results);

    results
}

fn discover_commands(arc_dir: &Path, results: &mut FolderDiscoveryResults) {
    let commands_dir = arc_dir.join("commands");
    if !commands_dir.exists() {
        return;
    }
    match std::fs::read_dir(&commands_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "toml") {
                    if let Some(stem) = path.file_stem() {
                        results.commands.push(stem.to_string_lossy().to_string());
                    }
                }
            }
        },
        Err(e) => {
            results
                .discovery_errors
                .push(format!("Failed to discover commands: {e}"));
        },
    }
}

fn discover_skills(arc_dir: &Path, results: &mut FolderDiscoveryResults) {
    let skills_dir = arc_dir.join("skills");
    if !skills_dir.exists() {
        return;
    }
    match std::fs::read_dir(&skills_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if entry.file_type().map_or(false, |t| t.is_dir()) {
                    let skill_md = entry.path().join("SKILL.md");
                    if skill_md.exists() {
                        results
                            .skills
                            .push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
        },
        Err(e) => {
            results
                .discovery_errors
                .push(format!("Failed to discover skills: {e}"));
        },
    }
}

fn discover_agents(arc_dir: &Path, results: &mut FolderDiscoveryResults) {
    let agents_dir = arc_dir.join("agents");
    if !agents_dir.exists() {
        return;
    }
    match std::fs::read_dir(&agents_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md") && !name.starts_with('_') {
                    results
                        .agents
                        .push(name.trim_end_matches(".md").to_string());
                }
            }
            if !results.agents.is_empty() {
                results
                    .security_warnings
                    .push("This project contains custom agents.".to_string());
            }
        },
        Err(e) => {
            results
                .discovery_errors
                .push(format!("Failed to discover agents: {e}"));
        },
    }
}

fn discover_settings(arc_dir: &Path, results: &mut FolderDiscoveryResults) {
    let settings_path = arc_dir.join("settings.toml");
    if !settings_path.exists() {
        return;
    }
    match std::fs::read_to_string(&settings_path) {
        Ok(content) => {
            if let Ok(table) = content.parse::<toml::Table>() {
                // Collect setting keys
                results.settings = table
                    .keys()
                    .filter(|k| !["mcp_servers", "hooks"].contains(&k.as_str()))
                    .cloned()
                    .collect();

                // Check MCP servers
                if let Some(toml::Value::Table(mcps)) = table.get("mcp_servers") {
                    results.mcps = mcps.keys().cloned().collect();
                }

                // Check hooks
                if let Some(toml::Value::Table(hooks)) = table.get("hooks") {
                    for (_event, hook_list) in hooks {
                        if let toml::Value::Array(arr) = hook_list {
                            for hook in arr {
                                if let Some(cmd) = hook.get("command").and_then(|v| v.as_str()) {
                                    results.hooks.push(cmd.to_string());
                                }
                            }
                        }
                    }
                }

                // Security warnings
                collect_security_warnings(&table, results);
            }
        },
        Err(e) => {
            results
                .discovery_errors
                .push(format!("Failed to read settings: {e}"));
        },
    }
}

fn collect_security_warnings(settings: &toml::Table, results: &mut FolderDiscoveryResults) {
    // Check if tools are auto-approved
    if let Some(toml::Value::Table(tools)) = settings.get("tools") {
        if let Some(toml::Value::Array(allowed)) = tools.get("allowed") {
            if !allowed.is_empty() {
                results
                    .security_warnings
                    .push("This project auto-approves certain tools (tools.allowed).".to_string());
            }
        }
        if tools.get("sandbox") == Some(&toml::Value::Boolean(false)) {
            results
                .security_warnings
                .push("This project disables the security sandbox (tools.sandbox).".to_string());
        }
    }

    // Check if folder trust is disabled
    if let Some(toml::Value::Table(security)) = settings.get("security") {
        if let Some(toml::Value::Table(ft)) = security.get("folder_trust") {
            if ft.get("enabled") == Some(&toml::Value::Boolean(false)) {
                results
                    .security_warnings
                    .push("This project attempts to disable folder trust.".to_string());
            }
        }
    }
}
