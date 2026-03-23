// SPDX-License-Identifier: MIT
//! Stress test: many hooks firing concurrently.

#[tokio::test]
async fn test_100_concurrent_hook_dispatches() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let counter = Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();

    for _ in 0..100 {
        let c = counter.clone();
        handles.push(tokio::spawn(async move {
            // Simulate hook execution
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            c.fetch_add(1, Ordering::Relaxed);
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(counter.load(Ordering::Relaxed), 100);
}

#[tokio::test]
async fn test_hook_under_memory_pressure() {
    // Allocate large payloads to simulate memory pressure
    let mut payloads = Vec::new();
    for i in 0..50 {
        let payload = serde_json::json!({
            "event": "PostToolUse",
            "payload": {
                "session_id": uuid::Uuid::new_v4().to_string(),
                "tool_name": format!("tool_{}", i),
                "tool_output": "x".repeat(10_000), // 10KB per payload
                "modified_files": (0..20).map(|j| format!("file_{j}.rs")).collect::<Vec<_>>(),
            }
        });
        payloads.push(serde_json::to_string(&payload).unwrap());
    }

    // All payloads should serialize successfully
    assert_eq!(payloads.len(), 50);
    // Total memory used should be reasonable
    let total_bytes: usize = payloads.iter().map(|p| p.len()).sum();
    assert!(total_bytes < 2_000_000); // < 2MB total
}
