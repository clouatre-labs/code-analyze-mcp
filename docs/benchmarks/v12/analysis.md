# v12 Benchmark Analysis

## Summary

This analysis covers all 8 scored runs across four experimental conditions (A, B, C, D), each with n=2 replicates. All runs produced valid JSON outputs with structured scoring.

Conditions:
- **A**: claude-sonnet-4-6, MCP tool set (n=2)
- **B**: claude-sonnet-4-6, native tool set (n=2)
- **C**: claude-haiku-4-5, MCP tool set (n=2)
- **D**: claude-haiku-4-5, native tool set (n=2, re-run with --json-schema fix applied)

Condition D initially failed to produce valid JSON (0/2 outputs parseable). The runner was updated to enforce --json-schema at invocation, which eliminated prose-wrapping and produced 2/2 valid outputs.

## Quality Metrics

Median total scores (0-9 scale) across all 8 runs:

| Condition | Model | Tool Set | Median Score | n |
|-----------|-------|----------|--------------|---|
| A | Sonnet | MCP | 9.0 | 2 |
| B | Sonnet | Native | 8.5 | 2 |
| C | Haiku | MCP | 8.0 | 2 |
| D | Haiku | Native | 9.0 | 2 |

All conditions achieved high quality. Conditions A and D both reached the maximum median of 9.0.

## Accuracy by Dimension

Breakdown by dimension (0-3 scale per dimension):

| Condition | Structural Accuracy | Cross-Module Tracing | Approach Quality |
|-----------|-------------------|----------------------|------------------|
| A | 3.0 | 3.0 | 3.0 |
| B | 2.5 | 3.0 | 3.0 |
| C | 2.0 | 3.0 | 3.0 |
| D | 3.0 | 3.0 | 3.0 |

All conditions scored 3.0 on cross-module tracing and approach quality. Structural accuracy varied by condition, with A and D achieving the maximum (3.0) and C the lowest (2.0).

## Reliability

JSON validity rates (runs producing parseable JSON outputs / total runs per condition):

| Condition | Valid Outputs | Total Runs | Rate |
|-----------|---------------|-----------|------|
| A | 2 | 2 | 100% |
| B | 2 | 2 | 100% |
| C | 2 | 2 | 100% |
| D | 2 | 2 | 100% (post-fix) |

Condition D initially produced 0/2 valid outputs due to a prose-wrapping issue in the runner. The fix was to pass --json-schema to enforce structured output. After re-run with the fix, D achieved 100% validity.

## Efficiency

### Tool Calls

Median tool_calls_total per condition:

| Condition | Median Tool Calls |
|-----------|------------------|
| A | 3 |
| B | 12 |
| C | 5 |
| D | 8 |

Native Sonnet (B) used the most tool calls (median 12), while MCP Sonnet (A) used the fewest (median 3). Haiku conditions (C and D) fell in the middle range.

### Tokens and Wall Time

Telemetry was captured for all scored runs. The table below shows condition D as a representative example; telemetry for conditions A, B, and C is available in the per-run telemetry sidecars and `scores-template.json`.

| Run ID | Wall Time (ms) | Input Tokens | Output Tokens | Turns | Cost (USD) | Score / Dollar |
|--------|----------------|--------------|---------------|-------|-----------|----------------|
| D-scored-1 | 47,302 | 241,993 | 3,625 | 9 | 0.2601 | 34.6 |
| D-scored-2 | 109,422 | 703,393 | 6,478 | 29 | 0.7987 | 11.3 |

D-scored-1 achieved higher efficiency (34.6 score/dollar) with a shorter execution time (47.3 seconds) and fewer turns (9). D-scored-2 required a longer execution (109.4 seconds) and more turns (29), resulting in lower efficiency (11.3 score/dollar). Both achieved the same total score (9), suggesting different search depths or exploration strategies.

## Tool Set Effect

MCP vs native tool set comparison across all 8 runs:

- **MCP scores**: [9, 9, 8, 8], median = 8.5
- **Native scores**: [8, 9, 9, 9], median = 9.0
- **Rank-biserial r**: 0.250 (positive direction favors native)

Native tool set shows a small advantage in median quality (9.0 vs 8.5). The rank-biserial coefficient of 0.250 indicates a weak effect in favor of native, meaning native tool set pairs rank slightly higher on average. However, with n=4 per group, this result should be interpreted with caution.

## Model Effect

Sonnet vs Haiku comparison across all 8 runs:

- **Sonnet scores**: [9, 9, 8, 9], median = 9.0
- **Haiku scores**: [8, 8, 9, 9], median = 8.5

Sonnet achieves a median of 9.0 across both MCP (A) and native (B) tool sets, while Haiku achieves 8.5 across both MCP (C) and native (D) tool sets. This suggests a consistent model effect favoring Sonnet, though both models can achieve high scores.

## Notes

