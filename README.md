<div align="center">

# code-analyze-mcp

[![MCP Security Scan](https://img.shields.io/github/actions/workflow/status/clouatre-labs/code-analyze-mcp/mcp-scan.yml?label=mcp-scan&logo=cisco)](https://github.com/clouatre-labs/code-analyze-mcp/actions/workflows/mcp-scan.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/protocol-MCP-purple.svg)](https://modelcontextprotocol.io)
[![CI](https://img.shields.io/github/actions/workflow/status/clouatre-labs/code-analyze-mcp/ci.yml?label=ci)](https://github.com/clouatre-labs/code-analyze-mcp/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/code-analyze-mcp.svg)](https://crates.io/crates/code-analyze-mcp)

Standalone MCP server for code structure analysis using tree-sitter.

</div>

> [!NOTE]
> Native agent tools (regex search, path matching, file reading) handle targeted lookups well. `code-analyze-mcp` handles the mechanical, non-AI work: mapping directory structure, extracting symbols, and tracing call graphs. Offloading this to a dedicated tool reduces token usage and speeds up coding with better accuracy.

## Overview

code-analyze-mcp is a Model Context Protocol server that analyzes code structure across 6 registered languages (Rust, Python, Go, Java, TypeScript, and TSX -- TypeScript and TSX use distinct grammars but share the same queries). It exposes four tools: `analyze_directory` (file tree with metrics, test/prod partitioning, and a SUGGESTION footer), `analyze_file` (functions, classes, and imports from a single file), `analyze_module` (lightweight function/import index, ~75% smaller than `analyze_file`), and `analyze_symbol` (call graph for a named symbol). The server implements MCP completions for path autocompletion and emits async JSONL metrics for observability. It integrates with any MCP-compatible orchestrator (Claude Code, Kiro, Fast-Agent, MCP-Agent, and others), minimizing token usage while giving the LLM precise structural context.

## Installation

### Homebrew (macOS and Linux)

```bash
brew install clouatre-labs/tap/code-analyze-mcp
```

Update: `brew upgrade code-analyze-mcp`

### cargo-binstall (no Rust required)

```bash
cargo binstall code-analyze-mcp
```

### cargo install (requires Rust toolchain)

```bash
cargo install code-analyze-mcp
```

## Quick Start

### Build from source

```bash
cargo build --release
```

The binary is at `target/release/code-analyze-mcp`.

### Configure MCP Client

After installation via brew or cargo, register with the Claude Code CLI:

```bash
claude mcp add --transport stdio code-analyze -- code-analyze-mcp
```

If you built from source, use the binary path directly:

```bash
claude mcp add --transport stdio code-analyze -- /path/to/repo/target/release/code-analyze-mcp
```

stdio is intentional: this server runs locally and processes files directly on disk. The low-latency, zero-network-overhead transport matches the use case. Streamable HTTP adds a network hop with no benefit for a local tool.

Or add manually to `.mcp.json` at your project root (shared with your team via version control):

```json
{
  "mcpServers": {
    "code-analyze": {
      "command": "code-analyze-mcp",
      "args": []
    }
  }
}
```

## Tools

All optional parameters may be omitted. Shared optional parameters for `analyze_directory`, `analyze_file`, and `analyze_symbol` (`analyze_module` does not support these):

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `summary` | boolean | auto | Compact output; auto-triggers above 50K chars |
| `cursor` | string | -- | Pagination cursor from a previous response's `next_cursor` |
| `page_size` | integer | 100 | Items per page |
| `force` | boolean | false | Bypass output size warning |
| `verbose` | boolean | false | true = full output with section headers and imports (Markdown-style headers in `analyze_directory`; adds `I:` section in `analyze_file`); false = compact format |

`summary=true` and `cursor` are mutually exclusive. Passing both returns an error.

### `analyze_directory`

Walks a directory tree, counts lines of code, functions, and classes per file. Respects `.gitignore` rules. Output is partitioned into a `PATH` section (production files) and a `TEST FILES` section (test files), preceded by a `SUMMARY:` block with aggregate counts and a `SUGGESTION:` footer naming the largest source subdirectory.

**Required:** `path` *(string)* -- directory to analyze

**Additional optional:** `max_depth` *(integer, default unlimited)* -- recursion limit; use 2-3 for large monorepos

**Example output:**

```
12 files, 843L, 42F, 0C (rust 100%)
SUMMARY:
Shown: 12 files (9 prod, 3 test), 843L, 42F, 0C (max_depth=2)
Languages: rust (100%)

PATH [LOC, FUNCTIONS, CLASSES]
  main.rs [18L, 1F]
  lib.rs [156L, 12F, 3C]
  formatter.rs [210L, 14F]
  languages/
    rust.rs [97L, 8F, 2C]
    python.rs [84L, 7F, 2C]

TEST FILES [LOC, FUNCTIONS, CLASSES]
  formatter_test.rs [143L, 9F]
  languages/
    rust_test.rs [65L, 5F]

SUGGESTION: Largest source directory: src/ (9 files total). For module details, re-run with path=src/ and max_depth=2.
```

```bash
analyze_directory path: /path/to/project
analyze_directory path: /path/to/project max_depth: 2
analyze_directory path: /path/to/project summary: true
```

### `analyze_file`

Extracts functions, classes, and imports from a single file.

**Required:** `path` *(string)* -- file to analyze

**Additional optional:** `ast_recursion_limit` *(integer, default 256)* -- tree-sitter recursion cap for stack safety

**Example output:**

```
FILE: src/lib.rs(156L, 12F, 3C, 5I)
C:
  CodeAnalyzer:20; SemanticExtractor:45; ParseError:88
F:
  new:27, analyze:35, extract:52, format_content:78, build_index:89,
  validate:102, run:115, reset:130
I:
  rmcp(3); serde(2); thiserror(1); tree_sitter(4); tracing(1)
```

```bash
analyze_file path: /path/to/file.rs
analyze_file path: /path/to/file.rs page_size: 50
analyze_file path: /path/to/file.rs cursor: eyJvZmZzZXQiOjUwfQ==
```

### `analyze_module`

Extracts a minimal function/import index from a single file. ~75% smaller output than `analyze_file`. Use when you need function names and line numbers or the import list, without signatures, types, or call graphs. Returns an actionable error if called on a directory path, steering to `analyze_directory`.

**Required:** `path` *(string)* -- file to analyze

**Example output:**

```
FILE: analyze.rs (510L, 3F, 2I)
F:
  analyze_directory:174, analyze_file:200, analyze_module_file:460
I:
  crate::formatter:format_file_details; std::path:Path, PathBuf
```

```bash
analyze_module path: /path/to/file.rs
```

### `analyze_symbol`

Builds a call graph for a named symbol across all files in a directory. Uses sentinel values `<module>` (top-level calls) and `<reference>` (type references). Functions called >3 times show `(•N)` notation.

**Required:**
- `path` *(string)* -- directory to search
- `symbol` *(string)* -- symbol name, case-sensitive exact-match

**Additional optional:**
- `follow_depth` *(integer, default 1)* -- call graph traversal depth
- `max_depth` *(integer, default unlimited)* -- directory recursion limit
- `ast_recursion_limit` *(integer, default 256)* -- tree-sitter recursion cap for stack safety
- `match_mode` *(string, default exact)* -- Symbol lookup strategy:
  - `exact`: Case-sensitive exact match (default)
  - `insensitive`: Case-insensitive exact match
  - `prefix`: Case-insensitive prefix match; returns an error listing candidates when multiple symbols match
  - `contains`: Case-insensitive substring match; returns an error listing candidates when multiple symbols match
  All non-exact modes return an error with candidate names when the match is ambiguous; use the listed candidates to refine to a unique match.

**Example output:**

```
FOCUS: analyze (2 defs, 3 callers, 4 callees)
DEPTH: 2
DEFINED:
  src/lib.rs:35

CALLERS:
  main -> analyze [src/main.rs:12]
  <module> -> analyze [src/lib.rs:40]
  process_request -> analyze [src/handler.rs:88]

CALLEES:
  analyze -> determine_mode [src/analyze.rs:44]
  analyze -> format_output [src/formatter.rs:12] (•2)
  analyze -> validate_params [src/validation.rs:5]
  determine_mode -> is_directory [src/utils.rs:23]
```

```bash
analyze_symbol path: /path/to/project symbol: my_function
analyze_symbol path: /path/to/project symbol: my_function follow_depth: 3
analyze_symbol path: /path/to/project symbol: my_function max_depth: 3 follow_depth: 2
```

## Output Management

For large codebases, two mechanisms prevent context overflow:

**Pagination**

`analyze_file` and `analyze_symbol` append a `NEXT_CURSOR:` line when output is truncated. Pass the token back as `cursor` to fetch the next page. `summary=true` and `cursor` are mutually exclusive; passing both returns an error.

```
# Response ends with:
NEXT_CURSOR: eyJvZmZzZXQiOjUwfQ==

# Fetch next page:
analyze_symbol path: /my/project symbol: my_function cursor: eyJvZmZzZXQiOjUwfQ==
```

**Summary Mode**

When output exceeds 50K chars, the server auto-compacts results using aggregate statistics. Override with `summary: true` (force compact) or `summary: false` (disable).

```bash
# Force summary for large project
analyze_directory path: /huge/codebase summary: true

# Disable summary (get full details, may be large)
analyze_directory path: /project summary: false
```

## Non-Interactive Pipelines

In single-pass subagent sessions, prompt caches are written but never reused. Benchmarks showed MCP responses writing ~2x more to cache than native-only workflows, adding cost with no quality gain. Set `DISABLE_PROMPT_CACHING=1` (or `DISABLE_PROMPT_CACHING_HAIKU=1` for Haiku-specific pipelines) to avoid this overhead.

The server's own instructions expose a 4-step recommended workflow for unknown repositories: survey the repo root with `analyze_directory` at `max_depth=2`, drill into the source package, run `analyze_file` on key files, then use `analyze_symbol` to trace call graphs. MCP clients that surface server instructions will present this workflow automatically to the agent.

## Observability

All four tools emit metrics to daily-rotated JSONL files at `$XDG_DATA_HOME/code-analyze-mcp/` (fallback: `~/.local/share/code-analyze-mcp/`). Each record captures tool name, duration, output size, and result status. Files are retained for 30 days. See [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) for the full schema.

## Supported Languages

| Language | Extensions | Status |
|----------|-----------|--------|
| Rust | `.rs` | Implemented |
| Python | `.py` | Implemented |
| TypeScript | `.ts`, `.tsx` | Implemented |
| Go | `.go` | Implemented |
| Java | `.java` | Implemented |

## Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Design goals, module map, data flow, language handler system, caching strategy
- **[MCP, Agents, and Orchestration](docs/anthropic-mcp-agents-orchestration.md)** - Best practices for agentic loops, orchestration patterns, MCP tool design, memory management, and safety controls
- **[OBSERVABILITY.md](docs/OBSERVABILITY.md)** - Metrics schema, JSONL format, and retention policy
- **[ROADMAP.md](docs/ROADMAP.md)** - Development history and future direction
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development workflow, commit conventions, PR checklist
- **[SECURITY.md](SECURITY.md)** - Security policy and vulnerability reporting

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
