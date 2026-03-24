# Roadmap

## Wave History

### [Complete] Wave 1: Core Analysis
Initial release. Four tools (`analyze_directory`, `analyze_file`, `analyze_module`, `analyze_symbol`), seven languages (Rust, Go, Java, Python, TypeScript, TSX, Fortran), tree-sitter AST extraction, rayon parallelism, .gitignore-aware walk via `ignore` crate.

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

- Fix A from #341: true total file annotation in directory count line (deferred from Wave 6 -- requires full subtree walk; benchmark-backed)
- MCP SEP adoption: #1487 (`trustedHint`), #1560 (`secretHint`), #1561 (`unsafeOutputHint`), #1913 (trust/sensitivity annotations), #1984 (governance annotations) -- all open upstream; no action until specs stabilize

Not pertinent for current deployment:

- Streamable HTTP or remote transport: not currently implemented; the deployment uses stdio transport for co-located agent use. Revisit only if remote or multi-agent deployment becomes a goal.
