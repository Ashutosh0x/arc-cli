//! Path traversal attack prevention tests.

use std::path::PathBuf;

fn sanitize_path(base: &std::path::Path, requested: &str) -> Result<PathBuf, String> {
    let requested_path = PathBuf::from(requested);

    // Resolve the full path
    let full_path = if requested_path.is_absolute() {
        requested_path
    } else {
        base.join(&requested_path)
    };

    // Canonicalize to resolve .. and symlinks
    // Note: in production, check if file exists first or use a different approach
    let canonical = full_path
        .canonicalize()
        .unwrap_or_else(|_| full_path.clone());

    // Verify the resolved path is within the base directory
    let canonical_base = base
        .canonicalize()
        .unwrap_or_else(|_| base.to_path_buf());

    if canonical.starts_with(&canonical_base) {
        Ok(canonical)
    } else {
        Err(format!(
            "Path traversal detected: {} escapes {}",
            requested,
            base.display()
        ))
    }
}

fn is_safe_filename(name: &str) -> bool {
    !name.contains("..")
        && !name.contains('\0')
        && !name.starts_with('/')
        && !name.starts_with('\\')
        && !name.contains("://")
}

#[test]
fn test_blocks_dot_dot_traversal() {
    assert!(!is_safe_filename("../../../etc/passwd"));
    assert!(!is_safe_filename("..\\..\\windows\\system32"));
}

#[test]
fn test_blocks_absolute_paths() {
    assert!(!is_safe_filename("/etc/passwd"));
    assert!(!is_safe_filename("\\windows\\system32"));
}

#[test]
fn test_blocks_null_bytes() {
    assert!(!is_safe_filename("file\0.txt"));
}

#[test]
fn test_blocks_url_paths() {
    assert!(!is_safe_filename("file://etc/passwd"));
    assert!(!is_safe_filename("https://evil.com/payload"));
}

#[test]
fn test_allows_normal_paths() {
    assert!(is_safe_filename("src/main.rs"));
    assert!(is_safe_filename("tests/integration/test.rs"));
    assert!(is_safe_filename("Cargo.toml"));
    assert!(is_safe_filename("README.md"));
}

#[test]
fn test_path_sanitization_blocks_escape() {
    let base = std::env::temp_dir().join("arc-test-base");
    std::fs::create_dir_all(&base).unwrap();

    let result = sanitize_path(&base, "../../../etc/passwd");
    assert!(result.is_err());
}

#[test]
fn test_checkpoint_id_validation() {
    // Checkpoint IDs should only contain safe characters
    fn is_valid_checkpoint_id(id: &str) -> bool {
        let re = regex::Regex::new(r"^[a-zA-Z0-9_-]{1,64}$").unwrap();
        re.is_match(id)
    }

    assert!(is_valid_checkpoint_id("checkpoint_001"));
    assert!(is_valid_checkpoint_id("session-abc123"));
    assert!(is_valid_checkpoint_id("a1b2c3d4"));

    assert!(!is_valid_checkpoint_id("../escape"));
    assert!(!is_valid_checkpoint_id("path/traversal"));
    assert!(!is_valid_checkpoint_id(""));
    assert!(!is_valid_checkpoint_id(&"a".repeat(65)));
    assert!(!is_valid_checkpoint_id("has spaces"));
    assert!(!is_valid_checkpoint_id("has;semicolons"));
}
