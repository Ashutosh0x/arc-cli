# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | ✅ Active support  |
| < 1.0   | ❌ Not supported   |

## Reporting a Vulnerability

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please report vulnerabilities via:

1. **GitHub Security Advisories**: [Report a vulnerability](https://github.com/Ashutosh0x/arc-cli/security/advisories/new)
2. **Email**: Create a private security advisory on GitHub

### What to include

- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Fix release**: Within 2 weeks for critical issues

## Security Measures

ARC CLI implements multiple layers of security:

- `#![forbid(unsafe_code)]` across the entire workspace
- Nightly `cargo audit` + `cargo deny` + `cargo vet` scans
- Environment variable sanitization (15+ secret patterns)
- Landlock syscall filters on Linux
- Shadow workspace isolation for autonomous changes
- Instruction hierarchy to prevent prompt injection
- OS keyring integration with zeroize memory scrubbing

## Supply Chain Security

Dependencies are verified through:
- `cargo-audit` for known CVEs
- `cargo-deny` for license compliance and duplicate detection
- `cargo-vet` for first-party audit trails
- `cargo-auditable` for embedded dependency metadata
- Dependabot for automated weekly updates
