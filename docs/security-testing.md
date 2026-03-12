# Security Testing

## MCP Scanner

The code-analyze-mcp project uses the Cisco AI Defense `mcp-scanner` to detect potential security issues in the codebase.

**What it checks:**

- **YARA signatures**: Detects known malware patterns and suspicious code patterns
- **LLM behavioral analysis**: Uses Claude Haiku to analyze code for security-sensitive behaviors and logic flaws
- **Source behavioral analysis**: Identifies suspicious patterns in source code without requiring external APIs

## CI Integration

The MCP security scan runs as an advisory workflow on every push to `main` and every pull request. The workflow is separate from the main CI pipeline and does not block merges. A badge in the README indicates the scan status.

## Running Locally

To run all three analyzers locally, install `cisco-ai-mcp-scanner` and execute:

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

## Fork PRs

When mcp-scanner runs on pull requests from forks, GitHub Actions does not expose repository secrets to the forked workflow for security reasons. As a result:

- **YARA analyzer**: Runs (no API key needed)
- **Behavioral analyzer**: Runs (no API key needed)
- **LLM analyzer**: Skipped (requires `ANTHROPIC_API_KEY`)

The workflow will still provide meaningful security feedback via YARA and behavioral analysis.

## Analyzers

**YARA** - Signature-based detection. Matches code against known malware patterns and suspicious idioms. No external API or authentication required.

**LLM** - Behavioral analysis via Claude. Examines code logic for potential security flaws, unsafe patterns, and high-risk operations. Requires `ANTHROPIC_API_KEY` secret to be set. Configured with a 30-second timeout per operation.

**Behavioral** - Source code pattern analysis. Detects suspicious control flow, resource leaks, and unsafe operations statically. No external API or authentication required.
