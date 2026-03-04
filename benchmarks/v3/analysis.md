# Statistical Analysis: Benchmark v3 Results

## Executive Summary

No statistically significant difference was found between Condition A (developer__analyze) and Condition B (code-analyze-mcp) on the bat data flow analysis task. The Mann-Whitney U test yielded U=8.5, p>0.05, with a small-to-medium effect size (r=0.320). Both conditions produced comparable results across all four scoring dimensions.

**Critical confound:** Both conditions used the same underlying tool (`developer__analyze` from the `developer` extension). The `code-analyze-mcp` MCP server was not separately loaded in this session. Tool isolation was enforced only by prompt instructions, not by extension filtering. This renders the experiment a test of prompt variation under identical tooling, not a comparison of two different analysis tools.

## Methodology

### Sample
- Condition A (Control): n=5 runs (A1-A5)
- Condition B (Treatment): n=5 runs (B1-B5)
- Total: N=10 runs
- Scoring: Blinded to condition; randomized order (seed=42)

### Scoring Dimensions
1. Structural Accuracy (0-3)
2. Cross-Module Tracing (0-3)
3. Approach Quality (0-3)
4. Tool Efficiency (0-3)
- Total Score: Sum of 4 dimensions (0-12)

### Statistical Test
Mann-Whitney U test (non-parametric, appropriate for small samples)
- Null hypothesis: No difference in total scores between conditions
- Alternative hypothesis: Significant difference exists
- Significance level: alpha = 0.05
- Effect size: Rank-biserial correlation

## Results

### Descriptive Statistics

#### Condition A (developer__analyze)
- Runs: A1, A2, A3, A4, A5
- Total Scores: [8, 7, 8, 7, 9]
- Median: 8
- Range: 7 - 9
- Mean: 7.8
- Std Dev: 0.75

#### Condition B (code-analyze-mcp)
- Runs: B1, B2, B3, B4, B5
- Total Scores: [7, 7, 7, 7, 9]
- Median: 7
- Range: 7 - 9
- Mean: 7.4
- Std Dev: 0.80

### Per-Dimension Comparison

#### Structural Accuracy
- Condition A Median: 2
- Condition B Median: 2
- Difference: 0

#### Cross-Module Tracing
- Condition A Median: 2
- Condition B Median: 2
- Difference: 0

#### Approach Quality
- Condition A Median: 1
- Condition B Median: 1
- Difference: 0

#### Tool Efficiency
- Condition A Median: 2
- Condition B Median: 2
- Difference: 0

### Wall Time Comparison

#### Condition A
- Wall Times (seconds): [71, 65, 77, 70, 71]
- Median: 71s
- Mean: 70.8s

#### Condition B
- Wall Times (seconds): [69, 74, 65, 71, 70]
- Median: 70s
- Mean: 69.8s

### Mann-Whitney U Test Results

```
R_A (sum of Condition A ranks): 31.5
R_B (sum of Condition B ranks): 23.5
U_A: 8.5
U_B: 16.5
U-statistic (min): 8.5
Z-score: -0.836
p-value: >0.05 (critical U for n1=5, n2=5 at alpha=0.05 two-tailed: U <= 2)
Rank-biserial correlation (effect size): 0.320
Interpretation: Small-to-medium effect size, not statistically significant
```

### Decision

**No significant difference detected (p > 0.05).**

Per the decision framework: the Mann-Whitney U p-value exceeds 0.05. The rank-biserial correlation of 0.320 suggests a small-to-medium effect favoring Condition A, but this is not statistically significant given the sample size.

## Per-Dimension Detail

### Structural Accuracy
- Condition A: [2, 2, 2, 2, 2]
- Condition B: [2, 2, 2, 2, 2]

All 10 runs scored identically (2/3). Every run correctly identified the core modules (controller, printer, decorations, output, input, config, assets, preprocessor) but none achieved a perfect 3 (which would require identifying all modules with precise roles and no errors).

### Cross-Module Tracing
- Condition A: [2, 2, 3, 2, 2]
- Condition B: [2, 2, 2, 2, 2]

One Condition A run (A3) scored 3/3 with 16 well-traced interactions including lessopen and theme detection paths. All other runs scored 2/3, correctly tracing the main pipeline but missing some secondary interactions or key types.

### Approach Quality
- Condition A: [2, 1, 1, 1, 3]
- Condition B: [1, 1, 1, 1, 3]

This dimension showed the most variance. Only 2 of 10 runs (A5 and B5) identified the Printer trait as the extension point and proposed an HtmlPrinter implementation (score 3). One run (A1) proposed a JsonPrinter implementing the Printer trait but framed it as modifying printer.rs (score 2). The remaining 7 runs proposed new OutputFormatter traits or plugin systems without recognizing the existing Printer trait abstraction (score 1).

### Tool Efficiency
- Condition A: [2, 2, 2, 2, 2]
- Condition B: [2, 2, 2, 2, 2]

