# Assurance Case

This document provides the security assurance case for code-analyze-mcp.

## What the tool does

code-analyze-mcp is a static analysis server that parses source files using tree-sitter and returns structured code metadata (functions, classes, imports, call graphs) over the MCP protocol. **Analyzed code is never executed.** The server performs read-only operations on local files.

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
