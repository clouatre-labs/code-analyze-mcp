# Statistical Analysis: Benchmark v3 Results

## Executive Summary

This section will contain the results of the Mann-Whitney U test comparing Condition A (developer__analyze) and Condition B (code-analyze-mcp) on the bat data flow analysis task.

## Methodology

### Sample
- Condition A (Control): n=5 runs (A1-A5)
- Condition B (Treatment): n=5 runs (B1-B5)
- Total: N=10 runs
- Scoring: Blinded to condition; randomized order

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

## Results (To Be Filled)

### Descriptive Statistics

#### Condition A (developer__analyze)
- Runs: A1, A2, A3, A4, A5
- Total Scores: [__, __, __, __, __]
- Median: __
- Range: __ - __
- Mean: __
- Std Dev: __

#### Condition B (code-analyze-mcp)
- Runs: B1, B2, B3, B4, B5
- Total Scores: [__, __, __, __, __]
- Median: __
- Range: __ - __
- Mean: __
- Std Dev: __

### Per-Dimension Comparison

#### Structural Accuracy
- Condition A Median: __
- Condition B Median: __
- Difference: __

#### Cross-Module Tracing
- Condition A Median: __
- Condition B Median: __
- Difference: __

#### Approach Quality
- Condition A Median: __
- Condition B Median: __
- Difference: __

#### Tool Efficiency
- Condition A Median: __
- Condition B Median: __
- Difference: __

### Mann-Whitney U Test Results

```
U-statistic: __
p-value: __
Rank-biserial correlation (effect size): __
Interpretation: __
```

### Decision

Based on the Mann-Whitney U test:

- **p-value < 0.05:** Significant difference detected
  - Superior condition: __
  - Effect size interpretation: __
  - Conclusion: __

- **p-value >= 0.05:** No significant difference
  - Effect size: __
  - Conclusion: __

## Qualitative Analysis

### Tool Usage Patterns

#### Condition A (developer__analyze)
- Average tool calls per run: __
- Average tokens per run: __
- Common query patterns: __
- Observed strengths: __
- Observed limitations: __

#### Condition B (code-analyze-mcp)
- Average tool calls per run: __
- Average tokens per run: __
- Common query patterns: __
- Observed strengths: __
- Observed limitations: __

### Response Quality Observations

#### Structural Accuracy
- Condition A: __
- Condition B: __

#### Cross-Module Tracing
- Condition A: __
- Condition B: __

#### Approach Quality
- Condition A: __
- Condition B: __

#### Tool Efficiency
- Condition A: __
- Condition B: __

## Discussion

### Interpretation

[To be filled with interpretation of results]

### Limitations

1. **Small sample size:** n=5 per condition limits statistical power
2. **Single scorer:** No inter-rater reliability; blinding mitigates but does not eliminate bias
3. **Temperature variability:** Temperature 0.5 introduces randomness; results not deterministic
4. **Task specificity:** Results apply to bat data flow analysis; generalization limited
5. **Tool maturity:** code-analyze-mcp may be less mature than developer__analyze

### Implications for Tool Isolation

[To be filled with discussion of tool isolation effectiveness]

### Recommendations

1. [To be filled]
2. [To be filled]
3. [To be filled]

## Appendix: Calculation Details

### Mann-Whitney U Test Procedure

1. Combine all 10 total scores and rank from lowest to highest
2. Sum ranks for Condition A: R_A = __
3. Sum ranks for Condition B: R_B = __
4. Calculate U-statistic: U = n_A * n_B + (n_A * (n_A + 1)) / 2 - R_A
5. Calculate p-value using U-distribution with n_A=5, n_B=5
6. Calculate rank-biserial correlation: r = 1 - (2U) / (n_A * n_B)

### Scoring Audit Trail

[To be filled with per-run scoring details and rationale]

## References

- Mann-Whitney U test: https://en.wikipedia.org/wiki/Mann%E2%80%93Whitney_U_test
- Rank-biserial correlation: https://en.wikipedia.org/wiki/Rank-biserial_correlation
- Benchmark v3 design: README.md
- Scoring rubric: rubric.md
- Ground truth: ground-truth.md
