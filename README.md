<div align="center">

# code-analyze-mcp

[![MCP Security Scan](https://img.shields.io/github/actions/workflow/status/clouatre-labs/code-analyze-mcp/mcp-scan.yml?label=mcp-scan&logo=cisco)](https://github.com/clouatre-labs/code-analyze-mcp/actions/workflows/mcp-scan.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/protocol-MCP-purple.svg)](https://modelcontextprotocol.io)

Standalone MCP server for code structure analysis using tree-sitter.

</div>

> [!NOTE]
> Native agent tools (regex search, path matching, file reading) handle targeted lookups well. `code-analyze-mcp` handles the mechanical, non-AI work: mapping directory structure, extracting symbols, and tracing call graphs. Offloading this to a dedicated tool reduces token usage and speeds up coding with better accuracy.

## Overview

code-analyze-mcp is a Model Context Protocol server that analyzes code structure across 5 programming languages. It exposes three tools: `analyze_directory` (file tree with metrics), `analyze_file` (functions, classes, imports from a single file), and `analyze_symbol` (call graph for a named symbol). It integrates with any MCP-compatible orchestrator (Claude Code, Kiro, Fast-Agent, MCP-Agent, and others), minimizing token usage while giving the LLM precise structural context.

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

Add to `.mcp.json` at your project root (shared with your team via version control):

```json
{
  "mcpServers": {
    "code-analyze": {
      "command": "/path/to/code-analyze-mcp",
      "args": []
    }
  }
}
```

Or add via the Claude Code CLI:

```bash
claude mcp add code-analyze /path/to/code-analyze-mcp
```

## Tools

All optional parameters may be omitted. Shared optional parameters across tools:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `summary` | boolean | auto | Compact output; auto-triggers above 50K chars |
| `cursor` | string | -- | Pagination cursor from a previous response's `next_cursor` |
| `page_size` | integer | 100 | Items per page |
| `force` | boolean | false | Bypass output size warning |

### `analyze_directory`

Walks a directory tree, counts lines of code, functions, and classes per file. Respects `.gitignore` rules.

**Required:** `path` *(string)* -- directory to analyze

**Additional optional:** `max_depth` *(integer, default unlimited)* -- recursion limit; use 2-3 for large monorepos

**Example output:**

```
src/                                [328 LOC | F:28 C:5]
  main.rs                           [18 LOC | F:1 C:0]
  lib.rs                            [156 LOC | F:12 C:3]
  parser.rs                         [89 LOC | F:8 C:2]
  formatter.rs                      [65 LOC | F:7 C:0]
  languages/                        [142 LOC | F:19 C:5]
    mod.rs                          [45 LOC | F:5 C:2]
    rust.rs                         [97 LOC | F:14 C:3]

Total: 4 files, 328 LOC, 28 functions, 5 classes
```

```bash
analyze_directory path: /path/to/project
analyze_directory path: /path/to/project max_depth: 2
analyze_directory path: /path/to/project summary: true
```

### `analyze_file`

Extracts functions, classes, imports, and type references from a single file.

**Required:** `path` *(string)* -- file to analyze

**Additional optional:** `ast_recursion_limit` *(integer, default 256)* -- tree-sitter recursion cap for stack safety

**Example output:**

```
FILE: src/lib.rs [156 LOC | F:12 C:3]

CLASSES:
  CodeAnalyzer:20
  SemanticExtractor:45

FUNCTIONS:
  new:27
  analyze:35
  extract:52
  format_content:78
  build_index:89

IMPORTS:
  rmcp (3)
  serde (2)
  tree_sitter (4)
  thiserror (1)

REFERENCES:
  methods: [analyze, extract, format_content]
  types: [AnalysisResult, SemanticData, ParseError]
  fields: [path, mode, language]
```

```bash
analyze_file path: /path/to/file.rs
analyze_file path: /path/to/file.rs page_size: 50
analyze_file path: /path/to/file.rs cursor: eyJvZmZzZXQiOjUwfQ==
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

**Example output:**

```
FOCUS: analyze
DEPTH: 2
FILES: 12 analyzed

DEFINED:
  src/lib.rs:35

CALLERS (incoming):
  main -> analyze [src/main.rs:12]
  <module> -> analyze [src/lib.rs:40]
  process_request -> analyze [src/handler.rs:88]

CALLEES (outgoing):
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

`analyze_file` and `analyze_symbol` append a `NEXT_CURSOR:` line when output is truncated. Pass the token back as `cursor` to fetch the next page.

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
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development workflow, commit conventions, PR checklist
- **[SECURITY.md](SECURITY.md)** - Security policy and vulnerability reporting

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
