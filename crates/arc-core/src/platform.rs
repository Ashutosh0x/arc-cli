//! # Platform Hardening — Cross-Platform Compatibility Layer
//!
//! Windows path casing, CRLF handling, clipboard Unicode, terminal rendering fixes.

use std::path::{Path, PathBuf};

/// Normalize a file path for cross-platform consistency.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        result.push(component);
    }

    #[cfg(target_os = "windows")]
    {
        // Normalize drive letter to uppercase (C: vs c:).
        let s = result.to_string_lossy().to_string();
        if s.len() >= 2 && s.as_bytes()[1] == b':' {
            let upper = s[..1].to_uppercase();
            return PathBuf::from(format!("{}{}", upper, &s[1..]));
        }
    }

    result
}

/// Normalize line endings to LF.
pub fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

/// Detect line ending style of a file.
pub fn detect_line_endings(content: &str) -> LineEndings {
    if content.contains("\r\n") {
        LineEndings::Crlf
    } else {
        LineEndings::Lf
    }
}

/// Apply line ending style to content.
pub fn apply_line_endings(content: &str, style: LineEndings) -> String {
    let normalized = normalize_line_endings(content);
    match style {
        LineEndings::Lf => normalized,
        LineEndings::Crlf => normalized.replace('\n', "\r\n"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEndings {
    Lf,
    Crlf,
}

/// Copy text to clipboard cross-platform.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Use PowerShell Set-Clipboard for Unicode safety.
        let escaped = text.replace('\'', "''");
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Set-Clipboard -Value '{escaped}'"),
            ])
            .output()
            .map_err(|e| format!("Clipboard error: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let mut child = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("pbcopy failed: {e}"))?;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
        child.wait().map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        // Try xclip, then xsel, then wl-copy (Wayland).
        let cmds = [
            "xclip -selection clipboard",
            "xsel --clipboard --input",
            "wl-copy",
        ];
        for cmd in &cmds {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if let Ok(mut child) = std::process::Command::new(parts[0])
                .args(&parts[1..])
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                return Ok(());
            }
        }
        return Err("No clipboard tool found (xclip/xsel/wl-copy)".into());
    }
    #[allow(unreachable_code)]
    Err("Unsupported platform".into())
}

/// Get the appropriate shell for the current platform.
pub fn platform_shell() -> (&'static str, &'static str) {
    #[cfg(target_os = "windows")]
    {
        return ("cmd", "/C");
    }
    #[cfg(not(target_os = "windows"))]
    {
        return ("sh", "-c");
    }
}

/// Check if running in WSL.
pub fn is_wsl() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(release) = std::fs::read_to_string("/proc/version") {
            return release.to_lowercase().contains("microsoft")
                || release.to_lowercase().contains("wsl");
        }
    }
    false
}

/// Get the user's home directory cross-platform.
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Get the config directory cross-platform.
pub fn config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                home_dir()
                    .unwrap_or_default()
                    .join("AppData")
                    .join("Roaming")
            })
            .join("arc-cli")
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .unwrap_or_default()
            .join("Library")
            .join("Application Support")
            .join("arc-cli")
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir().unwrap_or_default().join(".config"))
            .join("arc-cli")
    }
}

/// Get the data directory cross-platform.
pub fn data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir().unwrap_or_default().join("AppData").join("Local"))
            .join("arc-cli")
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .unwrap_or_default()
            .join("Library")
            .join("Application Support")
            .join("arc-cli")
            .join("data")
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir().unwrap_or_default().join(".local").join("share"))
            .join("arc-cli")
    }
}
