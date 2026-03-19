//! Plugin installer: handles installation from marketplace, git, and local sources.

use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::manifest::LoadedPlugin;
use crate::registry::{PluginRegistry, PluginSource};

/// Result of a plugin installation.
#[derive(Debug)]
pub struct InstallResult {
    pub plugin_name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub hooks_registered: usize,
    pub commands_registered: usize,
    pub agents_registered: usize,
    pub skills_registered: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("Plugin '{0}' is already installed. Use --force to reinstall.")]
    AlreadyInstalled(String),

    #[error("Failed to download plugin: {0}")]
    Download(String),

    #[error("Failed to load plugin: {0}")]
    Load(#[from] crate::manifest::PluginLoadError),

    #[error("Failed to save registry: {0}")]
    Registry(#[from] std::io::Error),

    #[error("Plugin '{name}' requires ARC CLI >= {required}, but current version is {current}")]
    VersionMismatch {
        name: String,
        required: String,
        current: String,
    },

    #[error("Git clone failed: {0}")]
    GitClone(String),
}

pub struct PluginInstaller {
    /// Root directory where plugins are installed.
    plugins_dir: PathBuf,
    /// Project root for registry storage.
    project_root: PathBuf,
}

impl PluginInstaller {
    pub fn new(project_root: &Path) -> Self {
        Self {
            plugins_dir: project_root.join(".arc").join("plugins"),
            project_root: project_root.to_path_buf(),
        }
    }

    /// Install a plugin from a marketplace reference.
    /// Format: "plugin-name@marketplace-name"
    /// Example: "security-scanner@arc-plugins-official"
    pub async fn install_from_marketplace(
        &self,
        spec: &str,
        force: bool,
    ) -> Result<InstallResult, InstallError> {
        let (plugin_name, marketplace) = parse_plugin_spec(spec);

        let registry_url = resolve_marketplace_url(&marketplace);
        info!(
            plugin = %plugin_name,
            marketplace = %marketplace,
            url = %registry_url,
            "Installing from marketplace"
        );

        // Download the plugin
        let download_dir = self.plugins_dir.join(&plugin_name);

        if download_dir.exists() {
            if force {
                std::fs::remove_dir_all(&download_dir)?;
            } else {
                return Err(InstallError::AlreadyInstalled(plugin_name));
            }
        }

        std::fs::create_dir_all(&download_dir)?;

        // Clone from marketplace git repo
        let repo_url = format!("{}/{}", registry_url, plugin_name);
        self.git_clone(&repo_url, &download_dir, None).await?;

        self.finalize_install(&download_dir, PluginSource::Marketplace {
            registry_url: registry_url.to_string(),
        })
    }

    /// Install from a git repository URL.
    pub async fn install_from_git(
        &self,
        url: &str,
        branch: Option<&str>,
        force: bool,
    ) -> Result<InstallResult, InstallError> {
        // Derive plugin name from URL
        let plugin_name = url
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .trim_end_matches(".git");

        let download_dir = self.plugins_dir.join(plugin_name);

        if download_dir.exists() {
            if force {
                std::fs::remove_dir_all(&download_dir)?;
            } else {
                return Err(InstallError::AlreadyInstalled(plugin_name.to_string()));
            }
        }

        std::fs::create_dir_all(&download_dir)?;
        self.git_clone(url, &download_dir, branch).await?;

        self.finalize_install(
            &download_dir,
            PluginSource::Git {
                url: url.to_string(),
                branch: branch.map(String::from),
            },
        )
    }

    /// Install from a local directory (symlink or copy).
    pub fn install_from_local(
        &self,
        source_dir: &Path,
        force: bool,
    ) -> Result<InstallResult, InstallError> {
        // Load to get the name
        let plugin = LoadedPlugin::load_from_dir(source_dir)?;
        let plugin_name = &plugin.manifest.plugin.name;

        let install_dir = self.plugins_dir.join(plugin_name);

        if install_dir.exists() {
            if force {
                std::fs::remove_dir_all(&install_dir)?;
            } else {
                return Err(InstallError::AlreadyInstalled(plugin_name.clone()));
            }
        }

        // Copy the directory
        copy_dir_recursive(source_dir, &install_dir)?;

        self.finalize_install(
            &install_dir,
            PluginSource::Local {
                path: source_dir.display().to_string(),
            },
        )
    }

    /// Uninstall a plugin by name.
    pub fn uninstall(&self, plugin_name: &str) -> Result<(), InstallError> {
        let mut registry = PluginRegistry::load(&self.project_root);

        if let Some(entry) = registry.uninstall(plugin_name) {
            // Remove files
            let install_path = PathBuf::from(&entry.install_path);
            if install_path.exists() {
                std::fs::remove_dir_all(&install_path)?;
            }
            registry.save(&self.project_root)?;
            info!(plugin = %plugin_name, "Plugin uninstalled");
            Ok(())
        } else {
            warn!(plugin = %plugin_name, "Plugin not found in registry");
            Ok(())
        }
    }

    /// List all installed plugins.
    pub fn list(&self) -> Vec<crate::registry::PluginEntry> {
        let registry = PluginRegistry::load(&self.project_root);
        registry.list().into_iter().cloned().collect()
    }

    // ── Internal helpers ──────────────────────────────────────────────

    fn finalize_install(
        &self,
        install_dir: &Path,
        source: PluginSource,
    ) -> Result<InstallResult, InstallError> {
        let plugin = LoadedPlugin::load_from_dir(install_dir)?;

        let result = InstallResult {
            plugin_name: plugin.manifest.plugin.name.clone(),
            version: plugin.manifest.plugin.version.clone(),
            install_path: install_dir.to_path_buf(),
            hooks_registered: plugin.hooks.len(),
            commands_registered: plugin.commands.len(),
            agents_registered: plugin.agents.len(),
            skills_registered: plugin.skills.len(),
        };

        // Register in the local registry
        let mut registry = PluginRegistry::load(&self.project_root);
        registry.register(&plugin, source);
        registry.save(&self.project_root)?;

        info!(
            plugin = %result.plugin_name,
            version = %result.version,
            hooks = result.hooks_registered,
            commands = result.commands_registered,
            agents = result.agents_registered,
            skills = result.skills_registered,
            "Plugin installed successfully"
        );

        Ok(result)
    }

    async fn git_clone(
        &self,
        url: &str,
        target: &Path,
        branch: Option<&str>,
    ) -> Result<(), InstallError> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("clone").arg("--depth").arg("1");

        if let Some(b) = branch {
            cmd.arg("--branch").arg(b);
        }

        cmd.arg(url).arg(target);

        let output = cmd
            .output()
            .await
            .map_err(|e| InstallError::GitClone(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InstallError::GitClone(stderr.to_string()));
        }

        Ok(())
    }
}

fn parse_plugin_spec(spec: &str) -> (String, String) {
    if let Some((name, marketplace)) = spec.rsplit_once('@') {
        (name.to_string(), marketplace.to_string())
    } else {
        (spec.to_string(), "arc-plugins-official".to_string())
    }
}

fn resolve_marketplace_url(marketplace: &str) -> &str {
    match marketplace {
        "arc-plugins-official" => "https://github.com/arc-cli/arc-plugins-official",
        other => {
            tracing::warn!(marketplace = %other, "Unknown marketplace, using as URL");
            other
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;
    for entry in walkdir::WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let relative = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
