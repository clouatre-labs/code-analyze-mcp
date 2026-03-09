# Benchmark v5: Tool Isolation and Efficiency Under rg-Blocking Constraint

## Verdict

**rg-blocking closes the efficiency gap.** code-analyze-mcp with explicit tool isolation
(disabled native analyze, rg blocked for structural queries) matches the native analyze
baseline on both quality and total tool calls, while eliminating the supplementary shell
calls that inflated v3 treatment costs.

| Metric | A (native analyze) | B (code-analyze-mcp) | p-value | Significant? |
|--------|:------------------:|:--------------------:|:-------:|:------------:|
| Quality (0-12) | median 10 | median 10 | -- | No |
| Total tokens | median 25,818 | median 31,620 | 0.008 | **Yes** |
| Wall time (s) | median 70 | median 73 | 0.548 | No |
| Total tool calls | median 15 | median 14 | 0.095 | No |
| Shell calls | median 4 | median 1 | 0.008 | **Yes** |

The token overhead persists (22% more in B), but total tool calls are now statistically
equivalent (p=0.095), and wall time is indistinguishable (p=0.548). The shell call
elimination (median 4 to 1, p=0.008) confirms the rg-blocking constraint works as designed.

## Experiment Design

| Parameter | Value |
|-----------|-------|
| Target repo | lsd-rs/lsd (~13K LOC, 52 Rust source files) |
| Task | Cross-module research: module map, data flow, dependency hubs, change proposal |
| Model | Claude Haiku 4.5, temperature 0.5 |
| Provider | AWS Bedrock |
| Repetitions | n=5 per condition (10 total) |
| Condition A (control) | `analyze` (goose built-in native extension) |
| Condition B (treatment) | `code-analyze-mcp__analyze` with rg-blocking constraint |
| Run order | Randomized per `run-order.txt` (seed=124) |
| Blinding | Condition labels stripped, random shuffle before scoring |
| Extension isolation | `--no-profile` flag; Condition A: `--with-builtin developer,analyze`; Condition B: `--with-builtin developer --with-extension code-analyze-mcp` |

## Tool Isolation

Verified across all 10 runs via session database (10/10 PASS):

- All 5 Condition A runs used only `analyze` (native extension; zero `code-analyze-mcp` calls)
- All 5 Condition B runs used only `code-analyze-mcp__analyze` (zero native `analyze` calls; zero rg structural patterns)
- No runs discarded

Tool isolation was enforced at two levels:
1. **System-level:** `--no-profile` + selective `--with-builtin` ensured each condition only had access to its designated analyze tool.
2. **Prompt-level:** Condition B prompt explicitly blocked rg for structural analysis.

## Quality Results

### Per-Run Scores (0-12)

| Run | Structural | Tracing | Approach | Efficiency | Total |
|-----|:----------:|:-------:|:--------:|:----------:|:-----:|
| A1  | 3 | 2 | 3 | 2 | 10 |
| A2  | 3 | 3 | 3 | 2 | 11 |
| A3  | 2 | 2 | 3 | 1 | 8 |
| A4  | 3 | 3 | 3 | 2 | 11 |
| A5  | 2 | 2 | 3 | 2 | 9 |
| **A median** | **3** | **2** | **3** | **2** | **10** |
| B1  | 3 | 3 | 3 | 1 | 10 |
| B2  | 3 | 3 | 3 | 2 | 11 |
| B3  | 3 | 2 | 3 | 1 | 9 |
| B4  | 3 | 3 | 3 | 1 | 10 |
| B5  | 3 | 3 | 3 | 1 | 10 |
| **B median** | **3** | **3** | **3** | **1** | **10** |

### Quality Statistics

Quality totals are statistically equivalent between conditions (both median 10).

Per-dimension observations:
- **structural_accuracy:** B slightly higher (median 3 vs 3, but A has two 2s, B has zero).
- **cross_module_tracing:** B slightly higher (median 3 vs 2), suggesting code-analyze-mcp's
  structured output aids cross-module understanding.
- **approach_quality:** Ceiling effect (all 10 runs scored 3/3).
- **tool_efficiency:** A slightly higher (median 2 vs 1), because A runs averaged fewer
  analyze calls (median 9 vs 11).

