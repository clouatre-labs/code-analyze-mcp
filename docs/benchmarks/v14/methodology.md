# v14: ripgrep Sink Trait Implementation Audit Benchmark

## Overview

v14 measures the impact of MCP tools versus native tools on Rust codebase analysis. It uses the same
2x2 factorial design as v13 (model x tool_set) with 4 conditions, scored across 3 rubric dimensions,
and analyzed for tool-set effects on task performance.

The target repository is ripgrep (`BurntSushi/ripgrep`), a line-oriented search tool optimized for
speed and correctness. It comprises 98 Rust source files across 7 crates (searcher, printer, matcher,
regex, pcre2, cli, core), with trait dispatch patterns that scatter concrete implementations across
multiple crates. This structure is deliberately harder for native file-reading tools than Fortran was:
the Sink trait is defined in one crate (searcher) but implemented in three separate crates (searcher,
printer, testutil), so grep finds all impl blocks but cannot resolve which are live-path versus
test-only without reading every file. `analyze_symbol` with a directory path resolves the correct
implementations in one call.

## Background

v13 established MCP advantage on Fortran scientific HPC code (344 files, heavy name collision). v14
tests whether the same advantage holds on Rust, where trait dispatch scatters concrete implementations
across crates and requires understanding the dispatch mechanism to identify live-path instantiations.

## Repository

- **Repository:** `BurntSushi/ripgrep`
- **Commit:** `4649aa9700619f94cf9c66876e9549d83420e16c` (pinned for reproducibility)
- **License:** Apache-2.0/MIT
- **Language:** Rust (edition 2021)

## Module Structure

| Directory | Files | Role |
|-----------|-------|------|
| crates/searcher/src/ | ~15 | Sink trait definition, Searcher, search glue |
| crates/printer/src/ | ~10 | StandardSink, JSONSink, SummarySink implementations |
| crates/core/src/ | ~10 | SearchWorker, CLI dispatch, Printer enum |
| crates/matcher/src/ | ~5 | Matcher trait and implementations |
| crates/grep-regex/src/ | ~5 | RegexMatcher implementation |
| crates/grep-pcre2/src/ | ~5 | PCRE2Matcher implementation |
| crates/cli/src/ | ~20 | Command-line argument parsing and main entry point |

## Why This Codebase

1. **Trait scatter:** Sink implementations are defined in 3 separate crates (searcher, printer, testutil).
   Grep "impl Sink" returns all of them but does not distinguish live-path (StandardSink, JSONSink,
   SummarySink) from test-only (KitchenSink) or blanket impls (&mut S, Box<S>). Native tools require
   reading every file to understand which are used in production dispatch.

2. **Call chain depth:** SearchWorker::search -> SearchWorker::search_reader -> search_reader free fn
   -> Searcher::search_reader requires 3 hops. Native tools need iterative grep-read loops; analyze_symbol
   collapses to 1-2 calls.

3. **Cross-crate dispatch:** The Printer enum in crates/core/args.rs dispatches to concrete Sink impls
   from crates/printer. Tracing this with grep requires reading both crates and understanding the
   enum-to-impl mapping.

## Design

Same 2x2 factorial design as v12/v13:
- **Model:** claude-sonnet-4-6 (A, B) vs claude-haiku-4-5 (C, D)
- **Tool set:** MCP tools only (A, C) vs native tools only (B, D)
- **Sample design:** N=2 scored runs + N=1 pilot run per condition = 12 total runs
- **Randomization:** Pilots first (4 runs), then scored runs in randomized order (seed=42)

## Task

(See prompts/task.md for full task description.)

The task is "ripgrep Sink Trait Implementation Audit". Context: auditing BurntSushi/ripgrep before
adding a new output format. The Sink trait in crates/searcher/src/sink.rs drives all search output.

Three subtasks:

1. Identify all concrete types that implement the Sink trait -- names, files, and approximate line
   numbers of their impl blocks.

2. Trace the call chain from SearchWorker::search (crates/core/search.rs) through to
   Searcher::search_reader (crates/searcher/src/searcher/mod.rs). Identify which Sink implementations
   are instantiated at the dispatch point.

3. Produce a change-impact map: which files and line ranges must be modified to add a new Sink
   implementation (e.g., a CSV printer) and integrate it into the dispatch path.

## Execution

Runner: `scripts/bench-v14-run.sh`. Parameterized by CONDITION_ID (A-D) and RUN_ID.

