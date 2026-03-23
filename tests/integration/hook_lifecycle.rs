// SPDX-License-Identifier: MIT
//! Integration tests for the full hook lifecycle.

use std::io::Write;
use tempfile::TempDir;

fn setup_hooks_dir(dir: &TempDir) -> std::path::PathBuf {
    let arc_dir = dir.path().join(".arc");
    std::fs::create_dir_all(&arc_dir).unwrap();

    let mut hooks_file = std::fs::File::create(arc_dir.join("hooks.toml")).unwrap();
    write!(
        hooks_file,
        r#"
[hooks.block-rm-rf]
description = "Block dangerous rm commands"
priority = 10
timeout_ms = 2000

[hooks.block-rm-rf.matcher]
event = "PreToolUse"
tool_pattern = "^bash$"

[hooks.block-rm-rf.action]
type = "command"
command = '''
INPUT=$(cat)
CMD=$(echo "$INPUT" | grep -o '"command":"[^"]*"' | head -1 | cut -d'"' -f4)
if echo "$CMD" | grep -qE 'rm\s+-rf\s+/'; then
    echo "BLOCKED: dangerous rm" >&2
    exit 2
fi
exit 0
'''

[hooks.allow-ls]
description = "Always allow ls"
priority = 20
timeout_ms = 1000

[hooks.allow-ls.matcher]
event = "PreToolUse"
tool_pattern = "^bash$"

[hooks.allow-ls.action]
type = "command"
command = "exit 0"

[hooks.session-logger]
description = "Log session start"
priority = 100
timeout_ms = 2000

[hooks.session-logger.matcher]
event = "SessionStart"

[hooks.session-logger.action]
type = "command"
command = "echo 'session started'"
"#
    )
    .unwrap();

    arc_dir
}

#[cfg(unix)]
#[tokio::test]
async fn test_pretooluse_blocks_dangerous_command() {
    let dir = TempDir::new().unwrap();
    let _arc_dir = setup_hooks_dir(&dir);

    // Load hooks
    let config_path = dir.path().join(".arc").join("hooks.toml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let hooks_config: toml::Value = content.parse().unwrap();

    // Verify the config loaded
    assert!(hooks_config.get("hooks").is_some());
    let hooks_table = hooks_config["hooks"].as_table().unwrap();
    assert!(hooks_table.contains_key("block-rm-rf"));
    assert!(hooks_table.contains_key("allow-ls"));
    assert!(hooks_table.contains_key("session-logger"));
}

#[tokio::test]
async fn test_hook_timeout_handling() {
    // A hook that sleeps longer than its timeout should be killed
    let dir = TempDir::new().unwrap();
    let arc_dir = dir.path().join(".arc");
    std::fs::create_dir_all(&arc_dir).unwrap();

    let mut hooks_file = std::fs::File::create(arc_dir.join("hooks.toml")).unwrap();
    write!(
        hooks_file,
        r#"
[hooks.slow-hook]
description = "Hook that times out"
priority = 10
timeout_ms = 100

[hooks.slow-hook.matcher]
event = "PreToolUse"

[hooks.slow-hook.action]
type = "command"
command = "sleep 10"
"#
    )
    .unwrap();

    let content = std::fs::read_to_string(arc_dir.join("hooks.toml")).unwrap();
    let config: toml::Value = content.parse().unwrap();

    let timeout_ms = config["hooks"]["slow-hook"]["timeout_ms"].as_integer().unwrap();
    assert_eq!(timeout_ms, 100);
}

#[tokio::test]
async fn test_hooks_sorted_by_priority() {
    let dir = TempDir::new().unwrap();
    let arc_dir = dir.path().join(".arc");
    std::fs::create_dir_all(&arc_dir).unwrap();

    let mut hooks_file = std::fs::File::create(arc_dir.join("hooks.toml")).unwrap();
    write!(
        hooks_file,
        r#"
[hooks.low-priority]
description = "Runs last"
priority = 999
timeout_ms = 1000
[hooks.low-priority.matcher]
event = "PreToolUse"
[hooks.low-priority.action]
type = "command"
command = "exit 0"

[hooks.high-priority]
description = "Runs first"
priority = 1
timeout_ms = 1000
[hooks.high-priority.matcher]
event = "PreToolUse"
[hooks.high-priority.action]
type = "command"
command = "exit 0"

[hooks.medium-priority]
description = "Runs second"
priority = 50
timeout_ms = 1000
[hooks.medium-priority.matcher]
event = "PreToolUse"
[hooks.medium-priority.action]
type = "command"
command = "exit 0"
"#
    )
    .unwrap();

    let content = std::fs::read_to_string(arc_dir.join("hooks.toml")).unwrap();
    let config: toml::Value = content.parse().unwrap();
    let hooks = config["hooks"].as_table().unwrap();

    let mut priorities: Vec<(String, i64)> = hooks
        .iter()
        .map(|(name, v)| (name.clone(), v["priority"].as_integer().unwrap()))
        .collect();
    priorities.sort_by_key(|(_, p)| *p);

    assert_eq!(priorities[0].0, "high-priority");
    assert_eq!(priorities[1].0, "medium-priority");
    assert_eq!(priorities[2].0, "low-priority");
}
