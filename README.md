<div align="center">

# code-analyze-mcp

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024_edition-orange.svg)](https://www.rust-lang.org)
[![crates.io](https://img.shields.io/crates/v/code-analyze-mcp.svg)](https://crates.io/crates/code-analyze-mcp)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12275/badge)](https://www.bestpractices.dev/projects/12275)
[![OpenSSF Silver](https://www.bestpractices.dev/projects/12275/badge?level=silver)](https://www.bestpractices.dev/projects/12275/silver)

Standalone MCP server for code structure analysis using tree-sitter.

</div>
<!-- mcp-name: io.github.clouatre-labs/code-analyze-mcp -->

> [!NOTE]
> Native agent tools (regex search, path matching, file reading) handle targeted lookups well. `code-analyze-mcp` handles the mechanical, non-AI work: mapping directory structure, extracting symbols, and tracing call graphs. Offloading this to a dedicated tool reduces token usage and speeds up coding with better accuracy.

## Benchmarks

Auth migration task on Claude Code against [Django](https://github.com/django/django) (Python) source tree. [Full methodology](docs/benchmarks/v12/methodology.md).

| Mode | Sonnet 4.6 | Haiku 4.5 |
|---|---|---|
| MCP | 112k tokens, $0.39 | 406k tokens, $0.42 |
| Native | 276k tokens, $0.95 | 473k tokens, $0.53 |
| **Savings** | **59% fewer tokens, 59% cheaper** | **14% fewer tokens, 21% cheaper** |

AeroDyn integration audit task on Claude Code against [OpenFAST](https://github.com/OpenFAST/openfast) (Fortran) source tree. [Full methodology](docs/benchmarks/v13/methodology.md).

| Mode | Sonnet 4.6 | Haiku 4.5 |
|---|---|---|
| MCP | 472k tokens, $1.65 | 687k tokens, $0.72 |
| Native | 877k tokens, $2.85 | 2162k tokens, $2.21 |
| **Savings** | **46% fewer tokens, 42% cheaper** | **68% fewer tokens, 68% cheaper** |

## Overview

code-analyze-mcp is a Model Context Protocol server that gives AI agents precise structural context about a codebase: directory trees, symbol definitions, and call graphs, without reading raw files. It supports Rust, Python, Go, Java, TypeScript, TSX, and Fortran, and integrates with any MCP-compatible orchestrator (Claude Code, Kiro, Fast-Agent, MCP-Agent, and others).

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

Walks a directory tree, counts lines of code, functions, and classes per file. Respects `.gitignore` rules. Default output is a flat `PAGINATED` list. Pass `verbose=true` for `FILES` / `TEST FILES` section headers. Pass `summary=true` for a compact `STRUCTURE` tree with aggregate counts.

**Required:** `path` *(string)* -- directory to analyze

**Additional optional:** `max_depth` *(integer, default unlimited)* -- recursion limit; use 2-3 for large monorepos

**Example output (default):**

```
PAGINATED: showing 16 of 16 files (max_depth=1)

analyze.rs [737L, 13F, 4C]
cache.rs [105L, 5F, 2C]
completion.rs [129L, 2F]
formatter.rs [1876L, 32F, 2C]
graph.rs [926L, 34F, 3C]
lang.rs [41L, 3F]
lib.rs [1335L, 22F, 1C]
logging.rs [136L, 11F, 3C]
main.rs [50L, 1F]
metrics.rs [254L, 13F, 3C]
pagination.rs [198L, 11F, 4C]
parser.rs [990L, 19F, 4C]
schema_helpers.rs [56L, 4F]
traversal.rs [90L, 1F, 2C]
types.rs [575L, 8F, 27C]

test_detection.rs [100L, 5F]
```

**Example output (`verbose=true`):**

```
PAGINATED: showing 16 of 16 files (max_depth=1)

FILES [LOC, FUNCTIONS, CLASSES]
analyze.rs [737L, 13F, 4C]
cache.rs [105L, 5F, 2C]
completion.rs [129L, 2F]
formatter.rs [1876L, 32F, 2C]
graph.rs [926L, 34F, 3C]
lang.rs [41L, 3F]
lib.rs [1335L, 22F, 1C]
logging.rs [136L, 11F, 3C]
main.rs [50L, 1F]
metrics.rs [254L, 13F, 3C]
pagination.rs [198L, 11F, 4C]
parser.rs [990L, 19F, 4C]
schema_helpers.rs [56L, 4F]
traversal.rs [90L, 1F, 2C]
types.rs [575L, 8F, 27C]

TEST FILES [LOC, FUNCTIONS, CLASSES]
test_detection.rs [100L, 5F]
```

**Example output (`summary=true`):**

```
SUMMARY:
16 files (15 prod, 1 test), 7598L, 184F, 55C (max_depth=1)
Languages: rust (100%)

STRUCTURE (depth 1):
  analyze.rs [737L, 13F, 4C]
  cache.rs [105L, 5F, 2C]
  completion.rs [129L, 2F]
  formatter.rs [1876L, 32F, 2C]
  graph.rs [926L, 34F, 3C]
  lang.rs [41L, 3F]
  languages/
  lib.rs [1335L, 22F, 1C]
  logging.rs [136L, 11F, 3C]
  main.rs [50L, 1F]
  metrics.rs [254L, 13F, 3C]
  pagination.rs [198L, 11F, 4C]
  parser.rs [990L, 19F, 4C]
  schema_helpers.rs [56L, 4F]
  test_detection.rs [100L, 5F]
  traversal.rs [90L, 1F, 2C]
  types.rs [575L, 8F, 27C]

SUGGESTION:
Use a narrower path for details (e.g., analyze src/core/)
```

```bash
analyze_directory path: /path/to/project
analyze_directory path: /path/to/project max_depth: 2
analyze_directory path: /path/to/project summary: true
analyze_directory path: /path/to/project verbose: true
```

### `analyze_file`

Extracts functions, classes, and imports from a single file.

**Required:** `path` *(string)* -- file to analyze

**Additional optional:**
- `ast_recursion_limit` *(integer, optional)* -- tree-sitter AST traversal depth cap; leave unset for unlimited depth. Minimum value is 1; 0 is treated as unset.
- `fields` *(array of strings, optional)* -- limit output to specific sections. Valid values: `"functions"`, `"classes"`, `"imports"`. Omit to return all sections. The FILE header (path, line count, section counts) is always emitted regardless. Ignored when `summary=true`. When `"imports"` is listed explicitly, the `I:` section is rendered regardless of the `verbose` flag.

**Example output (default, page 1 of 2):**

```
FILE: src/lib.rs (1335L, 1-10/22F, 1C, 66I)
C:
  CodeAnalyzer:143
F:
  summary_cursor_conflict:65, error_meta:69, err_to_tool_result:81,
  no_cache_meta:85, paginate_focus_chains:96, list_tools:154, new:158,
  emit_progress:175, handle_overview_mode:202, handle_file_details_mode:303

NEXT_CURSOR: eyJtb2RlIjoiZGVmYXVsdCIsIm9mZnNldCI6MTB9
```

**Example output (`verbose=true`, adds `I:` section before `F:`):**

```
FILE: src/lib.rs (1335L, 1-10/22F, 1C, 66I)
C:
  CodeAnalyzer:143
I:
  cache(1)
  crate::pagination(2)
  crate::types(3)
  formatter(6)
  pagination(6)
  rmcp(6)
  rmcp::model(19)
  types(5)
F:
  summary_cursor_conflict:65, error_meta:69, err_to_tool_result:81,
  no_cache_meta:85, paginate_focus_chains:96, list_tools:154, new:158,
  emit_progress:175, handle_overview_mode:202, handle_file_details_mode:303

NEXT_CURSOR: eyJtb2RlIjoiZGVmYXVsdCIsIm9mZnNldCI6MTB9
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
FILE: lib.rs (1335L, 22F, 66I)
F:
  summary_cursor_conflict:65, error_meta:69, err_to_tool_result:81,
  no_cache_meta:85, paginate_focus_chains:96, list_tools:154, new:158,
  emit_progress:175, handle_overview_mode:202, handle_file_details_mode:303,
  handle_focused_mode:344, analyze_directory:553, analyze_file:691,
  analyze_symbol:846, analyze_module:998, get_info:1079, on_initialized:1106,
  on_cancelled:1154, complete:1167, set_level:1221
I:
  cache:AnalysisCache; formatter:format_structure_paginated;
  pagination:paginate_slice; rmcp::model:CallToolResult;
  types:AnalyzeDirectoryParams
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
- `ast_recursion_limit` *(integer, optional)* -- tree-sitter AST traversal depth cap; leave unset for unlimited depth. Minimum value is 1; 0 is treated as unset.
- `impl_only` *(boolean, optional)* -- when true, restrict callers to only those originating from an `impl Trait for Type` block (Rust only). Returns `INVALID_PARAMS` if the path contains no `.rs` files. Emits a `FILTER:` header showing how many callers were retained out of total.
- `match_mode` *(string, default exact)* -- Symbol lookup strategy:
  - `exact`: Case-sensitive exact match (default)
  - `insensitive`: Case-insensitive exact match
  - `prefix`: Case-insensitive prefix match; returns an error listing candidates when multiple symbols match
  - `contains`: Case-insensitive substring match; returns an error listing candidates when multiple symbols match
  All non-exact modes return an error with candidate names when the match is ambiguous; use the listed candidates to refine to a unique match.

**Example output:**

```
FOCUS: format_structure_paginated (1 defs, 1 callers, 3 callees)
CALLERS (1-1 of 1):
  format_structure_paginated <- analyze_directory
    <- format_structure_paginated
CALLEES: 3 (use cursor for callee pagination)
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
| Fortran | `.f`, `.f77`, `.f90`, `.f95`, `.f03`, `.f08`, `.for`, `.ftn` | Implemented |

## Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Design goals, module map, data flow, language handler system, caching strategy
- **[MCP, Agents, and Orchestration](docs/anthropic-mcp-agents-orchestration.md)** - Best practices for agentic loops, orchestration patterns, MCP tool design, memory management, and safety controls
- **[OBSERVABILITY.md](docs/OBSERVABILITY.md)** - Metrics schema, JSONL format, and retention policy
- **[ROADMAP.md](docs/ROADMAP.md)** - Development history and future direction
- **[DESIGN-GUIDE.md](docs/DESIGN-GUIDE.md)** - Design decisions, rationale, and replication guide for building high-performance MCP servers
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development workflow, commit conventions, PR checklist
- **[SECURITY.md](SECURITY.md)** - Security policy and vulnerability reporting

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
