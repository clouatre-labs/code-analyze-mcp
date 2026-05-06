<p align="center">

<h1 align="center">aptu-coder</h1>

<p align="center">
  <a href="https://crates.io/crates/aptu-coder"><img alt="crates.io" src="https://img.shields.io/crates/v/aptu-coder.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
  <a href="https://slsa.dev"><img alt="SLSA Level 3" src="https://img.shields.io/badge/SLSA-Level%203-green?style=for-the-badge" height="20"></a>
  <a href="https://www.bestpractices.dev/projects/12275"><img alt="OpenSSF Best Practices" src="https://img.shields.io/cii/level/12275?style=for-the-badge" height="20"></a>
</p>

<p align="center">Standalone MCP server for code structure analysis using tree-sitter. OpenSSF silver certified: fewer than 1% of open source projects reach this level.</p>

<!-- mcp-name: io.github.clouatre-labs/aptu-coder -->

> [!NOTE]
> Native agent tools (regex search, path matching, file reading) handle targeted lookups well. `aptu-coder` handles the mechanical, non-AI work: mapping directory structure, extracting symbols, and tracing call graphs. Offloading this to a dedicated tool reduces token usage and speeds up coding with better accuracy.

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

aptu-coder is a Model Context Protocol server that gives AI agents precise structural context about a codebase: directory trees, symbol definitions, and call graphs, without reading raw files. It supports Rust, Python, Go, Java, TypeScript, TSX, Fortran, JavaScript, C/C++, and C#, and integrates with any MCP-compatible orchestrator.

## Supported Languages

All languages are enabled by default. Disable individual languages at compile time via Cargo feature flags.

| Language | Extensions | Feature flag |
|----------|------------|--------------|
| Rust | `.rs` | `lang-rust` |
| Python | `.py` | `lang-python` |
| TypeScript | `.ts` | `lang-typescript` |
| TSX | `.tsx` | `lang-tsx` |
| Go | `.go` | `lang-go` |
| Java | `.java` | `lang-java` |
| Fortran | `.f`, `.f77`, `.f90`, `.f95`, `.f03`, `.f08`, `.for`, `.ftn` | `lang-fortran` |
| JavaScript | `.js`, `.mjs`, `.cjs` | `lang-javascript` |
| C | `.c` | `lang-cpp` |
| C++ | `.cc`, `.cpp`, `.cxx`, `.h`, `.hpp`, `.hxx` | `lang-cpp` |
| C# | `.cs` | `lang-csharp` |

To build with a subset of languages, disable default features and opt in:

```toml
[dependencies]
aptu-coder-core = { version = "*", default-features = false, features = ["lang-rust", "lang-python"] }
```

