# code-analyze-mcp

Standalone MCP server for code structure analysis using tree-sitter.

## Overview

code-analyze-mcp is a Model Context Protocol server that analyzes code structure across 5 programming languages. It provides three analysis modes: directory overview (file tree with metrics), file-level semantic analysis (functions, classes, imports), and symbol-focused call graphs. Unlike goose's built-in analyze command, this is a standalone binary that can be integrated into any MCP client, with proper TypeScript support, JSX/TSX handling, and language-specific semantic extraction.

## Quick Start

### Build

```bash
cargo build --release
```

The binary is at `target/release/code-analyze-mcp`.

### Configure MCP Client

Add to your MCP client configuration (e.g., Claude Desktop):

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

### Example Usage

```bash
# Directory overview (auto-detected)
analyze path: /path/to/project

# File details (auto-detected)
analyze path: /path/to/file.rs

# Symbol call graph (requires focus parameter)
analyze path: /path/to/project focus: my_function follow_depth: 2
```

## Tool Interface

The `analyze` tool accepts these parameters:

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | File or directory to analyze |
| `max_depth` | integer | No | Directory recursion limit (default: 3) |
| `focus` | string | No | Symbol name for call graph analysis |
| `follow_depth` | integer | No | Call graph traversal depth (default: 2) |
| `ast_recursion_limit` | integer | No | Tree-sitter recursion limit for stack safety |
| `force` | boolean | No | Bypass output size warning (1000 lines) |
| `mode` | string | No | Analysis mode: 'overview', 'file_details', or 'symbol_focus' (auto-detected if not provided) |
| `summary` | boolean | No | Generate compact output. true=force summary, false=force full, unset=auto-detect when output exceeds 50K chars |
| `cursor` | string | No | Opaque pagination cursor token (from previous response's next_cursor) |
| `page_size` | integer | No | Number of items per page (default: 100) |

**Mode Auto-Detection:**
- `focus` provided → Symbol focus mode
- Path is a file → File details mode
- Path is a directory → Directory overview mode

## Output Management

For large codebases, two mechanisms prevent context overflow:

- **Pagination**: File details and symbol focus modes return a `next_cursor` when output is truncated. Pass it back as `cursor` to fetch the next page.
- **Summary mode**: When output exceeds 50K chars, the server auto-compacts results using `(xN)` notation for repeated call chains. Override with `summary: true` (force) or `summary: false` (disable).

## Analysis Modes

### Structure Mode (Directory Overview)

Walks a directory tree and counts lines of code, functions, and classes per file. Respects `.gitignore` and optional `.gooseignore` files.

```
src/
  main.rs [18 | F:1 C:0]
  lib.rs [156 | F:12 C:3]
  parser.rs [420 | F:8 C:2]
  languages/
    rust.rs [89 | F:3 C:0]
```

### Semantic Mode (File Details)

Extracts functions, classes, imports, and type references from a single file.

```
FILE: src/lib.rs [156 LOC, F:12, C:3]

C: CodeAnalyzer:20 SemanticExtractor:45

F: new:27 analyze:35 extract:52 format:78 ...

I: rmcp(3); serde(2); tree_sitter(4)

R: methods[analyze(Path)]; types[AnalysisResult]; fields[path, mode]
```

### Focused Mode (Symbol Call Graph)

Builds a call graph for a symbol and traverses it with configurable depth. Uses sentinel values `<module>` (top-level calls) and `<reference>` (type references).

```
FOCUS: analyze
DEPTH: 2
FILES: 8 analyzed

DEFINED:
  src/lib.rs:35

CALLERS (incoming):
  main -> analyze [src/main.rs:12]
  <module> -> analyze [src/lib.rs:40]

CALLEES (outgoing):
  analyze -> determine_mode [src/analyze.rs:44]
  analyze -> format_output [src/formatter.rs:12] (•2)
```

Functions called >3 times show `(•N)` notation.

## Supported Languages

| Language | Extensions | Status |
|----------|-----------|--------|
| Rust | `.rs` | Implemented |
| Python | `.py` | Implemented |
| TypeScript | `.ts`, `.tsx` | Implemented |
| Go | `.go` | Implemented |
| Java | `.java` | Implemented |

## Current Status

This project is approximately 90% complete. See [issue #1](https://github.com/clouatre-labs/code-analyze-mcp/issues/1) for the full roadmap and wave-based merge plan.

| Wave | Milestone | Status |
|------|-----------|--------|
| 0 | Foundation (CI, community files) | Complete |
| 1 | Tooling (dependencies, guidelines) | Complete |
| 2 | Core Features (semantic mode, languages) | Complete |
| 3 | Call Graphs (symbol focus mode) | Complete |
| 4a | Polish (caching, output limiting) | Complete |
| 4b | MCP Protocol (#42, #43, #44) | Planned |
| 4c | Performance testing (#7) | Planned |

## Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Design goals, module map, data flow, language handler system, caching strategy
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development workflow, commit conventions, PR checklist
- **[SECURITY.md](SECURITY.md)** - Security policy and vulnerability reporting

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
