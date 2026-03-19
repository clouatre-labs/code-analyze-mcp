# Architecture

## See Also

- [anthropic-mcp-agents-orchestration.md](anthropic-mcp-agents-orchestration.md) - MCP tool design principles and annotation semantics that informed this server's interface design
- [ROADMAP.md](ROADMAP.md) - Wave history, benchmark-driven development process, small-model-first constraint

## Design Goals

- **Minimize token usage**: Return only structured, relevant context - no prose, no noise
- **Language-agnostic parsing via tree-sitter**: Support 6 languages (Rust, Go, Java, Python, TypeScript, TSX) with a unified query-based extraction system; TypeScript and TSX use distinct grammars (`LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX`) but share the same queries in `src/languages/typescript.rs`
- **Four focused MCP tools**: `analyze_directory`, `analyze_file`, `analyze_module`, and `analyze_symbol` -- each with a clear, explicit interface rather than a single tool with auto-detected modes
- **Compatible with any MCP orchestrator**: Claude Code, Kiro, Fast-Agent, MCP-Agent, and others
- **Performance via parallelism**: Use rayon for parallel file processing and ignore crate for efficient .gitignore-aware directory walking

## Module Map

| Module | File | Responsibility |
|--------|------|-----------------|
| `main` | `src/main.rs` | MCP server entry point; initializes tracing and stdio transport |
| `lib` | `src/lib.rs` | CodeAnalyzer struct; MCP tool handlers for `analyze_directory`, `analyze_file`, `analyze_module`, `analyze_symbol` |
| `analyze` | `src/analyze.rs` | High-level analysis orchestration; directory, file, and module analysis |
| `parser` | `src/parser.rs` | Tree-sitter parsing; ElementExtractor and SemanticExtractor |
| `formatter` | `src/formatter.rs` | Output formatting for all four tools |
| `traversal` | `src/traversal.rs` | Directory walking with .gitignore support via ignore crate |
| `types` | `src/types.rs` | Shared data structures (AnalyzeParams, AnalysisResult, etc.) |
| `lang` | `src/lang.rs` | Extension-to-language mapping |
| `languages/mod` | `src/languages/mod.rs` | LanguageInfo registry and handler function types |
| `languages/rust` | `src/languages/rust.rs` | Rust-specific queries and semantic handlers |
| `cache` | `src/cache.rs` | LRU cache with mtime invalidation and lock_or_recover pattern |
| `completion` | `src/completion.rs` | Path completion support respecting .gitignore |
| `logging` | `src/logging.rs` | MCP logging integration via tracing; McpLoggingLayer bridges events to MCP clients |
| `schema_helpers` | `src/schema_helpers.rs` | JSON Schema helpers for integer and page_size field validation |
| `test_detection` | `src/test_detection.rs` | Test file detection by path heuristics (directory and filename patterns) |
| `pagination` | `src/pagination.rs` | Cursor-based pagination with CursorData and PaginationMode (Default, Callers, Callees) |
| `graph` | `src/graph.rs` | CallGraph struct and BFS traversal for symbol focus mode |

## Data Flow

```mermaid
graph TD
    A["MCP Request"] --> B1["analyze_directory"]
    A --> B2["analyze_file"]
    A --> B4["analyze_module"]
    A --> B3["analyze_symbol"]
    B1 --> M["walk_directory"]
    M --> N["Parallel Parse rayon"]
    N --> O["ElementExtractor"]
    O --> P["format_structure"]
    B2 --> J["Read File"]
    J --> K["SemanticExtractor"]
    K --> L["format_file_details"]
    B4 --> R["Read File"]
    R --> S["analyze_module_file"]
    S --> T["format_module"]
    T --> Q["MCP Response"]
    B3 --> G["walk_directory"]
    G --> H["Build CallGraph BFS"]
    H --> I["format_focused"]
    P --> Q["MCP Response"]
    L --> Q
    T --> Q
    I --> Q
```

## Analysis Modes

### analyze_directory (Directory Overview)

1. Walk directory tree (respects .gitignore)
2. Filter to source files by extension
3. Parallel parse with rayon: extract function/class counts via ElementExtractor
4. Format as tree with LOC and counts per file

### analyze_file (File Details)

