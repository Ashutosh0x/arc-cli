// SPDX-License-Identifier: MIT
//! HTTP Hook Executor — POST JSON to a URL and receive JSON back.
//! Supports custom headers, env var interpolation, and timeout control.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHookConfig {
    pub url: String,
    #[serde(default)]
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    POST,
    PUT,
    PATCH,
}

fn default_timeout() -> u64 {
    5000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookPayload {
    pub event: String,
    pub session_id: String,
    pub timestamp: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub mutate: Option<serde_json::Value>,
}

pub struct HttpHookExecutor;

impl HttpHookExecutor {
    /// Interpolate environment variables in a string: ${VAR_NAME}
    pub fn interpolate_env(input: &str) -> String {
        let mut result = input.to_string();
        let re_pattern = regex::Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").ok();
        if let Some(re) = re_pattern {
            for cap in re.captures_iter(input) {
                if let Some(var_name) = cap.get(1) {
                    if let Ok(val) = std::env::var(var_name.as_str()) {
                        result = result.replace(cap.get(0).map_or("", |m| m.as_str()), &val);
                    }
                }
            }
        }
        result
    }

    /// Build headers with env var interpolation
    pub fn build_headers(config: &HttpHookConfig) -> HashMap<String, String> {
        config
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), Self::interpolate_env(v)))
            .collect()
    }

    /// Execute an HTTP hook (async, requires reqwest at call site)
    pub fn build_payload(event: &str, session_id: &str, data: serde_json::Value) -> HookPayload {
        HookPayload {
            event: event.to_string(),
            session_id: session_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data,
        }
    }

    pub fn timeout(config: &HttpHookConfig) -> Duration {
        Duration::from_millis(config.timeout_ms)
    }
}
