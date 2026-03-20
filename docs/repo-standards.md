# Repository Standards

This document maps every repo-level artifact to its purpose and the rationale behind non-obvious decisions. It is the checklist for replicating these standards in a sibling repo. For component design see [ARCHITECTURE.md](ARCHITECTURE.md); for MCP tool design principles see the [MCP agents orchestration guide](https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/anthropic-mcp-agents-orchestration.md).

## Artifact Map

| Artifact | Purpose |
|---|---|
| `.github/ISSUE_TEMPLATE/bug.md` | Structured bug reports; includes root cause hypothesis field to accelerate triage |
| `.github/ISSUE_TEMPLATE/feature.md` | Feature requests with scope boundary section to prevent creep |
| `.github/ISSUE_TEMPLATE/refactor.md` | Tracks refactors as first-class work, not hidden in feature PRs |
| `.github/PULL_REQUEST_TEMPLATE.md` | Verification checklist: tests, clippy, fmt, no-unwrap, API verification, GPG+DCO |
| `.github/copilot-instructions.md` | Repo context for Copilot agents |
| `.github/workflows/ci.yml` | Lint, test, bench, audit; path-filtered; aggregate `CI Result` job |
| `.github/workflows/build-and-attest.yml` | Reusable multi-platform build with cosign signing and provenance attestation |
| `.github/workflows/release.yml` | GPG tag verification, SBOM, Homebrew + cargo-binstall + crates.io distribution |
| `.github/workflows/mcp-scan.yml` | LLM-based MCP security scan (Claude Haiku); runs on push to main and release tags only |
| `.commitlintrc.yml` | Enforces Conventional Commits for automated changelog and searchable history |
| `clippy.toml` | Clippy configuration; lints enforced with `-D warnings` in CI |
| `deny.toml` | `cargo deny` configuration for advisory and license checks |
| `renovate.json` | Automated dependency updates via Renovate bot |
| `CONTRIBUTING.md` | Dev setup, commit format, PR process |
| `SECURITY.md` | Vulnerability disclosure policy |
| `Cargo.toml` `[profile.release]` | `opt-level=z`, `lto=true`, `codegen-units=1`, `panic=abort`, `strip=true` for minimal distribution binaries |
| `Cargo.toml` `[profile.ci]` | Inherits release; `lto=false`, `codegen-units=16` for faster CI builds without sacrificing correctness |

## Non-obvious Decisions

**Rulesets over legacy branch protection.** GitHub Rulesets apply consistently across the organization and support conditions the legacy API cannot express. Two rulesets are active: main branch protection (no force push, no deletion, required status on `CI Result`) and release tag protection (`v*.*.*` format, no overwrites).

**Single aggregate CI check.** `ci.yml` ends with a `CI Result` job that depends on all others. GitHub requires only this one check to pass. A single required check is simpler to reason about and eliminates the maintenance cost of keeping the required-checks list in sync with job names.

**Path-based change detection.** Format, lint, test, and bench jobs run only when `src/**`, `Cargo.*`, `tests/**`, or workflow files change. Documentation-only pushes skip expensive jobs and give faster feedback.

**mcp-scan on release only.** Full LLM-based scanning on every PR would slow feedback and create noise for changes that do not affect the security surface. Scanning at release time catches problems before they reach users.

**Provenance attestation.** `build-and-attest.yml` generates a signed attestation via `actions/attest-build-provenance`. Consumers can verify with `gh attestation verify` before installing. Combined with cosign signing and an SBOM, this covers the supply chain end to end.

## Applying to a New Repo

1. **GitHub metadata:** Set topics, copy the 11-label taxonomy (names, colors, descriptions), create the two rulesets.
2. **Templates:** Copy all three issue templates and the PR template; adapt wording to the target domain.
3. **CI:** Copy `ci.yml`; update path filters. Set `CI Result` as the sole required status check. Copy `.commitlintrc.yml`.
4. **Release:** Copy `build-and-attest.yml` and `release.yml`; update distribution channel config.
5. **Security:** Copy `mcp-scan.yml` if the target is an MCP server. Copy `deny.toml` and `SECURITY.md`.
6. **Cargo profiles:** Copy the `[profile.release]` and `[profile.ci]` blocks verbatim.
7. **Docs:** Add `ARCHITECTURE.md` for the target repo; link this document and the orchestration guide from README.

## Security Scanning

The project uses the Cisco AI Defense `mcp-scanner` to detect potential security issues.

**What it checks:**

- **YARA signatures**: Detects known malware patterns and suspicious code patterns
- **LLM behavioral analysis**: Uses Claude Haiku to analyze code for security-sensitive behaviors and logic flaws
- **Source behavioral analysis**: Identifies suspicious patterns in source code without requiring external APIs

**CI integration:** The scan runs on push to `main` and on release tags (`v*.*.*`). It is advisory and does not block merges.

**Running locally:**

```bash
# YARA + behavioral (no API key required)
mcp-scanner --analyzers yara,behavioral --source-path src/ --format summary stdio --stdio-command cargo --stdio-arg run

# LLM analyzer (requires ANTHROPIC_API_KEY)
export MCP_SCANNER_LLM_API_KEY="$ANTHROPIC_API_KEY"
export MCP_SCANNER_LLM_MODEL="anthropic/claude-haiku-4-5-20251001"
mcp-scanner --analyzers llm --llm-timeout 30 --format summary stdio --stdio-command cargo --stdio-arg run
```

Install via pip:

```bash
pip install cisco-ai-mcp-scanner==4.3.0
```

**Analyzers:**

- **YARA**: Signature-based detection. Matches code against known malware patterns. No API key required.
- **LLM**: Behavioral analysis via Claude. Examines code logic for potential security flaws. Requires `ANTHROPIC_API_KEY`.
- **Behavioral**: Source code pattern analysis. Detects suspicious control flow and unsafe operations statically. No API key required.
