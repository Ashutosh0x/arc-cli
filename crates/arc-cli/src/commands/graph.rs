//! `arc graph` subcommands — structural code intelligence via codebase-memory-mcp.

use arc_mcp::client::McpClient;
use arc_mcp::tools;
use std::path::PathBuf;

/// Run `arc graph index` — index the current project.
pub async fn run_index(repo_path: Option<String>) -> anyhow::Result<()> {
    let path = repo_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .to_string()
    });

    println!("⚡ Indexing {} ...", path);

    let client = spawn_mcp_client().await?;
    let result = tools::index_repository(&client, &path).await?;

    if let Some(text) = result.text() {
        println!("{}", text);
    } else {
        println!("✓ Indexing complete.");
    }

    client.shutdown().await;
    Ok(())
}

/// Run `arc graph search <pattern>` — structural search.
pub async fn run_search(
    pattern: &str,
    label: Option<String>,
    file_pattern: Option<String>,
) -> anyhow::Result<()> {
    let client = spawn_mcp_client().await?;

    let request = tools::SearchGraphRequest {
        name_pattern: Some(pattern.to_string()),
        label,
        file_pattern,
        project: None,
        limit: Some(25),
        offset: None,
    };

    let result = tools::search_graph(&client, request).await?;

    if let Some(text) = result.text() {
        println!("{}", text);
    } else {
        println!("No results found.");
    }

    client.shutdown().await;
    Ok(())
}

/// Run `arc graph trace <function>` — call graph traversal.
pub async fn run_trace(
    function_name: &str,
    direction: Option<String>,
    depth: Option<u32>,
) -> anyhow::Result<()> {
    let client = spawn_mcp_client().await?;

    let request = tools::TraceCallPathRequest {
        function_name: function_name.to_string(),
        direction: Some(direction.unwrap_or_else(|| "both".to_string())),
        depth: Some(depth.unwrap_or(3)),
        project: None,
    };

    let result = tools::trace_call_path(&client, request).await?;

    if let Some(text) = result.text() {
        println!("{}", text);
    } else {
        println!("No call paths found for '{}'.", function_name);
    }

    client.shutdown().await;
    Ok(())
}

/// Run `arc graph architecture` — full architecture overview.
pub async fn run_architecture() -> anyhow::Result<()> {
    let client = spawn_mcp_client().await?;

    let request = tools::GetArchitectureRequest { project: None };
    let result = tools::get_architecture(&client, request).await?;

    if let Some(text) = result.text() {
        println!("{}", text);
    } else {
        println!("No architecture data available. Run `arc graph index` first.");
    }

    client.shutdown().await;
    Ok(())
}

/// Run `arc graph impact` — git diff impact analysis.
pub async fn run_impact(repo_path: Option<String>) -> anyhow::Result<()> {
    let path = repo_path.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .to_string()
    });

    let client = spawn_mcp_client().await?;

    let request = tools::DetectChangesRequest {
        repo_path: path,
        project: None,
    };

    let result = tools::detect_changes(&client, request).await?;

    if let Some(text) = result.text() {
        println!("{}", text);
    } else {
        println!("No uncommitted changes detected.");
    }

    client.shutdown().await;
    Ok(())
}

/// Run `arc graph query <cypher>` — execute Cypher-like graph queries.
pub async fn run_query(query: &str) -> anyhow::Result<()> {
    let client = spawn_mcp_client().await?;

    let request = tools::QueryGraphRequest {
        query: query.to_string(),
        project: None,
    };

    let result = tools::query_graph(&client, request).await?;

    if let Some(text) = result.text() {
        println!("{}", text);
    } else {
        println!("Query returned no results.");
    }

    client.shutdown().await;
    Ok(())
}

/// Spawn a codebase-memory-mcp MCP client.
async fn spawn_mcp_client() -> anyhow::Result<McpClient> {
    McpClient::spawn("codebase-memory-mcp", &[], "codebase-memory")
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to connect to codebase-memory-mcp: {}. \
                 Install it: https://github.com/DeusData/codebase-memory-mcp/releases",
                e
            )
        })
}
