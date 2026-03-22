//! Typed wrappers for codebase-memory-mcp MCP tools.
//!
//! Each function wraps a specific MCP tool call with typed parameters
//! and structured return types.

use crate::client::{McpClient, McpClientError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─── Request types ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct IndexRequest {
    pub repo_path: String,
}

#[derive(Debug, Serialize)]
pub struct SearchGraphRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct TraceCallPathRequest {
    pub function_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>, // "inbound", "outbound", "both"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DetectChangesRequest {
    pub repo_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryGraphRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetArchitectureRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetCodeSnippetRequest {
    pub qualified_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchCodeRequest {
    pub pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_pattern: Option<String>,
}

// ─── Response types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ToolResult {
    pub content: Option<Vec<ContentBlock>>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

impl ToolResult {
    /// Extract the text content from the first content block.
    pub fn text(&self) -> Option<&str> {
        self.content
            .as_ref()?
            .first()?
            .text
            .as_deref()
    }

    /// Parse the text content as JSON.
    pub fn parse_json<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        let text = self.text()?;
        serde_json::from_str(text).ok()
    }
}

// ─── Tool call functions ──────────────────────────────────────────

/// Index a repository into the knowledge graph.
pub async fn index_repository(
    client: &McpClient,
    repo_path: &str,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(IndexRequest {
        repo_path: repo_path.to_string(),
    })
    .map_err(McpClientError::SerializeError)?;

    let result = client.call_tool("index_repository", args).await?;
    parse_tool_result(result)
}

/// Search the knowledge graph by name pattern, label, or file pattern.
pub async fn search_graph(
    client: &McpClient,
    request: SearchGraphRequest,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(request).map_err(McpClientError::SerializeError)?;
    let result = client.call_tool("search_graph", args).await?;
    parse_tool_result(result)
}

/// Trace call paths — who calls a function and what it calls.
pub async fn trace_call_path(
    client: &McpClient,
    request: TraceCallPathRequest,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(request).map_err(McpClientError::SerializeError)?;
    let result = client.call_tool("trace_call_path", args).await?;
    parse_tool_result(result)
}

/// Detect changes — map git diff to affected symbols with risk classification.
pub async fn detect_changes(
    client: &McpClient,
    request: DetectChangesRequest,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(request).map_err(McpClientError::SerializeError)?;
    let result = client.call_tool("detect_changes", args).await?;
    parse_tool_result(result)
}

/// Get architecture overview — languages, packages, routes, hotspots, clusters.
pub async fn get_architecture(
    client: &McpClient,
    request: GetArchitectureRequest,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(request).map_err(McpClientError::SerializeError)?;
    let result = client.call_tool("get_architecture", args).await?;
    parse_tool_result(result)
}

/// Execute a Cypher-like graph query.
pub async fn query_graph(
    client: &McpClient,
    request: QueryGraphRequest,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(request).map_err(McpClientError::SerializeError)?;
    let result = client.call_tool("query_graph", args).await?;
    parse_tool_result(result)
}

/// Get code snippet for a function by qualified name.
pub async fn get_code_snippet(
    client: &McpClient,
    request: GetCodeSnippetRequest,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(request).map_err(McpClientError::SerializeError)?;
    let result = client.call_tool("get_code_snippet", args).await?;
    parse_tool_result(result)
}

/// Grep-like text search within indexed project files.
pub async fn search_code(
    client: &McpClient,
    request: SearchCodeRequest,
) -> Result<ToolResult, McpClientError> {
    let args = serde_json::to_value(request).map_err(McpClientError::SerializeError)?;
    let result = client.call_tool("search_code", args).await?;
    parse_tool_result(result)
}

/// List all indexed projects.
pub async fn list_projects(client: &McpClient) -> Result<ToolResult, McpClientError> {
    let result = client
        .call_tool("list_projects", serde_json::json!({}))
        .await?;
    parse_tool_result(result)
}

/// Get graph schema — node/edge counts, relationship patterns.
pub async fn get_graph_schema(client: &McpClient) -> Result<ToolResult, McpClientError> {
    let result = client
        .call_tool("get_graph_schema", serde_json::json!({}))
        .await?;
    parse_tool_result(result)
}

// ─── Helpers ──────────────────────────────────────────────────────

fn parse_tool_result(value: Value) -> Result<ToolResult, McpClientError> {
    serde_json::from_value(value).map_err(McpClientError::SerializeError)
}
