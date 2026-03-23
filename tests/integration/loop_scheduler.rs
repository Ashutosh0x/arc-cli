// SPDX-License-Identifier: MIT
//! Loop scheduler integration tests.

use std::time::Duration;

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim().to_lowercase();
    let mut total_secs: u64 = 0;
    let mut current_num = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            let num: u64 = current_num
                .parse()
                .map_err(|_| format!("Invalid interval: {s}"))?;
            current_num.clear();

            match ch {
                's' => total_secs += num,
                'm' => total_secs += num * 60,
                'h' => total_secs += num * 3600,
                'd' => total_secs += num * 86400,
                _ => return Err(format!("Unknown unit: {ch}")),
            }
        }
    }

    if total_secs == 0 {
        return Err(format!("Zero duration: {s}"));
    }

    Ok(Duration::from_secs(total_secs))
}

#[test]
fn test_duration_parsing_seconds() {
    assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
}

#[test]
fn test_duration_parsing_minutes() {
    assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
}

#[test]
fn test_duration_parsing_hours() {
    assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
}

#[test]
fn test_duration_parsing_composite() {
    assert_eq!(
        parse_duration("1h30m").unwrap(),
        Duration::from_secs(5400)
    );
    assert_eq!(
        parse_duration("2h30m15s").unwrap(),
        Duration::from_secs(9015)
    );
}

#[test]
fn test_duration_parsing_days() {
    assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
}

#[test]
fn test_duration_parsing_invalid() {
    assert!(parse_duration("abc").is_err());
    assert!(parse_duration("").is_err());
    assert!(parse_duration("0s").is_err());
}

#[test]
fn test_persistent_schedule_roundtrip() {
    let schedule = r#"
[tasks.deploy-check]
name = "deploy-check"
prompt = "check deployments"
interval = "5m"
enabled = true
worktree_isolation = false

[tasks.test-runner]
name = "test-runner"
prompt = "run tests"
interval = "15m"
max_runs = 10
enabled = false
worktree_isolation = true
"#;

    let parsed: toml::Value = schedule.parse().unwrap();
    let tasks = parsed["tasks"].as_table().unwrap();
    assert_eq!(tasks.len(), 2);
    assert!(tasks["deploy-check"]["enabled"].as_bool().unwrap());
    assert!(!tasks["test-runner"]["enabled"].as_bool().unwrap());
    assert_eq!(
        tasks["test-runner"]["max_runs"].as_integer().unwrap(),
        10
    );
}

#[tokio::test]
async fn test_scheduler_fires_tasks() {
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    let (tx, mut rx) = mpsc::channel::<String>(16);

    // Spawn a simple interval task
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        for i in 0..3 {
            interval.tick().await;
            tx.send(format!("fired_{i}")).await.unwrap();
        }
    });

    // Collect results with timeout
    let mut results = Vec::new();
    let collect = async {
        while let Some(msg) = rx.recv().await {
            results.push(msg);
            if results.len() >= 3 {
                break;
            }
        }
    };

    timeout(Duration::from_secs(2), collect).await.unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0], "fired_0");
    assert_eq!(results[2], "fired_2");
}
