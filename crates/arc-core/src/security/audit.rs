// SPDX-License-Identifier: MIT
//! Structured audit logging system.
//! Writes security-relevant events to `~/.arc/audit.log` in JSON format.

use crate::config::ArcConfig;
use crate::error::ArcResult;
use chrono::Utc;
use serde::Serialize;
use std::io::Write;
use std::sync::Mutex;
use tracing::debug;

static AUDIT_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub provider: Option<String>,
    pub detail: String,
    pub severity: Severity,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    AuthAttempt,
    AuthSuccess,
    AuthFailure,
    CredentialAccess,
    CredentialStore,
    CredentialDelete,
    PromptInjectionBlocked,
    DataLeakPrevented,
    RateLimitHit,
    ConfigAccess,
    SessionStart,
    SessionEnd,
    ToolInvocation,
    SecurityViolation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// Initialize the audit log file.
pub fn init_audit_log() -> ArcResult<()> {
    let dir = ArcConfig::dir()?;
    std::fs::create_dir_all(&dir)?;

    let path = dir.join("audit.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    if let Ok(mut guard) = AUDIT_FILE.lock() {
        *guard = Some(file);
    }

    debug!("Audit log initialized at {}", path.display());
    Ok(())
}

/// Write an audit event.
pub fn log_event(event: AuditEvent) {
    if let Ok(mut guard) = AUDIT_FILE.lock() {
        if let Some(ref mut file) = *guard {
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = writeln!(file, "{json}");
            }
        }
    }
}

/// Convenience: log a security event.
pub fn log_security_event(event_type: AuditEventType, detail: &str, severity: Severity) {
    log_event(AuditEvent {
        timestamp: Utc::now().to_rfc3339(),
        event_type,
        provider: None,
        detail: detail.to_string(),
        severity,
    });
}
