# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it responsibly.

**Do not open a public issue for security vulnerabilities.**

Instead, please report security issues by emailing the maintainers or by using [GitHub's private vulnerability reporting](https://github.com/piekstra/market-calculations/security/advisories/new).

## Scope

This is a calculation library that processes numeric data locally. It does not:
- Make network requests
- Access the filesystem (beyond what Cargo does at build time)
- Execute external commands
- Handle authentication or authorization

Security concerns most likely to be relevant:
- Numeric overflow or underflow leading to incorrect results
- Panics on unexpected input (denial of service in consuming applications)
- Unsafe code (there is currently none)

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.2.x   | Yes       |
| < 0.2   | No        |
