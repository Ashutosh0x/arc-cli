//! Config file permissions guard.
//! Ensures config files are not world-readable.

use crate::config::ArcConfig;
use crate::error::ArcResult;
use tracing::info;
#[cfg(unix)]
use tracing::warn;

/// Check that the config directory and file have safe permissions.
pub fn check_config_permissions() -> ArcResult<Vec<String>> {
    #[allow(unused_mut)]
    let mut warnings = Vec::new();

    let config_dir = ArcConfig::dir()?;
    if !config_dir.exists() {
        return Ok(warnings);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir_meta = std::fs::metadata(&config_dir)?;
        let dir_mode = dir_meta.permissions().mode() & 0o777;

        if dir_mode & 0o077 != 0 {
            warnings.push(format!(
                "Config directory {:?} is accessible by others (mode: {:o}). \
                 Recommended: chmod 700",
                config_dir, dir_mode
            ));
            warn!("Config directory has unsafe permissions: {:o}", dir_mode);
        }

        let config_path = ArcConfig::path()?;
        if config_path.exists() {
            let file_meta = std::fs::metadata(&config_path)?;
            let file_mode = file_meta.permissions().mode() & 0o777;

            if file_mode & 0o077 != 0 {
                warnings.push(format!(
                    "Config file {:?} is accessible by others (mode: {:o}). \
                     Recommended: chmod 600",
                    config_path, file_mode
                ));
                warn!("Config file has unsafe permissions: {:o}", file_mode);
            }
        }
    }

    #[cfg(not(unix))]
    {
        info!("Config permission checks are Unix-only; skipping on this platform.");
    }

    Ok(warnings)
}

/// Fix config file permissions to safe defaults.
#[cfg(unix)]
pub fn fix_config_permissions() -> ArcResult<()> {
    use std::os::unix::fs::PermissionsExt;

    let config_dir = ArcConfig::dir()?;
    if config_dir.exists() {
        std::fs::set_permissions(&config_dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let config_path = ArcConfig::path()?;
    if config_path.exists() {
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600))?;
    }

    info!("Config permissions fixed");
    Ok(())
}