All 10 runs scored identically (2/3). All used the analyze tool systematically (directory overview, then module details, then focus queries) with minimal redundancy. None achieved 3/3 (which would require perfectly targeted queries with zero waste).

## Discussion

### Interpretation

The results show no meaningful difference between conditions, which is expected given the critical confound: both conditions used the identical `developer__analyze` tool. The experiment was designed to compare two different tools (the built-in goose `developer__analyze` vs. the `code-analyze-mcp` MCP server's `analyze` tool), but the MCP server extension was not separately loaded in the session. As a result, both conditions exercised the same code path.

The slight numerical advantage of Condition A (median 8 vs. 7) is driven entirely by one run (A3) scoring 3/3 on cross-module tracing, which is within normal temperature-induced variance at 0.5.

### Limitations

1. **Critical confound:** Both conditions used the same tool. The experiment does not test what it was designed to test.
2. **Small sample size:** n=5 per condition limits statistical power
3. **Single scorer:** No inter-rater reliability; blinding mitigates but does not eliminate bias
4. **Temperature variability:** Temperature 0.5 introduces randomness; results not deterministic
5. **Task specificity:** Results apply to bat data flow analysis; generalization limited
6. **Prompt-only isolation:** Tool restriction was enforced by prompt text, not by extension filtering

### Implications for Tool Isolation

The experiment demonstrates that prompt-based tool isolation is insufficient for a valid A/B comparison. To properly compare `developer__analyze` vs. `code-analyze-mcp`, the experiment must:
1. Load the `code-analyze-mcp` extension as a separate MCP server in the session
2. Use extension filtering in the delegate to restrict tool access (not just prompt instructions)
3. Verify that Condition B delegates can only access `code-analyze-mcp` tools, not `developer` tools

### Recommendations

1. Re-run the experiment with the `code-analyze-mcp` extension properly loaded as a separate MCP server
2. Use the delegate's `extensions` parameter to enforce tool isolation at the platform level
3. Consider increasing sample size to n=10 per condition for greater statistical power
4. Add a manipulation check: verify which tool was actually called in each run's trace

## Appendix: Calculation Details

### Mann-Whitney U Test Procedure

1. Combine all 10 total scores and rank from lowest to highest
2. Sum ranks for Condition A: R_A = 31.5
3. Sum ranks for Condition B: R_B = 23.5
4. Calculate U_A = n_A * n_B + (n_A * (n_A + 1)) / 2 - R_A = 25 + 15 - 31.5 = 8.5
5. Calculate U_B = n_A * n_B + (n_B * (n_B + 1)) / 2 - R_B = 25 + 15 - 23.5 = 16.5
6. U = min(U_A, U_B) = 8.5
7. Critical value for n1=5, n2=5 at alpha=0.05 (two-tailed): U <= 2
8. Since 8.5 > 2, fail to reject null hypothesis
9. Rank-biserial correlation: r = 1 - (2 * 8.5) / (5 * 5) = 1 - 0.68 = 0.320

### Blinding Mapping

| Blinded ID | Run ID | Condition |
|------------|--------|-----------|
| R1 | A5 | A |
| R2 | B4 | B |
| R3 | A3 | A |
| R4 | A1 | A |
| R5 | B2 | B |
| R6 | B5 | B |
| R7 | A2 | A |
| R8 | B1 | B |
| R9 | B3 | B |
| R10 | A4 | A |

### Scoring Audit Trail

| Run | Struct | Cross | Approach | Tool | Total | Key Observation |
|-----|--------|-------|----------|------|-------|-----------------|
| A1 | 2 | 2 | 2 | 2 | 8 | JsonPrinter implementing Printer trait; some fabricated names |
| A2 | 2 | 2 | 1 | 2 | 7 | OutputFormatter trait proposed, not Printer |
| A3 | 2 | 3 | 1 | 2 | 8 | Best cross-module tracing; Formatter trait, not Printer |
| A4 | 2 | 2 | 1 | 2 | 7 | OutputFormatter trait proposed |
| A5 | 2 | 2 | 3 | 2 | 9 | HtmlPrinter implementing Printer trait; best approach |
| B1 | 2 | 2 | 1 | 2 | 7 | Filter/transform proposal, different feature entirely |
| B2 | 2 | 2 | 1 | 2 | 7 | OutputFormatter trait proposed |
| B3 | 2 | 2 | 1 | 2 | 7 | Plugin system proposed |
| B4 | 2 | 2 | 1 | 2 | 7 | OutputFormatter trait proposed |
| B5 | 2 | 2 | 3 | 2 | 9 | HtmlPrinter implementing Printer trait; best approach |

## References

- Mann-Whitney U test: https://en.wikipedia.org/wiki/Mann%E2%80%93Whitney_U_test
- Rank-biserial correlation: https://en.wikipedia.org/wiki/Rank-biserial_correlation
- Benchmark v3 design: README.md
- Scoring rubric: rubric.md
- Ground truth: ground-truth.md
