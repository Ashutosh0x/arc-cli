// SPDX-License-Identifier: MIT
//! MCP (Model Context Protocol) JSON-RPC 2.0 stdio client.
//!
//! Spawns an MCP server binary as a child process and communicates
//! via JSON-RPC 2.0 over stdin/stdout.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, warn};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// JSON-RPC 2.0 request.
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: Value,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MCP error {}: {}", self.code, self.message)
    }
}

/// MCP stdio client — spawns a child process and speaks JSON-RPC 2.0.
pub struct McpClient {
    child: Mutex<Child>,
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
    server_name: String,
}

impl McpClient {
    /// Spawn an MCP server process and return a connected client.
    pub async fn spawn(command: &str, args: &[String], name: &str) -> Result<Self, McpClientError> {
        debug!("Spawning MCP server: {} {:?}", command, args);

        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| McpClientError::SpawnFailed {
                command: command.to_string(),
                reason: e.to_string(),
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or(McpClientError::NoPipe("stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(McpClientError::NoPipe("stdout".into()))?;

        let client = Self {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            server_name: name.to_string(),
        };

        // Initialize the MCP session
        client.initialize().await?;

        Ok(client)
    }

    /// Send the MCP `initialize` handshake.
    async fn initialize(&self) -> Result<Value, McpClientError> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "arc-cli",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let result = self.send_request("initialize", params).await?;
        debug!("MCP server {} initialized: {:?}", self.server_name, result);

        // Send initialized notification (no response expected)
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let mut stdin = self.stdin.lock().await;
        let msg = serde_json::to_string(&notification).map_err(McpClientError::SerializeError)?;
        stdin
            .write_all(format!("{}\n", msg).as_bytes())
            .await
            .map_err(|e| McpClientError::IoError(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| McpClientError::IoError(e.to_string()))?;

        Ok(result)
    }

    /// Call an MCP tool by name with the given arguments.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, McpClientError> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        self.send_request("tools/call", params).await
    }

    /// List available tools from the MCP server.
    pub async fn list_tools(&self) -> Result<Value, McpClientError> {
        self.send_request("tools/list", serde_json::json!({})).await
    }

    /// Send a JSON-RPC request and wait for the response.
    async fn send_request(&self, method: &str, params: Value) -> Result<Value, McpClientError> {
        let id = REQUEST_ID.fetch_add(1, Ordering::Relaxed);

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let request_str =
            serde_json::to_string(&request).map_err(McpClientError::SerializeError)?;

        debug!("MCP request [{}]: {} {}", id, method, request_str);

        // Write request
        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(format!("{}\n", request_str).as_bytes())
                .await
                .map_err(|e| McpClientError::IoError(e.to_string()))?;
            stdin
                .flush()
                .await
                .map_err(|e| McpClientError::IoError(e.to_string()))?;
        }

        // Read response — skip notifications, find our response by id
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = stdout
                .read_line(&mut line)
                .await
                .map_err(|e| McpClientError::IoError(e.to_string()))?;

            if bytes_read == 0 {
                return Err(McpClientError::ServerClosed);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as JSON-RPC response
            if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                if let Some(err) = response.error {
                    return Err(McpClientError::RpcError(err));
                }
                return Ok(response.result.unwrap_or(Value::Null));
            }

            // Notification or non-response — skip
            debug!("MCP non-response line: {}", trimmed);
        }
    }

    /// Cleanly shut down the MCP server.
    pub async fn shutdown(&self) {
        let mut child = self.child.lock().await;
        if let Err(e) = child.kill().await {
            warn!("Failed to kill MCP server {}: {}", self.server_name, e);
        } else {
            debug!("MCP server {} shut down", self.server_name);
        }
    }

    /// Check if the child process is still running.
    pub async fn is_alive(&self) -> bool {
        let mut child = self.child.lock().await;
        matches!(child.try_wait(), Ok(None))
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        &self.server_name
    }
}

/// Errors from the MCP client.
#[derive(Debug, thiserror::Error)]
pub enum McpClientError {
    #[error("Failed to spawn MCP server '{command}': {reason}")]
    SpawnFailed { command: String, reason: String },

    #[error("MCP server did not provide {0} pipe")]
    NoPipe(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("JSON serialization error: {0}")]
    SerializeError(serde_json::Error),

    #[error("MCP server closed unexpectedly")]
    ServerClosed,

    #[error("MCP RPC error: {0}")]
    RpcError(JsonRpcError),
}