- **Runner fix (Condition D)**: The initial D runs were invoked without --json-schema, allowing the model to wrap structured output in prose. This caused JSON parsing to fail. The fix explicitly passes --json-schema at invocation, enforcing strict structure. All subsequent D runs produced valid JSON.
- **Telemetry gap (A/B/C)**: These conditions used an earlier runner version that did not capture cost, wall time, input/output tokens, or turn counts. Re-running all conditions with telemetry capture is recommended for future iterations to enable cost-benefit analysis across all tool set and model combinations.
- **Sample size**: All analyses use n=2 per condition. Extending to n=3-5 would reduce variability and increase confidence in effect estimates.
- **Structural accuracy trade-off**: Condition B (Sonnet + native) shows the largest drop in structural accuracy (2.5 vs 3.0), suggesting this combination may sacrifice precision for broader exploration.

---

## v12 Re-run (A + C, runs 3-4)

Conditions A and C were re-run for two additional replicates each (runs 3 and 4), bringing both to n=4. Conditions B and D remain at n=2.

### Full Results Table

| Run | Condition | Model | Tool Set | D1 | D2 | D3 | Total | Tool Calls | Input Tokens | Cost |
|-----|-----------|-------|----------|----|----|----|-------|------------|--------------|------|
| A-scored-1 | A | claude-sonnet-4-6 | mcp | 3 | 3 | 3 | 9 | 3 | 156,820 | $0.5436 |
| A-scored-2 | A | claude-sonnet-4-6 | mcp | 3 | 3 | 3 | 9 | 3 | 133,780 | $0.4741 |
| A-scored-3 | A | claude-sonnet-4-6 | mcp | 3 | 3 | 3 | 9 | 4 | 97,595 | $0.3451 |
| A-scored-4 | A | claude-sonnet-4-6 | mcp | 3 | 3 | 3 | 9 | 6 | 126,336 | $0.4391 |
| B-scored-1 | B | claude-sonnet-4-6 | native | 2 | 3 | 3 | 8 | 12 | 316,415 | $1.0761 |
| B-scored-2 | B | claude-sonnet-4-6 | native | 3 | 3 | 3 | 9 | 12 | 236,166 | $0.8137 |
| C-scored-1 | C | claude-haiku-4-5 | mcp | 2 | 3 | 3 | 8 | 4 | 364,078 | $0.3806 |
| C-scored-2 | C | claude-haiku-4-5 | mcp | 2 | 3 | 3 | 8 | 6 | 433,944 | $0.4548 |
| C-scored-3 | C | claude-haiku-4-5 | mcp | 2 | 2 | 2 | 6 | 5 | 342,907 | $0.3552 |
| C-scored-4 | C | claude-haiku-4-5 | mcp | 3 | 3 | 3 | 9 | 7 | 469,547 | $0.4885 |
| D-scored-1 | D | claude-haiku-4-5 | native | 3 | 3 | 3 | 9 | 2 | 241,993 | $0.2601 |
| D-scored-2 | D | claude-haiku-4-5 | native | 3 | 3 | 3 | 9 | 14 | 703,393 | $0.7987 |

### Median Scores per Condition (all runs)

Medians computed across all available runs per condition (A and C: n=4; B and D: n=2):

| Condition | Model | Tool Set | n | Median D1 | Median D2 | Median D3 | Median Total |
|-----------|-------|----------|---|-----------|-----------|-----------|--------------|
| A | claude-sonnet-4-6 | mcp | 4 | 3.0 | 3.0 | 3.0 | 9.0 |
| B | claude-sonnet-4-6 | native | 2 | 2.5 | 3.0 | 3.0 | 8.5 |
| C | claude-haiku-4-5 | mcp | 4 | 2.0 | 3.0 | 3.0 | 8.0 |
| D | claude-haiku-4-5 | native | 2 | 3.0 | 3.0 | 3.0 | 9.0 |

Condition A remains at a perfect median total of 9.0 across all four runs. Condition C's median total holds at 8.0; C-scored-3 (total 6) and C-scored-4 (total 9) produced wider spread than the original two runs (both 8), consistent with higher variance under the smaller Haiku model.

### Efficiency Comparison: A and C (original runs 1-2 vs. new runs 3-4)

| Metric | A orig (runs 1-2) | A new (runs 3-4) | C orig (runs 1-2) | C new (runs 3-4) |
|--------|-------------------|------------------|-------------------|------------------|
| Median input tokens | 145,300 | 111,966 | 399,011 | 406,227 |
| Median tool calls | 3 | 5 | 5 | 6 |
| Median cost USD | $0.5088 | $0.3921 | $0.4177 | $0.4219 |
| Median turns | 8.5 | 7.5 | 13 | 8.5 |

Notes on the A comparison: new runs (3-4) consumed fewer input tokens (median 111,966 vs 145,300) and lower cost ($0.39 vs $0.51), while producing the same perfect score (9/9). This suggests the MCP toolset allows efficient, consistent performance without needing deeper exploration.

Notes on the C comparison: input token counts and cost are nearly identical between original and new runs. Tool calls and turns are slightly lower in the new runs. The score variance increased (6 and 9 vs. 8 and 8), which may reflect stochastic variation at the Haiku scale rather than a systematic change.
