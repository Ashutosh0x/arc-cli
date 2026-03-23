// SPDX-License-Identifier: MIT
//! Built-in security hook presets that ship as default hooks.
//!
//! These implement the 9+ security patterns that Claude Code's PreToolUse scanner checks,
//! but as inspectable, configurable Rust code rather than opaque plugins.

use crate::config::{HookAction, HookDefinition, HooksConfig};
use crate::matcher::HookMatcher;

/// Generate the default security hooks configuration.
/// These are automatically loaded unless explicitly disabled.
pub fn default_security_hooks() -> HooksConfig {
    let mut config = HooksConfig::default();

    // 1. Block writes to sensitive files
    config.hooks.insert(
        "security:block-sensitive-writes".into(),
        HookDefinition {
            description: "Block writes to .env, .git/, and production config files".into(),
            matcher: HookMatcher::new("PreToolUse", Some("^(file_write|file_edit|write|patch)$".into())),
            action: HookAction::Command {
                command: r#"
                    INPUT=$(cat)
                    TARGET=$(echo "$INPUT" | grep -o '"target_path":"[^"]*"' | head -1 | cut -d'"' -f4)
                    
                    # Block patterns
                    if echo "$TARGET" | grep -qE '\.env($|\.)'; then
                        echo "BLOCKED: Cannot write to .env files" >&2
                        exit 2
                    fi
                    if echo "$TARGET" | grep -qE '\.git/'; then
                        echo "BLOCKED: Cannot write to .git/ directory" >&2
                        exit 2
                    fi
                    if echo "$TARGET" | grep -qE '(production|prod)\.(json|yaml|yml|toml|conf)$'; then
                        echo "BLOCKED: Cannot write to production config files" >&2
                        exit 2
                    fi
                    if echo "$TARGET" | grep -qE '(id_rsa|id_ed25519|\.pem|\.key)$'; then
                        echo "BLOCKED: Cannot write to private key files" >&2
                        exit 2
                    fi
                    
                    exit 0
                "#.into(),
                working_directory: None,
                env: Default::default(),
            },
            timeout_ms: 2000,
            enabled: true,
            installed_by_plugin: Some("arc-security-defaults".into()),
            priority: 10, // High priority — runs first
        },
    );

    // 2. Scan for command injection patterns
    config.hooks.insert(
        "security:command-injection-scanner".into(),
        HookDefinition {
            description: "Scan bash commands for injection patterns".into(),
            matcher: HookMatcher::new("PreToolUse", Some("^(bash|shell|command|exec)$".into())),
            action: HookAction::Command {
                command: r#"
                    INPUT=$(cat)
                    CMD=$(echo "$INPUT" | grep -o '"command":"[^"]*"' | head -1 | cut -d'"' -f4)
                    
                    # Pattern 1: Command injection via backticks or $()
                    if echo "$CMD" | grep -qE '`.*`|\$\(.*\)' | grep -qvE '^(echo|printf)'; then
                        # Allow simple variable substitutions but flag complex ones
                        if echo "$CMD" | grep -qE '`(curl|wget|nc|bash|sh|python|ruby|perl)'; then
                            echo "BLOCKED: Potential command injection detected in: $CMD" >&2
                            exit 2
                        fi
                    fi
                    
                    # Pattern 2: Pipe to shell
                    if echo "$CMD" | grep -qE '\|\s*(ba)?sh(\s|$)'; then
                        echo "BLOCKED: Piping to shell detected: $CMD" >&2
                        exit 2
                    fi
                    
                    # Pattern 3: curl | bash patterns
                    if echo "$CMD" | grep -qE 'curl.*\|.*sh'; then
                        echo "BLOCKED: curl-pipe-to-shell pattern detected" >&2
                        exit 2
                    fi
                    
                    # Pattern 4: Dangerous rm patterns
                    if echo "$CMD" | grep -qE 'rm\s+(-rf?|--recursive)\s+(/|~|\$HOME|\$\{HOME\})'; then
                        echo "BLOCKED: Dangerous recursive delete detected" >&2
                        exit 2
                    fi
                    
                    exit 0
                "#.into(),
                working_directory: None,
                env: Default::default(),
            },
            timeout_ms: 2000,
            enabled: true,
            installed_by_plugin: Some("arc-security-defaults".into()),
            priority: 10,
        },
    );

    // 3. Scan for dangerous code patterns in file writes
    config.hooks.insert(
        "security:dangerous-code-scanner".into(),
        HookDefinition {
            description: "Scan written code for eval(), os.system(), pickle, and XSS patterns".into(),
            matcher: HookMatcher::new("PreToolUse", Some("^(file_write|file_edit|write|patch)$".into())),
            action: HookAction::Command {
                command: r#"
                    INPUT=$(cat)
                    CONTENT=$(echo "$INPUT" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    content = data.get('tool_input', {})
    if isinstance(content, dict):
        print(content.get('content', content.get('new_content', '')))
    else:
        print(str(content))
except:
    print('')
" 2>/dev/null || echo "")
                    
                    VIOLATIONS=""
                    
                    # Pattern 1: eval() usage
                    if echo "$CONTENT" | grep -qE '\beval\s*\('; then
                        VIOLATIONS="${VIOLATIONS}eval() usage detected; "
                    fi
                    
                    # Pattern 2: os.system() / subprocess with shell=True
                    if echo "$CONTENT" | grep -qE '(os\.system|subprocess.*shell\s*=\s*True)'; then
                        VIOLATIONS="${VIOLATIONS}os.system/shell=True detected; "
                    fi
                    
                    # Pattern 3: pickle.loads (deserialization)
                    if echo "$CONTENT" | grep -qE 'pickle\.(loads?|Unpickler)'; then
                        VIOLATIONS="${VIOLATIONS}pickle deserialization detected; "
                    fi
                    
                    # Pattern 4: innerHTML / dangerouslySetInnerHTML
                    if echo "$CONTENT" | grep -qE '(innerHTML|dangerouslySetInnerHTML|v-html)'; then
                        VIOLATIONS="${VIOLATIONS}XSS-prone HTML injection detected; "
                    fi
                    
                    # Pattern 5: SQL injection patterns
                    if echo "$CONTENT" | grep -qE "f['\"].*SELECT.*\{|\.format\(.*SELECT|%s.*SELECT"; then
                        VIOLATIONS="${VIOLATIONS}Potential SQL injection (use parameterized queries); "
                    fi
                    
                    # Pattern 6: Hardcoded secrets
                    if echo "$CONTENT" | grep -qEi '(password|secret|api_key|token)\s*=\s*["\x27][^"\x27]{8,}'; then
                        VIOLATIONS="${VIOLATIONS}Possible hardcoded secret detected; "
                    fi
                    
                    if [ -n "$VIOLATIONS" ]; then
                        echo "BLOCKED: Security violations: $VIOLATIONS" >&2
                        exit 2
                    fi
                    
                    exit 0
                "#.into(),
                working_directory: None,
                env: Default::default(),
            },
            timeout_ms: 5000,
            enabled: true,
            installed_by_plugin: Some("arc-security-defaults".into()),
            priority: 15,
        },
    );

    // 4. Block git push --force to protected branches
    config.hooks.insert(
        "security:block-force-push".into(),
        HookDefinition {
            description: "Block git push --force to main/master/production branches".into(),
            matcher: HookMatcher::new("PreToolUse", Some("^(bash|shell|command|git)$".into())),
            action: HookAction::Command {
                command: r#"
                    INPUT=$(cat)
                    CMD=$(echo "$INPUT" | grep -o '"command":"[^"]*"' | head -1 | cut -d'"' -f4)
                    
                    if echo "$CMD" | grep -qE 'git\s+push.*--force|git\s+push.*-f'; then
                        if echo "$CMD" | grep -qE '(main|master|production|release)'; then
                            echo "BLOCKED: Force push to protected branch detected" >&2
                            exit 2
                        fi
                    fi
                    
                    exit 0
                "#.into(),
                working_directory: None,
                env: Default::default(),
            },
            timeout_ms: 2000,
            enabled: true,
            installed_by_plugin: Some("arc-security-defaults".into()),
            priority: 10,
        },
    );

    // 5. Auto-format after file edits (PostToolUse)
    config.hooks.insert(
        "quality:auto-format".into(),
        HookDefinition {
            description: "Run formatter on modified files after edits".into(),
            matcher: HookMatcher::new("PostToolUse", Some("^(file_write|file_edit|write|patch)$".into())),
            action: HookAction::Command {
                command: r#"
                    INPUT=$(cat)
                    FILES=$(echo "$INPUT" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    for f in data.get('payload', {}).get('modified_files', []):
        print(f)
except:
    pass
" 2>/dev/null)
                    
                    for FILE in $FILES; do
                        EXT="${FILE##*.}"
                        case "$EXT" in
                            rs)
                                rustfmt "$FILE" 2>/dev/null || true
                                ;;
                            py)
                                black "$FILE" --quiet 2>/dev/null || ruff format "$FILE" 2>/dev/null || true
                                ;;
                            ts|tsx|js|jsx)
                                npx prettier --write "$FILE" 2>/dev/null || true
                                ;;
                            go)
                                gofmt -w "$FILE" 2>/dev/null || true
                                ;;
                        esac
                    done
                    
                    exit 0
                "#.into(),
                working_directory: None,
                env: Default::default(),
            },
            timeout_ms: 15000,
            enabled: false, // Opt-in
            installed_by_plugin: Some("arc-quality-defaults".into()),
            priority: 200, // Low priority — runs after security hooks
        },
    );

    config
}

/// Generate a TOML string of the default security hooks for user inspection.
pub fn default_security_hooks_toml() -> String {
    let config = default_security_hooks();
    toml::to_string_pretty(&config).unwrap_or_default()
}
