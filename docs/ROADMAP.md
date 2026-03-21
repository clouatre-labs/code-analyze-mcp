# Roadmap

## Wave History

### [Complete] Wave 1: Core Analysis
Initial release. Four tools (`analyze_directory`, `analyze_file`, `analyze_module`, `analyze_symbol`), seven languages (Rust, Go, Java, Python, TypeScript, TSX, Fortran), tree-sitter AST extraction, rayon parallelism, .gitignore-aware walk via `ignore` crate.

### [Complete] Wave 2: MCP Protocol (milestone 7)
Streamable HTTP transport, summary-first output, `outputSchema` per tool, cursor pagination.

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
- #340: `analyze_module` directory guard; actionable error steering agents to `analyze_directory`
- #341: Actionable SUGGESTION footer naming largest source directory with absolute path
- #342: Server instructions updated with 4-step recommended workflow
- #354: Async metrics collection via `src/metrics.rs`; zero hot-path overhead
- #357: ROADMAP.md and OBSERVABILITY.md documentation

---

## Benchmark-Driven Development

**Scoring rubric:** 4 dimensions scored 0-3 each: `structural_accuracy`, `cross_module_tracing`, `approach_quality`, `tool_efficiency`. `quality_score = sum` (max 12).

For the full methodology, condition matrix, statistical method, and blind scoring protocol, see [DESIGN-GUIDE.md](DESIGN-GUIDE.md#6-benchmark-driven-development).

---

## Small-Model-First Constraint

All output changes must be validated against Haiku, Mistral-small-2603, and MiniMax-M2.5 before Sonnet. See [DESIGN-GUIDE.md](DESIGN-GUIDE.md#3-designing-for-small-models) for the full constraint and rationale.

---

## Shared Exclusion List

`EXCLUDED_DIRS` in `src/formatter.rs` is the single authoritative constant; do not duplicate it. See [DESIGN-GUIDE.md](DESIGN-GUIDE.md#8-anti-patterns) for the duplication anti-pattern and corrective guidance.

---

## Annotation Posture Policy

Current posture: `readOnlyHint=true`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false`. No changes until new MCP SEPs land (#1913, #1984, #1561, #1560, #1487). See [DESIGN-GUIDE.md](DESIGN-GUIDE.md#7-mcp-tool-annotations) for rationale and the full annotation table.

---

## Wave 7+ Direction (Tentative)

- Fix A from #341: true total file annotation in directory count line (deferred from Wave 6; requires full subtree walk)
- Session ID field in `MetricEvent` schema (enables grouping without timestamp heuristic; see #355)
- `idempotentHint` correctness audit (#347)
- `readOnlyHint` client enforcement verification (#349)
- MCP SEP adoption pending #1913, #1984, #1561, #1560, #1487
- Streamable HTTP transport (fully specified in MCP 2025-06-18; enables remote deployment)
- outputSchema per tool definition (enables client-side validation of structured results)