Environment variables:
- `RIPGREP_REPO` -- local path to ripgrep clone (default: /tmp/ripgrep-benchmark)
- `RIPGREP_COMMIT` -- override the pinned commit SHA (optional; scored runs still abort if HEAD does not match)
- `ANTHROPIC_DEFAULT_SONNET_MODEL` -- model ID for conditions A/B (default: claude-sonnet-4-6)
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` -- model ID for conditions C/D (default: claude-haiku-4-5)

Tool isolation: MCP conditions (A, C) use --strict-mcp-config with mcp-code-analyze-only.json.
Native conditions (B, D) use empty MCP config. Tool isolation is validated by parsing session JSONL.

## Conditions

- **Condition A:** claude-sonnet-4-6 + MCP tools (analyze_directory, analyze_file, analyze_symbol, analyze_module)
- **Condition B:** claude-sonnet-4-6 + native tools (Bash, Glob, Grep, Read, Write, ToolSearch)
- **Condition C:** claude-haiku-4-5 + MCP tools
- **Condition D:** claude-haiku-4-5 + native tools

## Rubric

3 dimensions x 3 points = 9 max.

### Dimension 1: Sink Identification (0-3)

- **0:** No Sink impls identified; wrong files; confuses Sink with Matcher or other traits
- **1:** Identifies 1-2 Sink impls with correct files but no line numbers; misses live-path vs test-only distinction
- **2:** Correctly names 3+ Sink impls with correct files; line numbers within +-20; distinguishes live-path
  (StandardSink, JSONSink, SummarySink) from convenience impls (sinks::UTF8, sinks::Lossy, sinks::Bytes);
  blanket impls optional
- **3:** Names all 6 non-blanket impls (StandardSink, JSONSink, SummarySink, sinks::UTF8, sinks::Lossy,
  sinks::Bytes) with correct files and line numbers within +-20; explicitly notes KitchenSink is
  test-only; blanket impls (&mut S, Box<S>) noted as optional

**Calibration (commit 4649aa9):**
- StandardSink: crates/printer/src/standard.rs ~line 807
- JSONSink: crates/printer/src/json.rs ~line 684
- SummarySink: crates/printer/src/summary.rs ~line 646
- sinks::UTF8: crates/searcher/src/sink.rs ~line 550
- sinks::Lossy: crates/searcher/src/sink.rs ~line 598
- sinks::Bytes: crates/searcher/src/sink.rs ~line 648
- KitchenSink (test-only): crates/searcher/src/testutil.rs ~line 128
- &mut S blanket: crates/searcher/src/sink.rs ~line 224
- Box<S> blanket: crates/searcher/src/sink.rs ~line 282

### Dimension 2: Call Chain Tracing (0-3)

- **0:** No call chain; no mention of SearchWorker or Searcher::search_reader
- **1:** Identifies SearchWorker as entry point; names at most 1 intermediate step without file evidence
- **2:** Traces SearchWorker::search -> SearchWorker::search_reader -> Searcher::search_reader with
  correct files; minor gap at dispatch point
- **3:** Complete chain: SearchWorker::search -> search_reader method -> search_reader free fn ->
  Searcher::search_reader; identifies the Printer enum dispatch in crates/core/search.rs as the
  instantiation point; names at least 2 of the 3 live-path Sink impls instantiated there

**Calibration:**
- SearchWorker::search: crates/core/search.rs ~line 244
- SearchWorker::search_reader: crates/core/search.rs ~line 360
- search_reader free fn: crates/core/search.rs ~line 414
- Searcher::search_reader: crates/searcher/src/searcher/mod.rs ~line 707

### Dimension 3: Change Impact Map (0-3)

- **0:** No change-impact map; no files cited; no mention of where to add a new printer
- **1:** Cites only crates/printer/src/ as change point; does not trace back to CLI dispatch or Printer
  enum; no line ranges
- **2:** Identifies 3+ touchpoints (new printer file, Printer enum in crates/core, CLI flag dispatch,
  lib.rs re-export); provides line ranges for at least 2; minor gaps
- **3:** Complete map covering all 4 layers: new file in crates/printer/src/csv.rs, Printer enum
  variant in crates/core/args.rs or similar, match arm in search_reader free fn, re-export in
  crates/printer/src/lib.rs; line ranges for at least 3; notes that the Printer enum dispatch is
  the integration point

## Analysis

Same statistical approach as v13: rank-biserial r for tool-set effect, no p-values (n too small for
frequentist inference). Report descriptive statistics (mean, median, range) per condition and per
dimension.

## Run Order

See run-order.txt. Pilots execute in order (A, B, C, D). Scored runs execute in randomized order
(seed=42).

## Files

- docs/benchmarks/v14/methodology.md (this file)
- docs/benchmarks/v14/prompts/task.md
- docs/benchmarks/v14/prompts/condition-a-mcp-sonnet.md
- docs/benchmarks/v14/prompts/condition-b-native-sonnet.md
- docs/benchmarks/v14/prompts/condition-c-mcp-haiku.md
- docs/benchmarks/v14/prompts/condition-d-native-haiku.md
- docs/benchmarks/v14/run-order.txt
- docs/benchmarks/v14/scores-template.json
- docs/benchmarks/v14/mcp-code-analyze-only.json
- docs/benchmarks/v14/results/runs/.gitkeep
- scripts/bench-v14-run.sh
