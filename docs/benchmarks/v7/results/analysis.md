# Analysis: v7-benchmark

**Model:** claude-haiku-4-5
**Runs:** 10 (5 per condition)
**Analysis mode:** v7
**Base version for comparison:** V6

## Quality Analysis

| Dimension | Cond A | Cond B | U-stat | z | p-val | r |
|-----------|--------|--------|--------|-------|----------|---------|
| structural_accuracy | 3.0 | 2.0 | 15.0 | 0.52 | 0.631 | -0.20 |
| cross_module_tracing | 3.0 | 2.0 | 17.5 | 1.04 | 0.270 | -0.40 |
| approach_quality | 3.0 | 3.0 | 10.0 | -0.52 | 0.600 | 0.20 |
| tool_efficiency | 2.0 | 2.0 | 17.5 | 1.04 | 0.177 | -0.40 |
| **total** | **10.0** | **9.0** | **17.5** | **1.04** | **0.332** | **-0.40** |

## Efficiency Analysis

Tool efficiency from per_run_scores[run_id].efficiency.total_calls (if available)

## Tool Efficiency (4-point dimension)

| Condition | Median | Range | Scores |
|-----------|--------|-------|--------|
| A | 2.0 | 2-2 | [2, 2, 2, 2, 2] |
| B | 2.0 | 1-2 | [2, 2, 1, 2, 1] |

## Parameter Usage Frequency (v7 Condition B)

| Parameter | % of B Runs | Count |
|-----------|-------------|-------|
| summary=true | 80% | 4/5 |
| cursor | 0% | 0/5 |
| page_size | 0% | 0/5 |

## Cross-Version Comparison

### v7B vs V6B (Parameter Optimization Delta)

| Metric | V6B | v7B | Delta |
|--------|-----|-----|-------|
| Quality (total) | 10 | 9 | -1.0 |

### v7B vs v7A (Gap Closure)

| Metric | v7A | v7B | Gap |
|--------|-----|-----|-----|
| Quality | 10 | 9 | -1.0 |
| **Hypothesis: v7B agents use new parameters (summary, cursor, page_size) to reduce token overhead compared to v6B** | | | |

---

*Note: Mann-Whitney U test requires scipy for exact p-values. If scipy unavailable, p-value is approximate.*
*rank-biserial r: |r| >= 0.3 (small), >= 0.5 (medium), >= 0.7 (large effect).*
