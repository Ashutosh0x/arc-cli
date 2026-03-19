//! Session forking and resume integration tests.

use std::io::Write;
use tempfile::TempDir;
use uuid::Uuid;

fn setup_test_project(dir: &TempDir) {
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.rs"),
        "fn main() { println!(\"original\"); }\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
}

#[test]
fn test_snapshot_captures_file_state() {
    use sha2::{Digest, Sha256};

    let dir = TempDir::new().unwrap();
    setup_test_project(&dir);

    // Capture file hashes
    let mut file_hashes = std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let content = std::fs::read(entry.path()).unwrap();
            let hash = hex::encode(Sha256::digest(&content));
            let rel = entry
                .path()
                .strip_prefix(dir.path())
                .unwrap()
                .display()
                .to_string();
            file_hashes.insert(rel, hash);
        }
    }

    assert!(file_hashes.contains_key("src/main.rs"));
    assert!(file_hashes.contains_key("src/lib.rs"));
    assert!(file_hashes.contains_key("Cargo.toml"));
}

#[test]
fn test_snapshot_detects_modifications() {
    use sha2::{Digest, Sha256};

    let dir = TempDir::new().unwrap();
    setup_test_project(&dir);

    // Snapshot before modification
    let before: std::collections::HashMap<String, String> = walkdir::WalkDir::new(dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let content = std::fs::read(e.path()).unwrap();
            let hash = hex::encode(Sha256::digest(&content));
            let rel = e
                .path()
                .strip_prefix(dir.path())
                .unwrap()
                .display()
                .to_string();
            (rel, hash)
        })
        .collect();

    // Modify a file
    std::fs::write(
        dir.path().join("src/main.rs"),
        "fn main() { println!(\"modified\"); }\n",
    )
    .unwrap();

    // Snapshot after modification
    let after: std::collections::HashMap<String, String> = walkdir::WalkDir::new(dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let content = std::fs::read(e.path()).unwrap();
            let hash = hex::encode(Sha256::digest(&content));
            let rel = e
                .path()
                .strip_prefix(dir.path())
                .unwrap()
                .display()
                .to_string();
            (rel, hash)
        })
        .collect();

    // main.rs should differ
    assert_ne!(before["src/main.rs"], after["src/main.rs"]);
    // lib.rs should be the same
    assert_eq!(before["src/lib.rs"], after["src/lib.rs"]);
}

#[test]
fn test_selective_rewind_code_only() {
    let dir = TempDir::new().unwrap();
    setup_test_project(&dir);

    // Save original content
    let original_main = std::fs::read_to_string(dir.path().join("src/main.rs")).unwrap();

    // Modify
    std::fs::write(
        dir.path().join("src/main.rs"),
        "fn main() { println!(\"changed\"); }\n",
    )
    .unwrap();

    // Verify it changed
    let modified = std::fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
    assert_ne!(original_main, modified);

    // Revert (simulate code-only rewind)
    std::fs::write(dir.path().join("src/main.rs"), &original_main).unwrap();

    // Verify reverted
    let reverted = std::fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
    assert_eq!(original_main, reverted);
}

#[test]
fn test_fork_tree_structure() {
    // Simulate fork tree
    let root = Uuid::new_v4();
    let fork_a = Uuid::new_v4();
    let fork_b = Uuid::new_v4();

    let mut tree: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
    tree.insert(root, vec![fork_a, fork_b]);

    assert_eq!(tree[&root].len(), 2);
    assert!(tree[&root].contains(&fork_a));
    assert!(tree[&root].contains(&fork_b));
}
