# Assurance Case

This document provides the security assurance case for aptu-coder.

## What the tool does

aptu-coder is a static analysis server that parses source files using tree-sitter and returns structured code metadata (functions, classes, imports, call graphs) over the MCP protocol. **Analyzed code is never executed.** The server performs read-only operations on local files.

## Trust boundaries

| Boundary | Direction | Description |
|---|---|---|
| Local file system | Inbound | Source files provided by the MCP client via `path` arguments |
| stdio (MCP protocol) | Bidirectional | JSON-RPC requests from the client; JSON responses from the server |

The server makes no outbound network calls, holds no credentials, and writes no persistent state. See [ARCHITECTURE.md](ARCHITECTURE.md) for the full data flow.

## Attack surface

The only meaningful attack surface is **malformed or adversarially crafted source files** fed to tree-sitter. tree-sitter performs error recovery by design and never panics or executes the parsed content. The server does not eval, exec, or interpret any analyzed file.

There is no network listener, no database, no deserialization of untrusted network data, and no privilege escalation path.

## Common weaknesses countered

| Weakness | Status |
|---|---|
| SQL injection | Not applicable -- no database |
| Shell injection | Not applicable -- no shell exec |
| Deserialization of untrusted data | Not applicable -- no network input; only local files parsed by tree-sitter |
| Network I/O vulnerabilities | Not applicable -- no network I/O |
| Credential exposure | Not applicable -- no credentials or secrets handled |

## Supply chain hardening

Existing mechanisms (not duplicated here):

- `cargo deny` in CI: audits transitive dependencies for known advisories and license compliance
- Renovate: automated dependency update pull requests
- SLSA provenance: build provenance attestations published alongside each release
- cosign: release artifacts are signed with keyless signing; see [SECURITY.md](../SECURITY.md) for verification instructions
- GPG-signed commits: all commits to main are GPG-signed

## Site hardening

The project website and repository are hosted on GitHub (https://github.com/clouatre-labs/aptu-coder). GitHub enforces the following hardening headers by default, verified with `curl -sI`:

- `strict-transport-security: max-age=31536000; includeSubdomains; preload`
- `x-frame-options: deny`
- `x-content-type-options: nosniff`
- `content-security-policy: default-src 'none'` (comprehensive policy)

The project distribution channels (crates.io, Homebrew tap) are third-party platforms with established security postures; the project has no control over their headers.

## Security review

- **Review date:** 2026-03-29
- **Scope:** Full codebase, trust boundaries, attack surface, and supply chain (as documented in this file)
- **Conclusion:** No exploitable vulnerabilities identified; residual risks documented above
- **Reviewer:** Project maintainer (self-review; acceptable for solo projects under OpenSSF criteria)
