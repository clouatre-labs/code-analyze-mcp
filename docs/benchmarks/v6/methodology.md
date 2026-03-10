# v6 Benchmark Methodology

## Overview

This v6 benchmark extends the v5 experiment design to measure the impact of 5 lossless compaction changes on token overhead. The v5 benchmark found that code-analyze-mcp produced equivalent quality output (median scores: both A=10, B=10) but with 22% higher token usage. v6 tests whether targeted formatting improvements close this efficiency gap without quality regression.

## Compaction Context

Five lossless formatting changes were implemented to reduce token overhead while preserving semantic content:

- **#129**: Relative paths in all output modes (PR #135)
- **#130**: Tree-indent callees and callers in focus mode (PR #137)
- **#132**: Separate test callers into counted summary (PR #139)
- **#133**: Add summary counts to FOCUS header (PR #140)
- **#134**: Deduplicate repeated callee chains with (xN) annotation (PR #141)

These changes preserve all semantic information. The primary target was focus mode, where
v5 showed 73% more callee lines than native analyze for the same content: flat `X -> Y`
per line (repeating the parent on every depth-2 edge) vs tree indentation (one parent,
indented children). Secondary targets were absolute paths in all modes and test callers
mixed with production callers.

## Hypothesis

Lossless formatting improvements will reduce the v6B token overhead to <10% above v6A (Condition A) without quality regression. This would represent significant efficiency gain: from 22% overhead (v5B) to <10% overhead (v6B).

## Experimental Design (Unchanged from v5)

### Conditions

- **Condition A (Control)**: developer__analyze (goose built-in tool)
- **Condition B (Treatment)**: code-analyze-mcp__analyze with rg-blocking constraint

### Sample

- **Repository**: lsd-rs/lsd (same as v5)
- **Runs**: 10 (5 per condition)
- **Randomization**: seed=128 (distinct from v5 seed=124), conditions blinded before scoring

### Evaluation

**Rubric**: 4 dimensions x 4 levels (0-3) = 12 points max per run

1. **Structural Accuracy**: Module identification, data types, codebase structure alignment
2. **Cross-Module Tracing**: Call chain completeness, type flow accuracy, intermediate steps
3. **Approach Quality**: Change proposal reasoning, affected files, integration points, risk analysis
4. **Tool Efficiency**: Analysis tool call count (3=5 or fewer, 2=6-10, 1=11-20, 0=>20)

**Statistical Analysis**: Mann-Whitney U test (non-parametric, small n) to assess whether v6B differs from v6A in quality dimension and token efficiency.

## Outcomes

### Primary

- **Token Reduction**: Compare v6B token count vs v5B baseline (compaction delta)
- **Gap Closure**: Measure v6B vs v6A ratio relative to v5B vs v5A (22% baseline)

### Secondary

- **Quality Maintenance**: No regression in Condition B median score (expect A ≈ 10, B ≈ 10, no statistically significant difference)
- **Per-Dimension Analysis**: Tool efficiency should improve most; structural accuracy, cross-module tracing, and approach quality should be unaffected by formatting changes

## Cross-Version Comparisons

1. **v6B vs v5B**: Direct token comparison to quantify compaction impact
2. **v6B vs v6A**: Gap measurement to assess whether 22% overhead narrows to <10%

v5 baseline quality (median totals: A=10, B=10) embedded in scores-template.json as v5_baselines for reference during analysis.

## Tool Isolation

As in v5, both tools are run against the same repository snapshot with identical system prompt and rg-blocking constraint (applied to both). Tool isolation verified at run time; results stored in results/runs/{run_id}/.

## References

See [docs/benchmarks/v5/methodology.md](../v5/methodology.md) for unchanged aspects:
- Blinding procedure details
- Scoring process (independent evaluation, condition labels stripped)
- Rubric rationale
- Statistical test rationale