1. Detect language from extension
2. SemanticExtractor parses the file: functions with signatures, classes/structs with fields, imports, type references
3. Format as structured sections

### analyze_module (Module Index)

1. Detect language from extension
2. `analyze_module_file` in `src/analyze.rs` reads the file and dispatches to `SemanticExtractor`
3. Returns a minimal fixed schema: `name`, `line_count`, `language`, `functions[{name, line}]`, `imports[{module, items}]`
4. No call graph, no type references, no field accesses -- output is ~75% smaller than `analyze_file`

### analyze_symbol (Symbol Call Graph)

1. Walk entire directory to build symbol index
2. Build CallGraph via BFS: callers (incoming) and callees (outgoing) to configurable depth
3. Sentinel values: `<module>` for top-level calls, `<reference>` for type references
4. Symbols called >3x marked with `•N`
5. Format as FOCUS/DEPTH/DEFINED/CALLERS/CALLEES sections

## Language Handler System

Each language is registered in `languages/mod.rs` as a `LanguageInfo` with tree-sitter queries and optional handler functions:

- Mandatory queries: `element_query`, `call_query`
- Optional queries: `reference_query`, `import_query`, `impl_query`, `assignment_query`, `field_query`
- Handler functions: `extract_function_name`, `find_method_for_receiver`, `find_receiver_type`, `extract_inheritance` (optional)

Adding a language requires: a tree-sitter grammar crate, a language module with `ELEMENT_QUERY` and `CALL_QUERY`, registration in `languages/mod.rs`, and extension mappings in `lang.rs`. See CONTRIBUTING.md for a step-by-step guide.

## Call Graph Design

BFS from the target symbol outward, tracking callers and callees at each depth level. Visited symbols are memoized to avoid cycles. Call frequency is counted across the walk; symbols exceeding the threshold are annotated in output. Sentinel values (`<module>`, `<reference>`) represent call sites that have no enclosing function or are type-level references rather than call expressions.

Symbol resolution uses SymbolMatchMode to locate the target symbol: Exact (case-sensitive, default), Insensitive (case-insensitive exact), Prefix (case-insensitive prefix match), and Contains (case-insensitive substring match). When multiple candidates match, resolve_symbol() returns an error listing candidates; clients must refine the query or use a stricter match_mode.

## MCP Resources (Planned)

### Current state

`CodeAnalyzer` implements `ServerHandler` but does not override `list_resources()` or `read_resource()`. The default implementations return empty results and `method_not_found` respectively. No resource endpoints exist.

### Value proposition

With resources, agents can discover tool capabilities without making exploratory tool calls:

- Which languages and file extensions are supported?
- What are example queries for each analysis mode?
- What are the performance characteristics for different codebase sizes?

Without resources, agents must read documentation out-of-band or infer capabilities through trial and error.

### Resource URI scheme

| URI | Content | Format |
|-----|---------|--------|
| `catalog/languages` | Supported languages and file extensions | JSON |
| `catalog/modes` | Tool names, descriptions, when to use each | JSON |
| `patterns/overview/examples` | Example queries for `analyze_directory` | JSON |
| `patterns/file-details/examples` | Example queries for `analyze_file` | JSON |
| `patterns/module/examples` | Example queries for `analyze_module` | JSON |
| `patterns/symbol-focus/examples` | Example queries for `analyze_symbol` | JSON |
| `performance/characteristics` | Token and latency estimates by codebase size | JSON |

### Implementation path

Override two methods on `CodeAnalyzer`'s `ServerHandler` impl in `src/lib.rs`:

- `list_resources()` -- enumerate the URIs above with name and MIME type
- `read_resource()` -- route by URI and return static JSON content

Resource data (language registry, example queries) should be defined as static constants close to the relevant logic (language registry in `src/lang.rs`, mode examples adjacent to tool definitions).

### Notes

- This is a planned design, not a committed API contract. URI scheme and content may evolve before Phase 2 implementation.
- MCP resource subscription (`resources/subscribe`) is out of scope; all resources are static.
- Client adoption of MCP resources is still emerging. Validate real-world agent behavior before prioritizing above other enhancements.
- Phase 2 implementation is tracked in a separate issue.
