# Roadmap

## Wave History

### [Complete] Wave 1: Core Analysis
Initial release. Four tools (`analyze_directory`, `analyze_file`, `analyze_module`, `analyze_symbol`), seven languages (Rust, Go, Java, Python, TypeScript, TSX, Fortran), tree-sitter AST extraction, rayon parallelism, .gitignore-aware walk via `ignore` crate. (language support has since grown to 11; see [Supported Languages](../README.md))

### [Complete] Wave 2: MCP Protocol (milestone 7)
Summary-first output, `outputSchema` per tool, cursor pagination.

### [Complete] Wave 3: Analysis Quality (milestone 8)
Multi-strategy call graphs, inheritance tracking, cross-client compatibility.

### [Complete] Wave 4: Advanced Analysis (milestone 9)
Type-aware call resolution, dataflow analysis.

### [Complete] Wave 5: Progressive Disclosure (milestone 10)
Summary and pagination for FileDetails and SymbolFocus modes.

### [Complete] Wave 6: Agent UX & Performance (milestone 11)
Issues: #340, #341, #342, #354, #355, #356, #357.

Target: close the non-Sonnet model performance gap identified in v10 benchmark.

Key changes:
- #340: `analyze_module` directory guard — actionable error steering agents to `analyze_directory`
- #341: Actionable SUGGESTION footer naming largest source directory with absolute path
- #342: Server instructions updated with 4-step recommended workflow
- #354: Async metrics collection via `src/metrics.rs` — zero hot-path overhead
- #356: Idempotency audit and cross-client compatibility verification
- #357: ROADMAP.md and OBSERVABILITY.md documentation

### [Complete] 0.3.0 Library API

Promotes `code-analyze-core` to a stable public library API and adds structured output fields for programmatic consumption without text parsing.

Issues: #623, #624, #625.

Key changes:
- #623: `analyze_str(source, language, ast_recursion_limit)` -- public in-memory parsing API; eliminates TOCTOU race for consumers holding source text without an on-disk path; adds `AnalyzeError::UnsupportedLanguage` variant
- #624: `CallChainEntry { symbol, file, line }` public type; `callers`, `test_callers`, `callees` fields on `FocusedAnalysisOutput`; MCP clients can now consume caller/callee relationships from `structuredContent` without text parsing
- #625: `analyze_symbol` tool description updated to accurately reflect `FocusedAnalysisOutput` schema

### [Benchmarking] Wave 7: OpenFAST Fortran Analysis (v13)

2x2 factorial design (model x tool_set) on Fortran scientific HPC code (OpenFAST). See [v13 methodology](docs/benchmarks/v13/methodology.md).

### [Benchmarking] Wave 8: Rust Trait Dispatch Analysis (v14)

2x2 factorial design (model x tool_set) on Rust trait implementations (ripgrep). See [v14 methodology](docs/benchmarks/v14/methodology.md).

---

## Benchmark-Driven Development

Each Wave closes with a benchmark run validating the Wave's hypotheses.

**Benchmark location:** `docs/benchmarks/vN/` (v3–v10, v12 present)

**Scoring rubric (v12+):** 3 dimensions scored 0–3 each:
- `structural_accuracy`
- `cross_module_tracing`
- `approach_quality`

`quality_score = sum` (max 9)

Earlier benchmarks (v3–v10) used a 4-dimension rubric including `tool_efficiency` (max 12). The `tool_efficiency` dimension was dropped in v12; see each benchmark's `methodology.md` for the rubric in effect at the time.

**Evaluation protocol:** Blind scoring — scorer does not see condition labels during evaluation.

**Statistical method:** Mann-Whitney U with Bonferroni correction; 15 pairwise tests at alpha = 0.05/15 = 0.0033.

---

## Small-Model-First Constraint

All output changes, error messages, server instructions, and tool descriptions must be evaluated against Haiku, Mistral-small-2603, and MiniMax-M2.5 **before** Sonnet.

These models follow tool descriptions literally; they do not apply contextual reasoning to infer optimal paths. A change that improves Sonnet but regresses Haiku is a regression.

---

## Shared Exclusion List

The following directories are non-source and excluded from SUGGESTION footer logic (`src/formatter.rs`) and server instruction guidance (`src/lib.rs`). The constant is defined in `src/lib.rs`:

```
node_modules, vendor, .git, __pycache__, target, dist, build, .venv
```

This list is a single constant in the codebase:

```rust
// src/lib.rs
pub(crate) const EXCLUDED_DIRS: &[&str] = &[
    "node_modules", "vendor", ".git", "__pycache__",
    "target", "dist", "build", ".venv",
];
```

Do not duplicate this constant across modules. Both `#341` and `#342` reference `EXCLUDED_DIRS` from `src/lib.rs`.

---

## Annotation Posture Policy

Current settings are stable and reflect ground truth:

