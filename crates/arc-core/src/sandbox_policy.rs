// SPDX-License-Identifier: MIT
//! # Sandbox Network Policy — Filesystem + Network Isolation
//!
//! Controls allowedDomains, proxy ports, unix sockets, excluded commands.
//! Extends existing Landlock sandbox with network-aware policies.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Network isolation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub allow_unix_sockets: Vec<String>,
    #[serde(default)]
    pub allow_all_unix_sockets: bool,
    #[serde(default)]
    pub allow_local_binding: bool,
    #[serde(default)]
    pub http_proxy_port: Option<u16>,
    #[serde(default)]
    pub socks_proxy_port: Option<u16>,
    #[serde(default)]
    pub enable_weaker_nested_sandbox: bool,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            allowed_domains: Vec::new(),
            allow_unix_sockets: Vec::new(),
            allow_all_unix_sockets: false,
            allow_local_binding: false,
            http_proxy_port: None,
            socks_proxy_port: None,
            enable_weaker_nested_sandbox: false,
        }
    }
}

/// Full sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_allow_bash_if_sandboxed: bool,
    #[serde(default)]
    pub allow_unsandboxed_commands: bool,
    #[serde(default)]
    pub excluded_commands: Vec<String>,
    #[serde(default)]
    pub network: NetworkPolicy,
    #[serde(default)]
    pub filesystem: FilesystemPolicy,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_allow_bash_if_sandboxed: false,
            allow_unsandboxed_commands: true,
            excluded_commands: Vec::new(),
            network: NetworkPolicy::default(),
            filesystem: FilesystemPolicy::default(),
        }
    }
}

/// Filesystem isolation policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemPolicy {
    #[serde(default)]
    pub allow_write: Vec<String>,
    #[serde(default)]
    pub deny_read: Vec<String>,
    #[serde(default)]
    pub allow_read: Vec<String>,
}

/// Sandbox enforcement engine.
pub struct SandboxEnforcer {
    config: SandboxConfig,
    allowed_set: HashSet<String>,
}

impl SandboxEnforcer {
    pub fn new(config: SandboxConfig) -> Self {
        let allowed_set: HashSet<String> = config.network.allowed_domains.iter().cloned().collect();
        Self {
            config,
            allowed_set,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if a domain is allowed for network access.
    pub fn check_domain(&self, domain: &str) -> DomainDecision {
        if !self.config.enabled {
            return DomainDecision::Allowed;
        }
        if self.allowed_set.is_empty() {
            return DomainDecision::Allowed;
        }
        if self.allowed_set.contains(domain) {
            return DomainDecision::Allowed;
        }
        // Wildcard subdomain matching.
        for allowed in &self.config.network.allowed_domains {
            if allowed.starts_with("*.") {
                let suffix = &allowed[1..];
                if domain.ends_with(suffix) {
                    return DomainDecision::Allowed;
                }
            }
        }
        DomainDecision::Blocked(format!("Domain '{domain}' not in allowedDomains"))
    }

    /// Check if a command is excluded from sandboxing.
    pub fn is_excluded_command(&self, command: &str) -> bool {
        self.config
            .excluded_commands
            .iter()
            .any(|exc| command.starts_with(exc))
    }

    /// Check if bash should be auto-allowed when sandboxed.
    pub fn should_auto_allow_bash(&self) -> bool {
        self.config.enabled && self.config.auto_allow_bash_if_sandboxed
    }

    /// Check if a file write is allowed.
    pub fn check_write(&self, path: &str) -> bool {
        if !self.config.enabled {
            return true;
        }
        if self.config.filesystem.allow_write.is_empty() {
            return true;
        }
        self.config
            .filesystem
            .allow_write
            .iter()
            .any(|p| path.starts_with(p))
    }

    /// Check if a file read is blocked.
    pub fn check_read(&self, path: &str) -> bool {
        if !self.config.enabled {
            return true;
        }
        // Check deny first, then allow exceptions.
        for denied in &self.config.filesystem.deny_read {
            if path.starts_with(denied) {
                return self
                    .config
                    .filesystem
                    .allow_read
                    .iter()
                    .any(|a| path.starts_with(a));
            }
        }
        true
    }

    /// Validate sandbox dependencies on the current platform.
    pub fn check_dependencies() -> Vec<SandboxDependency> {
        let mut deps = Vec::new();
        #[cfg(target_os = "linux")]
        {
            deps.push(SandboxDependency {
                name: "Landlock".into(),
                available: Self::check_landlock(),
                required: true,
            });
            deps.push(SandboxDependency {
                name: "ripgrep (rg)".into(),
                available: which::which("rg").is_ok(),
                required: false,
            });
        }
        #[cfg(target_os = "macos")]
        {
            deps.push(SandboxDependency {
                name: "sandbox-exec".into(),
                available: std::path::Path::new("/usr/bin/sandbox-exec").exists(),
                required: true,
            });
        }
        #[cfg(target_os = "windows")]
        {
            deps.push(SandboxDependency {
                name: "Windows Sandbox".into(),
                available: false,
                required: false,
            });
        }
        deps
    }

    #[cfg(target_os = "linux")]
    fn check_landlock() -> bool {
        std::path::Path::new("/sys/kernel/security/landlock").exists()
    }
}

#[derive(Debug)]
pub enum DomainDecision {
    Allowed,
    Blocked(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxDependency {
    pub name: String,
    pub available: bool,
    pub required: bool,
}