The current version is published on [crates.io](https://crates.io/crates/aptu-coder-core). Replace `"*"` with the latest version string if you prefer a pinned dependency.

## Installation

### Homebrew (macOS and Linux)

```bash
brew install clouatre-labs/tap/aptu-coder
```

Update: `brew upgrade aptu-coder`

### cargo-binstall (no Rust required)

```bash
cargo binstall aptu-coder
```

### cargo install (requires Rust toolchain)

```bash
cargo install aptu-coder
```

## Quick Start

### Build from source

```bash
cargo build --release
```

The binary is at `target/release/aptu-coder`.

### Configure MCP Client

After installation via brew or cargo, register with the Claude Code CLI:

```bash
claude mcp add --transport stdio aptu-coder -- aptu-coder
```

If you built from source, use the binary path directly:

```bash
claude mcp add --transport stdio aptu-coder -- /path/to/repo/target/release/aptu-coder
```

stdio is intentional: this server runs locally and processes files directly on disk. The low-latency, zero-network-overhead transport matches the use case. Streamable HTTP adds a network hop with no benefit for a local tool.

Or add manually to `.mcp.json` at your project root (shared with your team via version control):

```json
{
  "mcpServers": {
    "aptu-coder": {
      "command": "aptu-coder",
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

**Additional optional:**
- `max_depth` *(integer, default unlimited)* -- recursion limit; use 2-3 for large monorepos
- `git_ref` *(string, optional)* -- Restrict analysis to files changed relative to this git ref (branch, tag, or commit SHA). Empty string or unset means no filtering.



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



```bash
analyze_file path: /path/to/file.rs
analyze_file path: /path/to/file.rs page_size: 50
analyze_file path: /path/to/file.rs cursor: eyJvZmZzZXQiOjUwfQ==
```

### `analyze_module`

Extracts a minimal function/import index from a single file. ~75% smaller output than `analyze_file`. Use when you need function names and line numbers or the import list, without signatures, types, or call graphs. Returns an actionable error if called on a directory path, steering to `analyze_directory`.

**Required:** `path` *(string)* -- file to analyze



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
- `import_lookup` *(boolean, optional)* -- When true, find all files in the directory that import the module named by `symbol`. Mutually exclusive with call-graph mode.
- `git_ref` *(string, optional)* -- Restrict analysis to files changed relative to this git ref (branch, tag, or commit SHA). Empty string or unset means no filtering.
- `def_use` *(boolean, optional)* -- When true, extract definition and use sites for the symbol. The initial response returns callers and callees as usual and includes a cursor that, when followed, pages through `def_use_sites` (each with `kind`, `symbol`, `file`, `line`, `column`, `snippet`, `enclosing_scope`). `def_use_sites` is empty in `structuredContent` until the client follows that cursor into def-use pagination mode.

The tool also returns `structuredContent` with typed arrays for programmatic consumption: `callers` (production callers), `test_callers` (callers from test files), and `callees` (direct callees), each as `Option<Vec<CallChainEntry>>`. A `CallChainEntry` has three fields: `symbol` (string), `file` (string), and `line` (JSON integer; `usize` in the Rust API). These arrays represent depth-1 relationships only; `follow_depth` does not affect them.

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

### `analyze_raw`

Read a file or range of lines from a file. Returns the file content with line numbers. Specify start_line and end_line (1-indexed, inclusive) to read a range; omit for full file.

**Required:** `path` *(string)* -- file to read

**Additional optional:**
- `start_line` *(integer, optional)* -- starting line number (1-indexed, inclusive). Defaults to 1 if omitted.
- `end_line` *(integer, optional)* -- ending line number (1-indexed, inclusive). Defaults to the last line if omitted.

```bash
analyze_raw path: /path/to/file.rs
analyze_raw path: /path/to/file.rs start_line: 1 end_line: 50
analyze_raw path: /path/to/file.rs start_line: 100 end_line: 150
```

### `edit_overwrite`

Create or overwrite a file at path with content. Creates parent directories if needed. Overwrites without confirmation; use `edit_replace` to replace a specific block instead of the whole file.

**Required:**
- `path` *(string)* -- file to create or overwrite
- `content` *(string)* -- UTF-8 content to write

```bash
edit_overwrite path: tests/foo_test.rs content: "..."
edit_overwrite path: src/config.rs content: "..."
```

### `edit_replace`

Replace a unique exact text block in a file. Errors if `old_text` appears zero times or more than once; fix by making `old_text` longer and more specific. Use `edit_overwrite` to replace the whole file.

**Required:**
- `path` *(string)* -- file to edit
- `old_text` *(string)* -- exact text block to find and replace (must appear exactly once)
- `new_text` *(string)* -- replacement text

```bash
edit_replace path: src/main.rs old_text: "..." new_text: "..."
```

### `edit_rename`

AST-aware rename within a single file. Matches only syntactic identifiers -- identifiers in string literals and comments are excluded. Errors if `old_name` not found. Note: the `kind` parameter is reserved for future use; supplying it currently returns an error.

**Required:**
- `path` *(string)* -- file to modify
- `old_name` *(string)* -- current name of the symbol (identifier) to rename
- `new_name` *(string)* -- new name for the symbol

**Additional optional:** `kind` *(string, optional)* -- reserved for future use; currently returns an error if supplied.

```bash
edit_rename path: src/config.rs old_name: parse_config new_name: load_config
edit_rename path: src/client.rs old_name: timeout new_name: timeout_ms
```

### `edit_insert`

Insert content immediately before or after a named AST node. `position` is `before` or `after`. The caller is responsible for including necessary newlines in `content`. Uses the first occurrence if `symbol_name` appears multiple times.

**Required:**
- `path` *(string)* -- file to modify
- `symbol_name` *(string)* -- name of the symbol (identifier) to locate
- `position` *(string)* -- `before` or `after`
- `content` *(string)* -- content to insert verbatim; include leading/trailing newlines as needed

```bash
edit_insert path: src/lib.rs symbol_name: handle_request position: before content: "#[instrument]\n"
edit_insert path: src/types.rs symbol_name: MyStruct position: after content: "\n#[derive(Debug)]\n"
```

### `exec_command`

> [!WARNING]
> This tool executes arbitrary shell commands via `sh -c` (or `$SHELL` if set). The `working_dir` parameter restricts the initial process working directory only -- it does not prevent shell-level escape via `cd` or absolute paths within the command string. Set `open_world_hint=true` in your MCP client configuration to surface this warning.

Run a shell command and return its combined stdout/stderr output. Intended for orchestrator BUILD agents that need to compile, test, or lint without a separate shell tool. Annotations: `destructive_hint=true`, `open_world_hint=true`.

**Required:** `command` *(string)* -- shell command to execute via `sh -c` (or `$SHELL` if set)

**Additional optional:**
- `timeout_secs` *(integer, default 30)* -- seconds before SIGKILL is sent to the process
- `working_dir` *(string, optional)* -- initial working directory for the process; path-validated and relative to the server's CWD. Does not restrict shell-level escapes.

```bash
exec_command command: "cargo test --workspace"
exec_command command: "cargo clippy -- -D warnings" working_dir: "crates/aptu-coder"
exec_command command: "cargo build --release" timeout_secs: 120
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

The server's own instructions expose a 4-step recommended workflow for unknown repositories: survey the repo root with `analyze_directory` at `max_depth=2`, drill into the source package, run `analyze_module` on key files for a function/import index (or `analyze_file` when signatures and types are needed), then use `analyze_symbol` to trace call graphs. MCP clients that surface server instructions will present this workflow automatically to the agent.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `CODE_ANALYZE_FILE_CACHE_CAPACITY` | `100` | Maximum number of file-analysis results held in the in-process LRU cache. Increase for large repos where many files are queried repeatedly. |
| `CODE_ANALYZE_DIR_CACHE_CAPACITY` | `20` | Maximum number of directory-analysis results held in the in-process LRU cache. |
| `DISABLE_PROMPT_CACHING` | unset | Set to `1` to disable prompt caching (recommended for single-pass subagent sessions). |
| `DISABLE_PROMPT_CACHING_HAIKU` | unset | Set to `1` to disable prompt caching for Haiku-specific pipelines only. |

## Observability

All ten tools emit metrics to daily-rotated JSONL files at `$XDG_DATA_HOME/aptu-coder/` (fallback: `~/.local/share/aptu-coder/`). Each record captures tool name, duration, output size, and result status. Files are retained for 30 days. See [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md) for the full schema.

## Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Design goals, module map, data flow, language handler system, caching strategy
- **[MCP Best Practices](docs/MCP-BEST-PRACTICES.md)** - Best practices for agentic loops, orchestration patterns, MCP tool design, memory management, and safety controls
- **[OBSERVABILITY.md](docs/OBSERVABILITY.md)** - Metrics schema, JSONL format, and retention policy
- **[ROADMAP.md](docs/ROADMAP.md)** - Development history and future direction
- **[DESIGN-GUIDE.md](docs/DESIGN-GUIDE.md)** - Design decisions, rationale, and replication guide for building high-performance MCP servers
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development workflow, commit conventions, PR checklist
- **[SECURITY.md](SECURITY.md)** - Security policy and vulnerability reporting

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
