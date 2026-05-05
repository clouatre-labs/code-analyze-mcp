# Wave 9 SML: Kotlin Language Support Implementation Benchmark

## Overview

Wave 9 SML measures the impact of MCP edit tools versus native tools on Rust language implementation tasks.
It uses the same 2x2 factorial design as v14 (model x tool_set) with 4 conditions, scored across 3 rubric
dimensions, and analyzed for tool-set effects on implementation completeness and correctness.

The target task is implementing Kotlin grammar support in the aptu-coder repository (issue #649). This
requires adding a new language handler: modifying 5 files (2 Cargo.toml files, 3 source files), writing
tree-sitter queries, implementing an inheritance extraction function, and adding unit tests. The task
exercises all 5 edit tools (edit_overwrite, edit_replace, edit_rename, edit_insert) and requires reading
existing code patterns (java.rs) to match style and structure.

## Background

Wave 9 introduced 5 new edit tools to the aptu-coder MCP server: edit_overwrite, edit_replace, edit_rename,
edit_insert. This benchmark validates their utility on a realistic implementation task. The SML (Small
Model Language) constraint from the ROADMAP requires that the benchmark task be completable by smaller
models (haiku) within reasonable token budgets, making language support implementation an ideal test case.

## Repository

- **Repository:** `clouatre-labs/aptu-coder`
- **Commit:** `origin/main` (pinned at run time; fresh worktree per run)
- **License:** Apache-2.0
- **Language:** Rust (edition 2024)

## Module Structure

| Directory | Files | Role |
|-----------|-------|------|
| crates/aptu-coder-core/src/languages/ | ~10 | Language handlers (java.rs, python.rs, etc.) |
| crates/aptu-coder-core/src/lang.rs | 1 | Extension map, supported_languages() |
| crates/aptu-coder-core/src/languages/mod.rs | 1 | Language registry, get_language_info(), get_ts_language() |
| crates/aptu-coder-core/Cargo.toml | 1 | Feature flags, optional dependencies |
| Cargo.toml | 1 | Workspace dependencies |

## Why This Codebase

1. **Well-known to the model:** aptu-coder is the subject of this benchmark; the model has context on
   its structure and patterns from the system prompt and repository analysis.

2. **Edit task exercises all 5 tools:** Creating kotlin.rs requires edit_insert (new file); modifying
   Cargo.toml files requires edit_replace (add dependency, feature); modifying mod.rs and lang.rs
   requires edit_insert (new arms) and edit_replace (update feature sets).

3. **No external clone:** Unlike v14 (ripgrep), the task targets the repo itself. Each run gets a fresh
   worktree from origin/main, eliminating setup overhead and ensuring reproducibility.

4. **Comparison is read-then-write vs write-only:** MCP conditions (A, C) can use analyze_* tools to
   understand patterns before editing; native conditions (B, D) must read files with native tools then
   write with native tools. This isolates the edit-tool advantage.

## Design

Same 2x2 factorial design as v14:
- **Model:** claude-sonnet-4-6 (A, B) vs claude-haiku-4-5 (C, D)
- **Tool set:** MCP tools only (A, C) vs native tools only (B, D)
- **Sample design:** N=2 scored runs + N=1 pilot run per condition = 12 total runs
- **Randomization:** Pilots first (4 runs), then scored runs in randomized order (seed=42)

## Task

(See prompts/task.md for full task description.)

The task is "Kotlin Language Support Implementation" (issue #649). Five subtasks:

1. Add `tree-sitter-kotlin-ng = "1.1.0"` to `[workspace.dependencies]` in root Cargo.toml.

2. In `crates/aptu-coder-core/Cargo.toml`: add optional dependency, feature flag, and add to default features.

3. Create `crates/aptu-coder-core/src/languages/kotlin.rs` with SPDX header, 5 query constants
   (ELEMENT_QUERY, CALL_QUERY, REFERENCE_QUERY, IMPORT_QUERY, DEFUSE_QUERY), extract_inheritance function,
   and 3+ unit tests.

4. In `crates/aptu-coder-core/src/languages/mod.rs`: add module declaration, get_language_info arm,
   get_ts_language arm.

5. In `crates/aptu-coder-core/src/lang.rs`: add .kt and .kts extensions to EXTENSION_MAP, add "kotlin"
   to supported_languages().

## Execution

Runner: `scripts/bench-wave9-run.sh`. Parameterized by CONDITION_ID (A-D) and RUN_ID.

Environment variables:
- `BENCH_MAX_BUDGET_USD` -- cap spend per run (optional, e.g. "2.00")
- `ANTHROPIC_DEFAULT_SONNET_MODEL` -- model ID for conditions A/B (default: claude-sonnet-4-6)
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` -- model ID for conditions C/D (default: claude-haiku-4-5)
- `CARGO_TARGET_DIR` -- optional shared target directory to speed up compilation across runs

Worktree isolation: Each run gets a fresh temporary worktree from origin/main (git worktree add
/tmp/wave9-run-$RUN_ID origin/main). Changes are captured via git diff. Worktree is removed after
post-run verification.

Tool isolation: MCP conditions (A, C) use mcp-aptu-coder-all-tools.json with all 9 tools. Native
conditions (B, D) use empty MCP config. Tool isolation is validated by parsing session JSONL.

## Conditions

- **Condition A:** claude-sonnet-4-6 + MCP tools (all 9: analyze_*, edit_*)
- **Condition B:** claude-sonnet-4-6 + native tools (Bash, Glob, Grep, Read, Write, ToolSearch)
- **Condition C:** claude-haiku-4-5 + MCP tools
- **Condition D:** claude-haiku-4-5 + native tools

## Prompt Symmetry

Each condition receives workflow guidance appropriate to its tool set. MCP conditions (A, C) include
a recommended MCP tool call sequence. Native conditions (B, D) include an equivalent file-read
workflow using native tools covering the same files in the same order. Task information is identical
across all conditions; only operational scaffolding differs.

This design follows established benchmark methodology: each condition is evaluated at its best under
its own operational constraints. Giving one condition sub-optimal scaffolding would introduce a
confound against that condition and make the comparison uninterpretable.

## Rubric

3 dimensions x 3 points = 9 max.

### Dimension 1: Implementation Completeness (0-3)

Verifiable from: agent_json.files_created + agent_json.files_modified + agent_json.extension_registrations

- **0:** Fewer than 3 files touched; kotlin.rs not created; missing both extensions
- **1:** 3-4 files touched; kotlin.rs created but incomplete; missing one required registration
  (e.g. lang.rs extension or mod.rs arm)
- **2:** All 5 files touched but one section incomplete (e.g. .kts extension missing, or get_ts_language
  arm absent, or feature not added to default set)
- **3:** All 5 files touched; both .kt and .kts registered; get_language_info + get_ts_language +
  mod declaration + feature + default feature all present

**Calibration ground truth:**
- Cargo.toml: tree-sitter-kotlin-ng = "1.1.0" added to [workspace.dependencies]
- crates/aptu-coder-core/Cargo.toml: optional dependency, lang-kotlin feature, added to default
- kotlin.rs: created with SPDX header
- mod.rs: #[cfg(feature = "lang-kotlin")] pub mod kotlin; + get_language_info arm + get_ts_language arm
- lang.rs: .kt and .kts in EXTENSION_MAP + "kotlin" in supported_languages()

### Dimension 2: Correctness (Compiles and Tests Pass) (0-3)

Verifiable from: post_run_cargo_test (exit code + test output) + agent_json.ts_crate_used + agent_json.compile_belief

- **0:** cargo test --features lang-kotlin fails to compile or exits non-zero; agent used incompatible
  tree-sitter-kotlin 0.3.8 with no fix
- **1:** cargo test compiles but some kotlin tests fail; partial correctness; agent expressed uncertainty
  about API compatibility
- **2:** cargo test passes but agent expressed uncertainty about API or used a workaround with known
  limitations; compile_belief is not confident_pass
- **3:** cargo test passes cleanly; agent correctly identified tree-sitter-kotlin-ng 1.1.0 as the
  compatible crate; compile_belief is confident_pass with accurate reason

**Calibration ground truth:**
- tree-sitter-kotlin 0.3.8 is incompatible (requires tree-sitter <0.23; workspace uses 0.26.6)
- tree-sitter-kotlin-ng 1.1.0 is compatible (requires tree-sitter ^0.24)
- cargo test --features lang-kotlin must exit 0 with all tests passing

### Dimension 3: Code Quality and Query Completeness (0-3)

Verifiable from: agent_json.queries_written + agent_json.extract_inheritance_present + agent_json.test_names
+ agent_json.files_created[kotlin.rs].has_spdx_header + agent_json.feature_flag_name

- **0:** SPDX header absent OR fewer than 2 of 5 queries written OR no tests
- **1:** SPDX present, 3-4 queries written, 1-2 tests, no DEFUSE_QUERY or extract_inheritance
- **2:** All 5 queries present, SPDX present, extract_inheritance present, 2-3 tests; minor gap
  (e.g. DEFUSE_QUERY absent or only 2 tests)
- **3:** All 5 queries (including DEFUSE_QUERY) present, SPDX header, extract_inheritance handles both
  superclass (parens) and interfaces (no parens), 3+ meaningful unit tests covering element/inheritance/call
  extraction

**Calibration ground truth:**
- SPDX-FileCopyrightText header required (Apache-2.0 license)
- 5 queries required: ELEMENT_QUERY, CALL_QUERY, REFERENCE_QUERY, IMPORT_QUERY, DEFUSE_QUERY
- extract_inheritance function required; must walk delegation_specifiers and distinguish superclass
  (with parens) from interfaces (without parens)
- 3+ unit tests required; must cover: element extraction, inheritance extraction, call extraction

## Analysis

Same statistical approach as v14: rank-biserial r for tool-set effect, no p-values (n too small for
frequentist inference). Report descriptive statistics (mean, median, range) per condition and per
dimension.

## Run Order

See run-order.txt. Pilots execute in order (A, B, C, D). Scored runs execute in randomized order (seed=42).

## Files

- docs/benchmarks/wave9-sml/methodology.md (this file)
- docs/benchmarks/wave9-sml/prompts/task.md
- docs/benchmarks/wave9-sml/prompts/condition-a-mcp-sonnet.md
- docs/benchmarks/wave9-sml/prompts/condition-b-native-sonnet.md
- docs/benchmarks/wave9-sml/prompts/condition-c-mcp-haiku.md
- docs/benchmarks/wave9-sml/prompts/condition-d-native-haiku.md
- docs/benchmarks/wave9-sml/run-order.txt
- docs/benchmarks/wave9-sml/scores-template.json
- docs/benchmarks/wave9-sml/mcp-aptu-coder-all-tools.json
- docs/benchmarks/wave9-sml/results/runs/.gitkeep
- scripts/bench-wave9-run.sh
