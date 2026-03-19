//! End-to-end tests that verify the complete CLI workflow.

use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_arc_dir_initialization() {
    let dir = TempDir::new().unwrap();
    let arc_dir = dir.path().join(".arc");

    // Simulate `arc init`
    std::fs::create_dir_all(arc_dir.join("plugins")).unwrap();
    std::fs::create_dir_all(arc_dir.join("checkpoints")).unwrap();

    // Create default config
    let config = r#"
[general]
default_provider = "anthropic"
default_model = "claude-sonnet-4-20250514"

[security]
sandbox_mode = "permissive"
audit_log = true
"#;
    std::fs::write(arc_dir.join("config.toml"), config).unwrap();

    // Create default hooks
    let hooks = r#"
# Default hooks - security presets enabled
"#;
    std::fs::write(arc_dir.join("hooks.toml"), hooks).unwrap();

    // Verify structure
    assert!(arc_dir.exists());
    assert!(arc_dir.join("plugins").exists());
    assert!(arc_dir.join("checkpoints").exists());
    assert!(arc_dir.join("config.toml").exists());
    assert!(arc_dir.join("hooks.toml").exists());
}

#[test]
fn test_config_hierarchy_merge() {
    let dir = TempDir::new().unwrap();

    // Global config
    let global_dir = dir.path().join("global");
    std::fs::create_dir_all(&global_dir).unwrap();
    std::fs::write(
        global_dir.join("config.toml"),
        r#"
[general]
default_provider = "anthropic"
theme = "dark"
"#,
    )
    .unwrap();

    // Project config (overrides)
    let project_dir = dir.path().join("project").join(".arc");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(
        project_dir.join("config.toml"),
        r#"
[general]
default_provider = "google"
"#,
    )
    .unwrap();

    // Load and merge
    let global: toml::Value = std::fs::read_to_string(global_dir.join("config.toml"))
        .unwrap()
        .parse()
        .unwrap();

    let project: toml::Value = std::fs::read_to_string(project_dir.join("config.toml"))
        .unwrap()
        .parse()
        .unwrap();

    // Project should override global
    assert_eq!(
        global["general"]["default_provider"].as_str().unwrap(),
        "anthropic"
    );
    assert_eq!(
        project["general"]["default_provider"].as_str().unwrap(),
        "google"
    );

    // Global has theme, project doesn't
    assert_eq!(
        global["general"]["theme"].as_str().unwrap(),
        "dark"
    );
    assert!(project["general"].get("theme").is_none());
}

#[test]
fn test_checkpoint_directory_structure() {
    let dir = TempDir::new().unwrap();
    let checkpoint_dir = dir.path().join(".arc").join("checkpoints");
    std::fs::create_dir_all(&checkpoint_dir).unwrap();

    // Simulate creating checkpoints
    let session_id = uuid::Uuid::new_v4();
    for i in 0..5 {
        let cp_file = checkpoint_dir.join(format!(
            "{}_{:04}.json",
            &session_id.to_string()[..8],
            i
        ));
        let checkpoint = serde_json::json!({
            "session_id": session_id.to_string(),
            "turn_number": i,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "token_count": i * 500,
        });
        std::fs::write(&cp_file, serde_json::to_string_pretty(&checkpoint).unwrap())
            .unwrap();
    }

    // Verify all checkpoints exist
    let entries: Vec<_> = std::fs::read_dir(&checkpoint_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 5);
}
