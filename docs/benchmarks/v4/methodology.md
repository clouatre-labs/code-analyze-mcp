# Benchmark v4: Validation of Output Verbosity Optimizations

## Experiment Design

v4 is a **Condition B-only replication** measuring the impact of output verbosity optimizations
(PRs #82, #83, #109, #112) on code-analyze-mcp efficiency. We do not re-run Condition A
(developer__analyze); instead, we embed v3 Condition A baselines for comparison.

| Parameter | Value |
|-----------|-------|
| Target repo | lsd-rs/lsd (~13K LOC, 52 Rust source files) |
| Task | Cross-module research: module map, data flow, dependency hubs, change proposal |
| Model | Claude Haiku 4.5, temperature 0.5 |
| Provider | AWS Bedrock |
| Repetitions | n=5 Condition B only |
| Condition B (treatment) | `code-analyze-mcp__analyze` (post-optimization) |
| Run order | Randomized per `run-order.txt` (seed=42) |
| Blinding | To be applied during scoring (condition labels stripped) |

## Comparison Framework

### v3 Baselines (Embedded for Reference)

v3 Condition A (developer__analyze, n=5):
- Quality median: 10/12
- Tokens median: 23,969
- Wall time median: 61s
- Tool calls median: 14

v3 Condition B (code-analyze-mcp, unoptimized, n=5):
- Quality median: 10/12
- Tokens median: 31,005
- Wall time median: 80s
- Tool calls median: 18

### v4 Comparison Strategy

**Primary comparison:** v4 B vs v3 B (optimization delta)
- Hypothesis: Output verbosity optimizations reduce token consumption and wall time while maintaining quality.
- Target: Reduce v4 B tokens to <28,000 (10% improvement over v3 B median of 31,005).
- Target: Reduce v4 B wall time to <75s (6% improvement over v3 B median of 80s).

**Secondary comparison:** v4 B vs v3 A (gap closure)
- If v4 B achieves v3 A efficiency levels, code-analyze-mcp becomes competitive.
- Success criterion: v4 B tokens within 5% of v3 A (i.e., <25,170).

## Scoring Rubric

Identical to v3 blinded-scores.json scoring_notes. Four dimensions, 0-3 points each:

1. **Structural accuracy (0-3):** Correctness of module map, submodule relationships, key types.
   - 0: Missing or incorrect module structure
   - 1: Partial module map, some relationships wrong
   - 2: Correct module map with minor gaps
   - 3: Complete, accurate module map with all relationships

2. **Cross-module tracing (0-3):** Quality of data flow trace and dependency hub identification.
   - 0: No data flow or hubs identified
   - 1: Partial data flow, weak hub analysis
   - 2: Clear data flow with 2-3 hubs identified
   - 3: Complete data flow trace with well-justified hub analysis

3. **Approach quality (0-3):** Clarity of reasoning, tool usage strategy, and change proposal feasibility.
   - 0: Incoherent or missing approach
   - 1: Weak approach, vague change proposal
   - 2: Reasonable approach, feasible change proposal
   - 3: Clear strategy, well-justified change proposal with risk analysis

4. **Tool efficiency (0-3):** Pragmatic use of code-analyze-mcp and supplementary tools.
   - 0: Excessive tool calls or poor strategy
   - 1: Some inefficiency, redundant calls
   - 2: Reasonable efficiency, <10 analyze calls
   - 3: Efficient use, <=8 analyze calls

**Total score:** Sum of four dimensions (0-12 points).

## Statistical Analysis Plan

### Descriptive Statistics

For v4 B (n=5):
- Per-run quality scores (0-12)
- Per-run efficiency metrics: tokens, wall time, tool call counts
- Median and range for each metric

### Comparison to v3 Baselines

1. **Quality:** Compare v4 B median to v3 B median (expect no difference; quality is not the target).
2. **Tokens:** Compare v4 B median to v3 B median (expect 10% reduction).
3. **Wall time:** Compare v4 B median to v3 B median (expect 6% reduction).
4. **Tool calls:** Compare v4 B median to v3 B median (expect similar or slight reduction).

### Interpretation

- If v4 B quality >= v3 B quality AND v4 B tokens < 28,000: Optimizations successful.
- If v4 B quality >= v3 B quality AND v4 B tokens < 25,170: Competitive with v3 A.
- If v4 B quality < v3 B quality: Optimizations degraded quality; further work needed.

## Caveats

1. **n=5 is statistically underpowered.** With only 5 runs, we cannot detect small effects
   (e.g., 5-10% token reduction) with confidence. Results are descriptive, not inferential.

2. **v3 A baseline is assumed valid.** We do not re-run Condition A. If developer__analyze
   behavior has changed since v3, the comparison is invalid.

3. **lsd-rs/lsd may have changed.** The target repository was last analyzed 6 months ago.
   If the codebase has evolved significantly, the task difficulty may differ from v3.

4. **Model stochasticity.** Claude Haiku 4.5 is non-deterministic. Even with identical
   prompts and temperature, results will vary. v4 B results may differ from v3 B due to
   randomness, not optimization impact.

## What Happens If Targets Are Not Met

### Scenario 1: v4 B quality drops below v3 B

**Action:** Revert optimizations; investigate which PR(s) caused degradation.
Conduct targeted fix and re-test with v4b.

### Scenario 2: v4 B tokens remain >31,000 (no improvement)

**Action:** Optimizations had minimal impact. Consider:
- Further output reduction (e.g., omit LOC counts, function signatures).
- Caching strategies to reduce redundant analyses.
- Larger-scale testing (50K+ LOC) to see if verbosity overhead grows with codebase size.

### Scenario 3: v4 B quality drops AND tokens remain high

**Action:** Optimizations failed. Archive code-analyze-mcp for this use case.
Consider v5 full benchmark (Condition A + B) with different optimization strategy.

## Artifacts

- `prompts/task.md` -- identical to v3 task (same lsd-rs/lsd target, same 4 questions)
- `prompts/condition-b-treatment.md` -- v4 Condition B prompt (optimized code-analyze-mcp)
- `run-order.txt` -- randomized execution order for 5 Condition B runs
- `scores-template.json` -- template for recording scores after runs complete
- `results/runs/` -- raw JSON reports per run (B1-B5)
- `methodology.md` -- this document