## Efficiency Results

### Per-Run Efficiency (from session database)

| Run | Condition | Tokens | Wall (s) | Analyze | Shell | Editor | Total Calls |
|-----|-----------|-------:|:--------:|:-------:|:-----:|:------:|:-----------:|
| A1  | A-control | 27,203 | 74 | 8 | 7 | 1 | 16 |
| A2  | A-control | 26,667 | 75 | 9 | 4 | 1 | 14 |
| A3  | A-control | 21,969 | 70 | 12 | 2 | 1 | 15 |
| A4  | A-control | 25,818 | 68 | 10 | 4 | 1 | 15 |
| A5  | A-control | 22,150 | 70 | 9 | 8 | 1 | 18 |
| B1  | B-treatment | 29,039 | 61 | 12 | 1 | 1 | 14 |
| B2  | B-treatment | 31,620 | 79 | 10 | 0 | 1 | 11 |
| B3  | B-treatment | 31,836 | 70 | 11 | 0 | 1 | 12 |
| B4  | B-treatment | 30,287 | 73 | 11 | 1 | 1 | 14 |
| B5  | B-treatment | 33,046 | 85 | 13 | 1 | 1 | 16 |

### Efficiency Statistics

| Metric | A median | B median | U | z | p | r | Significant? |
|--------|:--------:|:--------:|:-:|:-:|:-:|:-:|:------------:|
| Total tokens | 25,818 | 31,620 | 0.0 | -2.61 | 0.008 | 1.00 | **Yes** |
| Wall time (s) | 70 | 73 | 10.0 | -0.52 | 0.548 | 0.20 | No |
| Total tool calls | 15 | 14 | 20.5 | 1.67 | 0.095 | -0.64 | No |
| Analyze calls | 9 | 11 | 4.0 | -1.78 | 0.075 | 0.68 | No |
| Shell calls | 4 | 1 | 25.0 | 2.61 | 0.008 | -1.00 | **Yes** |

### Derived Metrics

| Metric | A median | B median |
|--------|:--------:|:--------:|
| Tokens per quality point | 2,582 | 3,162 |
| Quality per tool call | 0.67 | 0.71 |

## Cross-Version Comparisons

### v5B vs v3B (Optimization Delta)

| Metric | v3B | v5B | Delta | Interpretation |
|--------|:---:|:---:|:-----:|----------------|
| Quality (0-12) | 10 | 10 | 0 | No regression |
| Total tool calls | 18 | 14 | -4 | 22% fewer calls |
| Shell calls | 6 | 1 | -5 | rg-blocking eliminated structural shell usage |

The rg-blocking constraint achieved its design goal: v5B uses 22% fewer total tool calls
than v3B while maintaining identical quality. The reduction comes entirely from eliminating
supplementary shell calls (rg, cat) that v3B agents used to compensate for uncertainty.

### v5B vs v3A (Gap Closure)

| Metric | v3A | v5B | Gap | Interpretation |
|--------|:---:|:---:|:---:|----------------|
| Quality (0-12) | 10 | 10 | 0 | Parity achieved |
| Total tool calls | 14 | 14 | 0 | Full parity |
| Shell calls | 2 | 1 | -1 | B slightly leaner |

