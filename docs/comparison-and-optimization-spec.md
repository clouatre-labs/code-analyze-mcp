# code-analyze-mcp: Comparison and Optimization Spec

**Date:** 2026-03-04 (updated 2026-03-04)
**Status:** Active

## Executive Summary

This document compares code-analyze-mcp against Goose's built-in analyzer and establishes an optimization roadmap. code-analyze-mcp is a standalone MCP server supporting 6 languages (Rust, Python, Go, Java, TypeScript, TSX -- TypeScript and TSX use distinct grammars but share the same queries). Goose's analyzer is a platform extension supporting 10 languages with superior call graph resolution and inheritance extraction, but lacks caching, progress reporting, and portability. code-analyze-mcp already implements cycle detection, compact output notation (F:, C:, I:), caching, progress reporting, and cancellation. Our strategy: close language depth gaps (IMPORT and REFERENCE queries), improve call graph resolution, and leverage our architectural advantages (standalone MCP, caching, progressive output) to become the preferred code analysis server across all MCP clients.

Goose analyzer analysis based on [`block/goose` at 03b5bbb0](https://github.com/block/goose/tree/03b5bbb0/crates/goose/src/agents/platform_extensions/analyze) (2026-03-04). MCP best practices based on the [MCP specification](https://spec.modelcontextprotocol.io/).

## 1. Competitive Analysis

### 1.1 Architecture

| Dimension | code-analyze-mcp | Goose Analyzer |
|-----------|------------------|----------------|
| Integration | Standalone MCP server binary | Platform extension (goose-only) |
| Reusability | Any MCP client (goose, fast-agent, mcp-agent, Claude Code, Cursor) | Goose only |
| Transport | stdio (Streamable HTTP: P1) | Goose IPC |
| Caching | LRU cache (100 entries, mtime-based invalidation) | None |
| Parallelism | rayon par_iter with thread-local parsers | rayon par_iter, new parser per file |
| Languages (implemented) | 6 (Rust, Python, Go, Java, TypeScript, TSX) | 10 (Rust, Python, JS, TS, TSX, Go, Java, Kotlin, Swift, Ruby) |

*Table 1: Architecture comparison between code-analyze-mcp and Goose's built-in analyzer.*

### 1.2 Feature Matrix

| Feature | code-analyze-mcp | Goose Analyzer |
|---------|------------------|----------------|
| Overview mode | Yes | Yes |
| FileDetails mode | Yes | Yes |
| SymbolFocus / call graphs | Yes (BFS, string-based resolution) | Yes (BFS, multi-strategy resolution) |
| Cycle detection | Yes (visited set in BFS) | Yes |
| Caching | Yes (LRU, mtime-based) | No |
| Progress reporting | Yes (atomic counters, ProgressToken) | No |
| Cancellation support | Yes (CancellationToken) | No |
| Output size management | 1000-line warning + force override | Hard 50KB reject |
| Compact output notation | Yes (F:, C:, I: notation) | Yes (F:, C:, I:, bullet-N) |
| Tool annotations | Yes (read_only_hint) | No |
| MCP logging | Yes (tracing bridge) | No |
| Completions | Yes | No |
| Inheritance extraction | No | Yes (7 languages) |
| Test/prod partitioning | No | Yes (8 language patterns) |

*Table 2: Feature matrix comparison.*

### 1.3 Language Support Detail

| Language | Queries Implemented | Missing Queries | Notes |
|----------|---------------------|-----------------|-------|
| Rust | ELEMENT, CALL, REFERENCE, IMPORT, IMPL + 3 helpers | None (complete) | Full language support |
| Python | ELEMENT, CALL | REFERENCE, IMPORT | Uses generic @function/@class captures |
| Go | ELEMENT, CALL, REFERENCE | IMPORT | REFERENCE_QUERY implemented; IMPORT_QUERY missing |
| Java | ELEMENT, CALL | REFERENCE, IMPORT | Uses generic @function/@class captures |
| TypeScript | ELEMENT, CALL | REFERENCE, IMPORT | Uses generic @function/@class captures (design choice) |

*Table 3: Language support detail and gap analysis.*

### 1.4 MCP Compliance

| Capability | code-analyze-mcp | Goose Analyzer |
|------------|------------------|----------------|
| Tool design | Good (parameterized, schemars descriptions) | Good (JsonSchema) |
| Annotations | Good (read_only_hint=true) | None |
| Error handling | Good (thiserror, ErrorData) | Good (anyhow) |
| Progress/Cancellation | Good (atomic counters, tokens) | None |
| Logging | Good (tracing-to-MCP bridge) | None |
| Resources/Prompts | Not implemented | Not implemented |
| Transport | stdio | Goose IPC only |

*Table 4: MCP compliance comparison.*

## 2. Where We Already Win

- **Standalone MCP server**: portable binary works with any MCP client; goose analyzer is locked to goose
- **Caching**: LRU cache with mtime invalidation avoids re-parsing unchanged files; goose re-parses everything every call
- **Progress reporting**: atomic counters with MCP ProgressToken; goose has none
- **Cancellation**: CancellationToken for graceful shutdown; goose ignores cancellation
- **MCP logging**: tracing bridge sends structured logs to MCP client; goose has no MCP logging
- **Tool annotations**: read_only_hint=true signals safe operation; goose has no annotations
- **Completions**: parameter completion support; goose has none
- **Progressive output**: 1000-line warning with force override; goose hard-rejects at 50KB
- **Typed errors**: thiserror with ParserError, AnalyzeError, GraphError; goose uses anyhow
- **Thread-local parsers**: rayon with thread-local Parser reuse; goose creates new parser per file

## 3. Where Goose Wins (Gaps to Close)

- **Language depth**: goose extracts imports, inheritance, decorators, constructors for all languages; our non-Rust languages only extract elements and calls
- **Call graph resolution**: multi-strategy (same-file preference, line-proximity heuristics, cross-file with language filtering); ours uses string matching
- **Inheritance extraction**: 7 languages with syntax-specific logic; we have none
- **Test/prod partitioning**: separates test files from production in output; we do not
- **Field extraction**: extracts struct/class fields for Rust, Go, Java, Kotlin; we do not

## 4. MCP Best Practices Audit

### 4.1 Practices We Follow

- Tool annotations: read_only_hint=true on all tools
- Typed error handling with descriptive messages via ErrorData
- Progress reporting with ProgressToken and atomic counters
- Cancellation support via CancellationToken
- Structured logging with tracing-to-MCP bridge
- schemars descriptions on all parameter fields
- Single focused tool (analyze) rather than tool proliferation
- .gitignore-aware directory walking
- Parallel processing with rayon

### 4.2 Practices We Must Adopt

- **Output schema for tool results**: add outputSchema to tool definitions for client-side validation
- **Structured content in responses**: use structuredContent field in CallToolResult for richer client integration
- **Tool title field**: add human-readable display name for client discovery
- **Capability declaration**: declare tools capability during initialize
- **List changed notifications**: implement notifications/tools/list_changed for dynamic updates
- **Pagination**: implement cursor-based pagination with nextCursor tokens (MCP standard pattern)
- **Summary-first responses**: return high-level summary with optional detail expansion
- **Performance documentation**: document expected execution time for different codebase sizes
- **Cross-client testing**: verify against Claude Code, Cursor, fast-agent, mcp-agent

### 4.3 Anti-Patterns to Avoid

| Anti-Pattern | Our Risk | Mitigation |
|--------------|----------|------------|
| Tool proliferation | Low (single tool) | Keep analyze as single parameterized tool |
| Verbose output | Medium (tree format) | Implement compact notation mode |
| Blocking without progress | Low (implemented) | Already have progress reporting |
| Missing annotations | Low (implemented) | Already have read_only_hint |
| Ignoring transport limits | Medium (stdio only) | Add HTTP transport for remote deployment |
| Inconsistent errors | Low (typed errors) | Already use thiserror + ErrorData |

*Table 5: MCP anti-patterns, current risk level, and mitigations.*

## 5. Cross-Client Compatibility Strategy

code-analyze-mcp must work seamlessly across Goose, fast-agent, mcp-agent, Claude Code, Cursor, and others.

### Transport

- **stdio**: primary transport for local clients (Claude Code, Cursor)
- **Streamable HTTP**: fully specified in MCP 2025-06-18; enables remote deployment and multi-client access. Implementation is P1 priority.
- Bridge servers (mcp-remote, super-gateway) can convert between transports

### Schema

- Use standard JSON Schema via schemars for all parameters
- Include `#[schemars(description = "...")]` on every field
- Test schema rendering across clients (some clients display descriptions differently)

### Compatibility Risks

- Tool context pollution: MCP servers with many tools consume significant context window
- Configuration syntax varies between clients
- Timeout handling varies; some clients have strict limits
- Authentication requirements differ (OAuth 2.0 for remote, none for local stdio)

### Testing Matrix

| Client | Transport | Status | Test Method |
|--------|-----------|--------|-------------|
| Raw stdio (JSON-RPC) | stdio | Tested | McpStdioClient helper; test_initialize, test_tools_list, test_analyze_overview, test_analyze_file_details, test_analyze_error_invalid_path |
| MCP Inspector CLI | stdio | Tested | npx @modelcontextprotocol/inspector --cli; test_inspector_tools_list, test_inspector_analyze_file |
| Goose CLI | stdio | Tested | goose run --with-extension; test_goose_tool_discovery, test_goose_analyze_file |
| Claude Code | stdio | Untested, should work | Requires IDE extension |
| Cursor | stdio | Untested | Requires IDE extension |
| fast-agent | stdio/HTTP | Untested | Requires separate HTTP transport |
| mcp-agent | stdio/HTTP | Untested | Requires separate HTTP transport |

*Table 6: Cross-client compatibility testing matrix.*

## 6. Completed Items

The following features are already implemented and should not be treated as future work:

- **Cycle detection**: visited set in BFS traversal prevents infinite recursion on recursive functions
- **Compact output notation**: F:, C:, I: notation implemented in src/formatter.rs; saves ~30% tokens
- **LRU caching**: mtime-based invalidation persists across MCP calls within a session
- **Progress reporting**: atomic counters with ProgressToken for long-running operations
- **Cancellation support**: CancellationToken infrastructure for graceful shutdown
- **MCP logging**: tracing bridge sends structured logs to MCP client
- **Criterion benchmarks**: performance baselines established for regression detection

## 7. Optimization Roadmap

### 7.1 Performance Optimizations

1. **Implement Streamable HTTP transport** (P1, Medium): fully specified in MCP 2025-06-18; enables remote deployment and multi-client access.
2. **Optimize parser reuse** (P2, Low): verify thread-local parser pattern is optimal; benchmark against goose's new-parser-per-file approach.
3. **Implement incremental analysis** (P2, High): delta detection for changed files only; avoid re-analyzing entire directory on every call.
4. **Feature flags for languages** (P3, Low): compile with only needed grammar crates to reduce binary size.

### 7.2 Output Quality Optimizations

1. **Add test/prod partitioning** (P1, Medium): separate test files from production code in output. Helps LLMs focus on relevant code.
2. **Implement summary-first responses** (P1, Medium): return high-level summary with optional detail expansion for large codebases.
3. **Add outputSchema to tool definitions** (P1, Low): enables client-side validation of structured results.
4. **Add structuredContent to tool responses** (P1, Medium): enables richer client integration beyond text-only.
5. **Add cursor-based pagination** (P2, Medium): nextCursor pattern for large symbol lists and call graphs (MCP standard).
6. **Add tool title field** (P1, Low): human-readable display name for client discovery.
7. **Rich tool descriptions** (P2, Low): use PURPOSE/USAGE/PERFORMANCE/WORKFLOW hierarchy in tool description.

### 7.3 Language Completion

For each partial language, what needs to be added:

**Python** (Medium effort):
- IMPORT_QUERY: import statements, from...import
- REFERENCE_QUERY: type hints, annotations
- Decorator extraction
- Class method vs static method distinction

**Go** (Medium effort):
- IMPORT_QUERY: import declarations
- Type resolution handler (link type_identifier to definitions)
- Interface implementation tracking
- Receiver type extraction for methods

**Java** (Medium effort):
- REFERENCE_QUERY: type references, annotations
- IMPORT_QUERY: import statements
- Constructor handling
- Field extraction
- Annotation processing

**TypeScript/TSX** (Medium effort):
- Evaluate adding @func_name/@class_name named captures for consistency with Python/Java (optional; current @function/@class pattern is valid)
- REFERENCE_QUERY: type references, generics
- IMPORT_QUERY: import/export statements
- Generic type parameter extraction
- Decorator extraction

### 7.4 Features to Surpass Goose

These are areas where our architecture enables capabilities goose cannot match:

1. **Caching across calls**: LRU cache persists across MCP calls within a session; goose re-parses everything every time
2. **Progressive output**: graceful truncation with summary mode vs goose's hard 50KB reject
3. **Cross-client portability**: standalone binary vs platform-locked extension
4. **Cancellation**: graceful shutdown mid-analysis vs goose's fire-and-forget
5. **Progress reporting**: real-time progress updates vs goose's silent processing
6. **Type-aware resolution** (planned): use type information for call graph resolution; goose uses string matching only
7. **Dataflow analysis** (planned): track data dependencies, not just call edges
8. **Macro expansion** (planned): expand Rust macros for deeper analysis

## 8. Prioritized Action Items

### P0 (Must do next)

1. **Add IMPORT_QUERY for Python, Go, Java, TypeScript**: enables import extraction for all partial languages. Medium complexity per language.
2. **Add REFERENCE_QUERY for Python, Java, TypeScript**: enables type reference extraction. Medium complexity per language. (Go already has REFERENCE_QUERY.)
3. **Add test/prod partitioning**: separate test files from production in output. Medium complexity.

### P1 (Should do soon)

4. **Implement Streamable HTTP transport**: fully specified in MCP 2025-06-18; enables remote deployment and multi-client access. Medium complexity.
5. **Implement summary-first responses**: high-level summary with optional detail for large codebases. Medium complexity.
6. **Add outputSchema to tool definitions**: enables client-side validation. Low complexity.
7. **Add structuredContent to tool responses**: richer client integration. Medium complexity.
8. **Add cursor-based pagination**: nextCursor pattern for large symbol lists and call graphs. Medium complexity.

### P2 (Nice to have)

9. **Improve call graph resolution**: adopt multi-strategy approach matching Goose's same-file preference + line-proximity heuristics, then extend with type-aware resolution. High complexity.
10. **Add inheritance extraction**: language-specific logic for class hierarchies. Medium complexity per language.
11. **Cross-client compatibility testing**: verify against Claude Code, Cursor, fast-agent. Low complexity.
12. **Add tool title field and list_changed notifications**: improve client discovery and dynamic updates. Low complexity.

### P3 (Future)

13. **Type-aware call resolution**: use type information for disambiguation. High complexity.
14. **Dataflow analysis**: track variable dependencies. High complexity.