| Annotation | Value | Rationale |
|---|---|---|
| `readOnlyHint` | `true` | All tools are read-only filesystem operations |
| `destructiveHint` | `false` | No writes, no side effects |
| `idempotentHint` | `true` | Same input produces same output (verified by #347) |
| `openWorldHint` | `false` | Results are bounded by the input path |

No annotation changes until new MCP SEPs land (tracked in #1913, #1984, #1561, #1560, #1487). Validated against external MCP Blog 2 reference (2026-03-16).

---

## Wave 7+ Direction (Tentative)

Unimplemented and pertinent:

- MCP SEP adoption: #1487 (`trustedHint`), #1561 (`unsafeOutputHint`), #1913 (trust/sensitivity annotations), #1984 (governance annotations) -- open upstream; no action until specs stabilize. #1560 (`secretHint`) closed 2026-03-23; evaluate adoption once merged into spec.
- Streamable HTTP transport: add `--http` flag exposing `StreamableHttpService` (axum + rmcp `transport-streamable-http-server` + `transport-streamable-http-server-session` features) alongside existing stdio. Tower middleware: `RequestBodyLimitLayer` (4 MB) + `tower-governor` (per-token rate limit) + static Bearer token from env var. Target deployment: GCP e2-micro Always Free (us-central1) behind Cloudflare proxy (free tier, TLS termination, WAF, 5 rate-limit rules). No changes to tool handlers or session logic required.

## Wave 9: Editing Tools

Augments aptu-coder with five mechanical code-editing tools in two phases. The existing analysis tools and composition API remain unchanged. This wave completes the read-analyze-write loop that the coder-build agent (#664, #665) requires without introducing a second MCP server.

**Prerequisite:** the rename PR (#664) must merge before this wave begins.

### Rationale

Both a combined server and two separate servers inject into the same model context window. Token cost is identical. A single server with one MCP config entry, one binary, and one version pin is operationally simpler. Five editing tools keep the total tool count below the reliable SML selection ceiling (~10-12 tools).

The `ToolRouter::merge()` / `Add` / `AddAssign` API (verified against rmcp 1.5.0 source) supports multi-group composition with no breaking changes from 1.1.0. Write tools are placed in a second `#[tool_router(router = write_router, vis = "pub")]` impl block and merged at construction.

### Phase 1: Mechanical tools [no AST required]

Three tools with no tree-sitter dependency. These validate the BUILD agent workflow and establish the write-path integration before adding AST complexity.

- `read_file(path, start_line?, end_line?)` -- "Raw file content with optional line range. Prefer start_line/end_line to limit tokens on large files; omit both for full content. Use analyze_file for structure, not content. Example queries: Read lines 10-40 of src/lib.rs; Show the full contents of config.toml." `read_only_hint=true`, `idempotent_hint=true`
- `write_file(path, content)` -- "Create or overwrite a file at path with content. Creates parent directories if needed. Overwrites without confirmation; use edit_file to replace a specific block instead of the whole file. Example queries: Write a new test file at tests/foo_test.rs; Overwrite src/config.rs with updated content." `read_only_hint=false`, `destructive_hint=true`, `idempotent_hint=false`
- `edit_file(path, old_text, new_text)` -- "Replace a unique exact text block in a file. Errors if old_text appears zero times or more than once -- fix by making old_text longer and more specific. Use write_file to replace the whole file. Example queries: Replace the error handling block in src/main.rs; Update the function signature in lib.rs." `read_only_hint=false`, `destructive_hint=true`, `idempotent_hint=false`

Cache invalidation: `write_file` and `edit_file` must call `cache.invalidate(path)` after every successful write or the next `analyze_file` call returns stale data.

### Phase 2: AST-backed tools

Two tools that require `aptu-coder-core` (formerly `code-analyze-core`) capture data. These are the primary justification for keeping editing in the same crate rather than a separate repository.

- `rename_symbol(path, old_name, new_name, kind?)` -- "AST-aware rename within a single file. Matches by node kind, not string -- identifiers in string literals and comments are excluded. Errors if old_name not found; supply kind to disambiguate (function, variable, type). Directory-wide rename not supported in v1. Example queries: Rename function parse_config to load_config in src/config.rs; Rename struct field timeout to timeout_ms." `read_only_hint=false`, `destructive_hint=true`, `idempotent_hint=false`
- `insert_at_symbol(path, symbol_name, position, content)` -- "Insert content immediately before or after a named AST node. position is before|after. Uses start_byte/end_byte from the capture pipeline; errors if symbol_name not found in file. Example queries: Insert a tracing span before the handle_request function; Add a derive macro after the MyStruct definition." `read_only_hint=false`, `destructive_hint=true`, `idempotent_hint=false`

### Annotation posture update

Phase 2 tools are the first exception to the annotation freeze established in the Annotation Posture Policy section. Write tools carry `read_only_hint=false`, `destructive_hint=true`, `idempotent_hint=false`. Read tools (`read_file`, and all existing analysis tools) retain `read_only_hint=true`. The per-tool `#[tool(annotations(...))]` macro attribute in rmcp 1.5.0 is confirmed to support mixed postures within one server.

Note: `read_only_hint` is a hint surfaced to MCP clients in `tools/list`; rmcp 1.5.0 has no per-tool access control enforcement.

### SML validation requirement

Per the Small-Model-First Constraint: all five tools must be evaluated against Haiku, Mistral-small-2603, and MiniMax-M2.5 before Sonnet in a Wave 9 benchmark. Tool descriptions must follow literal-instruction style -- SML models follow tool descriptions literally.

### Risks

- **Cache staleness** -- mitigated by mandatory `cache.invalidate(path)` in all write paths
- **`rename_symbol` scope creep** -- directory-wide rename requires type information tree-sitter cannot provide; enforce single-file boundary in v1 with a clear error if a directory path is supplied
- **Annotation posture drift** -- document the per-tool posture in this section; update REUSE.toml for any new source files
- **SPDX headers** -- every new `.rs` file requires an SPDX header or `reuse lint` fails CI