v5B fully closes the efficiency gap with v3A. Total tool calls are identical (median 14).
Quality is identical (median 10). The only remaining difference is token overhead (B uses
~22% more tokens due to code-analyze-mcp's more verbose output format).

## Analysis

1. **rg-blocking works.** Condition B shell calls dropped from median 6 (v3B) to median 1
   (v5B). The single remaining shell call across B runs is for `mkdir` or `write` operations,
   not structural analysis. The constraint successfully forced the agent to rely exclusively
   on code-analyze-mcp for structural queries.

2. **Total tool calls reach parity.** v5B median total calls (14) match v3A (14) exactly.
   The v3 experiment found B used 29% more calls than A (18 vs 14); v5 eliminates this gap.
   The rg-blocking constraint prevents the agent from "double-checking" code-analyze-mcp
   results with shell commands.

3. **Token overhead persists.** code-analyze-mcp still costs 22% more tokens (31,620 vs
   25,818, p=0.008). This is structural: code-analyze-mcp returns richer output (LOC counts,
   function signatures, class hierarchies, call graphs) that inflates context. The native
   analyze tool returns comparable information more compactly.

4. **Wall time is now equivalent.** v3 showed a significant wall time difference (80s vs 61s,
   p=0.047). v5 shows no significant difference (73s vs 70s, p=0.548). The elimination of
   supplementary shell calls reduced B's latency overhead.

5. **code-analyze-mcp aids cross-module tracing.** B scored median 3 on cross_module_tracing
   vs A's median 2. While not statistically significant at n=5, this suggests code-analyze-mcp's
   structured call graph output (symbol_focus mode) provides better cross-module context than
   the native analyze tool.

6. **B uses more analyze calls but fewer total calls.** B makes median 11 analyze calls vs
   A's 9 (p=0.075, borderline), but compensates by making almost no shell calls. The net effect
   is equal total calls. This trade is favorable: analyze calls are more focused and structured
   than shell commands.

7. **Approach quality is a ceiling effect.** All 10 runs scored 3/3 on approach_quality. The
   rubric threshold is too lenient for this task at this model capability level. Future
   benchmarks should raise the bar (e.g., require implementation sketch, not just file list).

## Decision Framework Application

Per methodology.md:

| Scenario | Quality | Efficiency | Match? |
|----------|---------|------------|--------|
| v5B >= v3A quality AND v5B < v3B calls | v5B = v3A (10 = 10) | v5B < v3B (14 < 18) | **Yes** |

**Recommendation: Deploy code-analyze-mcp as recommended MCP tool with rg-blocking constraint.**

The v5 experiment demonstrates that code-analyze-mcp, when properly constrained to prevent
tool redundancy, achieves full parity with the native analyze baseline on quality and
efficiency. The remaining token overhead (22%) is a structural property of the tool's output
format, not a behavioral inefficiency.

### Actionable next steps

1. **Reduce output verbosity.** The 22% token overhead is the last remaining gap. Compact
   output modes (summary flags, truncated signatures) could close it.
2. **Raise approach_quality rubric.** All runs scored 3/3; the rubric needs a higher bar.
3. **Test at scale.** This benchmark uses a ~13K LOC repo. code-analyze-mcp's tree-sitter
   parsing may show greater advantage on 50K+ LOC codebases where native analyze degrades.
4. **Test SymbolFocus mode.** v5B's cross_module_tracing advantage (median 3 vs 2) hints
   that code-analyze-mcp's call graph analysis provides structural value. Wave 3 (SymbolFocus)
   should amplify this.

## Comparison with v3

| Metric | v3A | v3B | v5A | v5B | v3 Finding | v5 Finding |
|--------|:---:|:---:|:---:|:---:|------------|------------|
| Quality | 10 | 10 | 10 | 10 | Same | Same |
| Total calls | 14 | 18 | 15 | 14 | B worse (p=0.037) | Equivalent (p=0.095) |
| Shell calls | 2 | 6 | 4 | 1 | B higher (p=0.076) | B lower (p=0.008) |
| Tokens | 23,969 | 31,005 | 25,818 | 31,620 | B worse (p=0.016) | B worse (p=0.008) |
| Wall time | 61 | 80 | 70 | 73 | B worse (p=0.047) | Equivalent (p=0.548) |

v3 conclusion: "code-analyze-mcp adds no value; it costs more."
v5 conclusion: "code-analyze-mcp achieves parity when tool isolation is enforced."

The difference is entirely attributable to the rg-blocking constraint eliminating redundant
shell calls.

## Artifacts

- `prompts/` -- condition A and B prompts, task description
- `results/runs/` -- raw JSON reports per run (A1-A5, B1-B5)
- `results/blinded-scores.json` -- blinded scoring output with justifications
- `scores.json` -- unblinded scores with statistics and cross-version comparisons
- `scores-template.json` -- scoring template with rubric and v3 baselines
- `scripts/analyze.py` -- statistical analysis script
- `scripts/collect.py` -- session metrics extraction script
- `scripts/validate.py` -- tool isolation validation script
- `run-order.txt` -- randomized execution order (seed=124)
- `methodology.md` -- experiment design and decision framework
- `analysis.md` -- this document
