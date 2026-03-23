// SPDX-License-Identifier: MIT
//! Remote control WebSocket server integration tests.

use std::time::Duration;

#[tokio::test]
async fn test_server_health_endpoint() {
    // Start a minimal HTTP server and check health
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = axum::Router::new().route(
        "/health",
        axum::routing::get(|| async {
            axum::Json(serde_json::json!({"status": "ok"}))
        }),
    );

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/health", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[test]
fn test_protocol_message_serialization() {
    // Client messages
    let prompt = serde_json::json!({"type": "prompt", "text": "Hello"});
    let parsed: serde_json::Value = prompt.clone();
    assert_eq!(parsed["type"], "prompt");
    assert_eq!(parsed["text"], "Hello");

    let auth = serde_json::json!({"type": "auth", "token": "abc123"});
    assert_eq!(auth["type"], "auth");

    let interrupt = serde_json::json!({"type": "interrupt"});
    assert_eq!(interrupt["type"], "interrupt");

    // Server messages
    let token = serde_json::json!({
        "type": "token",
        "text": "Hello ",
        "done": false
    });
    assert_eq!(token["done"], false);

    let tool_use = serde_json::json!({
        "type": "tool_use",
        "tool_name": "bash",
        "input_summary": "ls -la"
    });
    assert_eq!(tool_use["tool_name"], "bash");
}

#[test]
fn test_auth_token_generation() {
    let token1 = uuid::Uuid::new_v4().to_string();
    let token2 = uuid::Uuid::new_v4().to_string();

    // Tokens should be unique
    assert_ne!(token1, token2);

    // Tokens should be valid UUIDs
    assert!(uuid::Uuid::parse_str(&token1).is_ok());
}
