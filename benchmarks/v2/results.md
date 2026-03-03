# Benchmark v2 Results

## Setup

- **Target repo:** sharkdp/bat (10.8K LOC, 40 Rust files, multi-module)
- **Task:** Map data flow from user input to terminal output; identify module boundaries and key types; propose HTML output extension point
- **Model:** claude-haiku-4-5@20251001 (via gcp_vertex_ai)
- **Temperature:** 0.5
- **Conditions:**
  - Control (A): `developer` extension only (no `developer__analyze`)
  - Treatment (B): `developer` + `code-analyze-mcp` (`developer__analyze` forced)

## Rubric

| Dimension | 0 | 1 | 2 | 3 |
|-----------|---|---|---|---|
| Structural accuracy | Wrong module map | Partial | Correct but incomplete | Complete and correct |
| Cross-module tracing | Missed dependencies | Found some | Found most | Found all with context |
| Approach quality | Infeasible | Works but naive | Good with tradeoffs | Elegant, minimal, safe |
| Tool efficiency | >25 calls | 16-25 | 8-15 | <8 calls |

## Raw Scores

### Control Run A1 (26 tool calls, 40s)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Structural accuracy | 3 | Listed 20 modules with accurate roles. Identified all core pipeline modules including preprocessor, terminal, wrapping. Correctly described Printer trait with two implementations. |
| Cross-module tracing | 3 | Traced full pipeline: main -> Controller -> Printer -> Output. Identified Printer trait, Decoration trait, all key types (Config, HighlightingAssets, OutputHandle, LineChanges). Named run_controller() entry point. 14-step pipeline with 10 boundary crossings. |
| Approach quality | 3 | Identified Printer trait as extension point. Proposed HtmlPrinter implementing Printer trait. Correctly noted Controller::print_file() takes &mut dyn Printer. Minimal file list: printer.rs, config.rs, controller.rs, clap_app.rs. |
| Tool efficiency | 0 | 26 tool calls (>25 threshold) |
| **Total** | **9** | |

### Control Run A2 (37 tool calls, 55s)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Structural accuracy | 3 | Listed 20 modules with accurate roles. Identified all core modules including pager.rs. Correctly described Printer trait and both implementations. |
| Cross-module tracing | 3 | Full pipeline traced: main -> Controller -> Printer -> Output. 14-step pipeline with 10 boundary crossings. Identified Printer trait, Decoration trait, all key types. Named run_controller() entry point. |
| Approach quality | 3 | Identified Printer trait as extension point. Proposed HtmlPrinter implementing same trait methods. Correctly noted decorations remain unchanged. Minimal file list. |
| Tool efficiency | 0 | 37 tool calls (>25 threshold) |
| **Total** | **9** | |

### Control Run A3 (31 tool calls, 76s)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Structural accuracy | 3 | Listed 20 modules with accurate roles. Identified all core pipeline modules. Correctly described Printer trait and both implementations. |
| Cross-module tracing | 3 | Full pipeline traced with 19 steps. Identified Printer trait, Decoration trait, all key types. Named run_controller() entry point. Detailed boundary crossings. |
| Approach quality | 3 | Identified Printer trait as extension point. Proposed HtmlPrinter. Correctly noted existing abstractions enable minimal changes. |
| Tool efficiency | 0 | 31 tool calls (>25 threshold) |
| **Total** | **9** | |

### Treatment Run B1 (18 tool calls, 25s)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Structural accuracy | 2 | Listed 18 modules but missed some (wrapping.rs, lessopen.rs, macros.rs, nonprintable_notation.rs). Roles mostly accurate. Did not explicitly name the Printer trait (listed InteractivePrinter and SimplePrinter but not the trait itself in module map). |
| Cross-module tracing | 2 | 10-step pipeline is compressed; misses some intermediate steps (e.g., print_file_ranges, line range filtering). Identified key boundary crossings but fewer details on types. Did not name run_controller() entry point. Missing diff.rs integration in pipeline. |
| Approach quality | 2 | Proposed OutputFormatter trait instead of using existing Printer trait. This adds unnecessary abstraction; the Printer trait already serves this purpose. Workable but not minimal. |
| Tool efficiency | 2 | 18 tool calls (16-25 range) |
| **Total** | **8** | |

### Treatment Run B2 (17 tool calls, 23s)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Structural accuracy | 2 | Listed 16 modules. Missing wrapping.rs, lessopen.rs, macros.rs, nonprintable_notation.rs, terminal.rs, diff.rs. Did not explicitly name the Printer trait in module map (listed InteractivePrinter and SimplePrinter). |
| Cross-module tracing | 3 | 17-step pipeline is detailed. Identified key boundary crossings including Printer -> decorations, Printer -> vscreen. Named key types. Traced full path from main to terminal output. |
| Approach quality | 2 | Proposed new Formatter trait alongside Printer. While workable, this misses that the Printer trait already is the abstraction point. Adding a Formatter trait is unnecessary indirection. |
| Tool efficiency | 2 | 17 tool calls (16-25 range) |
| **Total** | **9** | |

### Treatment Run B3 (19 tool calls, 25s)

| Dimension | Score | Notes |
|-----------|-------|-------|
| Structural accuracy | 2 | Listed 19 modules. Missing wrapping.rs, lessopen.rs, macros.rs, nonprintable_notation.rs, terminal.rs. Roles mostly accurate. Did not explicitly name the Printer trait. |
| Cross-module tracing | 3 | 20-step pipeline is detailed. Identified all key boundary crossings. Named key types. Traced full path from main to terminal output including pager. |
| Approach quality | 2 | Proposed Formatter trait abstraction. Same issue as B1/B2: misses that Printer trait already serves this purpose. Workable but adds unnecessary complexity. |
| Tool efficiency | 2 | 19 tool calls (16-25 range) |
| **Total** | **9** | |

## Summary Statistics

### Per-Dimension Means

| Dimension | Control Mean (A) | Treatment Mean (B) | Difference |
|-----------|-----------------|-------------------|------------|
| Structural accuracy | 3.0 | 2.0 | -1.0 |
| Cross-module tracing | 3.0 | 2.7 | -0.3 |
| Approach quality | 3.0 | 2.0 | -1.0 |
| Tool efficiency | 0.0 | 2.0 | +2.0 |
| **Total** | **9.0** | **8.7** | **-0.3** |

### Per-Condition Summary

| Condition | Run 1 | Run 2 | Run 3 | Mean | Range |
|-----------|-------|-------|-------|------|-------|
| Control (A) | 9 | 9 | 9 | 9.0 | 0 |
| Treatment (B) | 8 | 9 | 9 | 8.7 | 1 |

## Metrics Comparison

| Metric | Control (A) | Treatment (B) | Delta | Delta % |
|--------|:-----------:|:-------------:|:-----:|:-------:|
| Wall time mean (s) | 57.0 | 24.3 | -32.7 | -57% |
| Wall time range (s) | 40-76 | 23-25 | | |
| Tool calls mean | 31.3 | 21.3 | -10.0 | -32% |
| Tool calls range | 26-37 | 21-22 | | |
| Total tokens mean | 33463 | 26309 | -7154 | -21% |
| Input tokens mean | 31087 | 22963 | -8124 | -26% |
| Output tokens mean | 2376 | 3347 | +971 | +41% |
| Quality score mean (0-12) | 9.0 | 8.7 | -0.3 | -3% |
| Quality score range | 9-9 | 8-9 | | |

### Tool Call Counts and Wall Time

| Condition | Run 1 | Run 2 | Run 3 | Mean | Tool Breakdown |
|-----------|-------|-------|-------|------|---|
| Control (A) calls | 26 | 37 | 31 | 31.3 | shell: 24-35, text_editor: 2 |
| Control (A) wall time (s) | 40 | 55 | 76 | 57.0 | |
| Treatment (B) calls | 22 | 21 | 21 | 21.3 | analyze: 20-21, text_editor: 1 |
| Treatment (B) wall time (s) | 25 | 23 | 25 | 24.3 | |

## Analysis

- Total quality score: Control 9.0, Treatment 8.7 (difference -0.3).
- Wall time: Treatment 24.3s mean vs Control 57.0s mean (57% reduction).
- Tool calls: Treatment 21.3 mean vs Control 31.3 mean (32% reduction).
- Total tokens: Treatment 26309 mean vs Control 33463 mean (21% reduction).
- Input tokens: Treatment 22963 mean vs Control 31087 mean (26% reduction).
- Output tokens: Treatment 3347 mean vs Control 2376 mean (41% increase).
- Structural accuracy: Control 3.0 mean, Treatment 2.0 mean (difference -1.0). Control identified 20 modules consistently; Treatment identified 16-19 modules, missing wrapping.rs, lessopen.rs, macros.rs, nonprintable_notation.rs, terminal.rs, diff.rs.
- Cross-module tracing: Control 3.0 mean, Treatment 2.7 mean (difference -0.3). Control traced 14-19 step pipelines; Treatment traced 10-20 step pipelines.
- Approach quality: Control 3.0 mean, Treatment 2.0 mean (difference -1.0). Control identified Printer trait as extension point; Treatment proposed new Formatter trait in all runs.
- Tool efficiency: Control 0.0 mean (all runs >25 calls), Treatment 2.0 mean (all runs 16-25 calls).

## Summary

Raw data and session IDs are in conditions.json for independent verification.
