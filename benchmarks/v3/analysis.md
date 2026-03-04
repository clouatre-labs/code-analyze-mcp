# Benchmark v3: code-analyze-mcp vs developer__analyze

## Verdict

**Same quality, worse efficiency.** code-analyze-mcp produces equivalent research output but
costs significantly more tokens, time, and tool calls to get there.

| Metric | A (developer__analyze) | B (code-analyze-mcp) | p-value | Significant? |
|--------|:----------------------:|:--------------------:|:-------:|:------------:|
| Quality (0-12) | median 10 | median 10 | 0.754 | No |
| Total tokens | median 23,969 | median 31,005 | 0.016 | **Yes** |
| Wall time (s) | median 61 | median 80 | 0.047 | **Yes** |
| Total tool calls | median 14 | median 18 | 0.037 | **Yes** |

code-analyze-mcp costs **29% more tokens** and **31% more wall time** for the same quality score.

## Experiment Design

| Parameter | Value |
|-----------|-------|
| Target repo | lsd-rs/lsd (~13K LOC, 52 Rust source files) |
| Task | Cross-module research: module map, data flow, dependency hubs, change proposal |
| Model | Claude Haiku 4.5, temperature 0.5 |
| Provider | AWS Bedrock |
| Repetitions | n=5 per condition (10 total) |
| Condition A (control) | `developer__analyze` (goose built-in) |
| Condition B (treatment) | `code-analyze-mcp__analyze` |
| Run order | Randomized per `run-order.txt` |
| Blinding | Condition labels stripped, random shuffle (seed=69) before scoring |

## Tool Isolation

Verified across all 10 runs via session database:

- All 5 Condition A runs used only `developer__analyze` (zero `code-analyze-mcp` calls)
- All 5 Condition B runs used only `code-analyze-mcp__analyze` (zero `developer__analyze` calls)
- No runs discarded

## Quality Results

### Per-Run Scores (0-12)

| Run | Structural | Tracing | Approach | Efficiency | Total |
|-----|:----------:|:-------:|:--------:|:----------:|:-----:|
| A1  | 2 | 3 | 2 | 3 | 10 |
| A2  | 3 | 2 | 1 | 3 | 9 |
| A3  | 1 | 3 | 3 | 3 | 10 |
| A4  | 3 | 3 | 1 | 3 | 10 |
| A5  | 2 | 2 | 2 | 3 | 9 |
| **A median** | **2** | **3** | **2** | **3** | **10** |
| B1  | 3 | 3 | 1 | 3 | 10 |
| B2  | 3 | 2 | 3 | 3 | 11 |
| B3  | 1 | 3 | 2 | 3 | 9 |
| B4  | 2 | 3 | 1 | 3 | 9 |
| B5  | 2 | 3 | 2 | 3 | 10 |
| **B median** | **2** | **3** | **2** | **3** | **10** |

### Quality Statistics

| Statistic | Value |
|-----------|-------|
| Test | Mann-Whitney U (two-tailed) |
| n per condition | 5 |
| U | 11.0 |
| z | -0.313 |
| p | 0.754 |
| Rank-biserial r | 0.120 (small) |
| Significant? | No |

## Efficiency Results

### Per-Run Efficiency (from session database)

| Run | Condition | Tokens | Wall (s) | Analyze | Shell | Editor | Total Calls |
|-----|-----------|-------:|:--------:|:-------:|:-----:|:------:|:-----------:|
| A1  | A-control | 23,969 | 59 | 6 | 2 | 5 | 13 |
| A2  | A-control | 29,827 | 61 | 4 | 0 | 11 | 15 |
| A3  | A-control | 26,239 | 65 | 7 | 0 | 7 | 14 |
| A4  | A-control | 22,761 | 82 | 6 | 5 | 6 | 17 |
| A5  | A-control | 20,535 | 59 | 6 | 2 | 4 | 12 |
| B1  | B-treatment | 30,651 | 77 | 7 | 1 | 7 | 15 |
| B2  | B-treatment | 32,341 | 78 | 7 | 3 | 6 | 16 |
| B3  | B-treatment | 37,947 | 109 | 6 | 10 | 5 | 21 |
| B4  | B-treatment | 27,746 | 80 | 6 | 9 | 6 | 21 |
| B5  | B-treatment | 31,005 | 90 | 10 | 6 | 2 | 18 |

### Efficiency Statistics

| Metric | A median | B median | U | z | p | r | Significant? |
|--------|:--------:|:--------:|:-:|:-:|:-:|:-:|:------------:|
| Total tokens | 23,969 | 31,005 | 1.0 | -2.402 | 0.016 | 0.920 | **Yes** |
| Wall time (s) | 61 | 80 | 3.0 | -1.984 | 0.047 | 0.760 | **Yes** |
| Total tool calls | 14 | 18 | 2.5 | -2.089 | 0.037 | 0.800 | **Yes** |
| Analyze calls | 6 | 7 | 6.0 | -1.358 | 0.175 | 0.520 | No |
| Shell calls | 2 | 6 | 4.0 | -1.776 | 0.076 | 0.680 | No |

### Derived Metrics

| Metric | A median | B median |
|--------|:--------:|:--------:|
| Tokens per quality point | 2,529 | 3,234 |
| Quality per tool call | 0.69 | 0.56 |

## Analysis

1. **code-analyze-mcp returns more verbose output.** Its responses include full file trees with
   LOC/function/class counts, detailed function signatures, and import lists. This inflates
   input tokens on subsequent turns as the context grows. developer__analyze returns similar
   information but more compactly.

2. **code-analyze-mcp delegates compensate with more shell calls.** The treatment condition
   used median 6 shell calls vs 2 for control (p=0.076, borderline). This suggests
   code-analyze-mcp's output format doesn't fully satisfy the LLM's information needs,
   requiring supplementary `rg` and file reads.

3. **Analyze call counts are similar.** Both tools were called a median of 6-7 times. The
   quality difference is not in how many structural analyses were done, but in the overhead
   per call and the supplementary work needed.

4. **Quality variance comes from the LLM, not the tool.** Both conditions show identical
   per-dimension median scores (structural=2, tracing=3, approach=2, efficiency=3) and
   similar variance. The stochastic exploration strategy of the model dominates quality
   outcomes.

5. **Tool efficiency rubric dimension is a ceiling effect.** All 10 runs scored 3/3 on
   tool efficiency. The rubric threshold (<8 analyze calls for max score) is too easy.

## Comparison with PR #65 Baseline

The PR #65 developer__analyze baseline (n=3) showed median 9/12. This experiment's Condition A
shows median 10/12 (n=5). The improvement is likely due to the bookend prompt pattern and turn
budget instruction, not a tool change.

## Recommended Next Steps

Per the decision framework from issue #69:

| Result | Action |
|--------|--------|
| No quality difference, worse efficiency | code-analyze-mcp adds no value; it costs more |

**Recommendation:** code-analyze-mcp does not justify its maintenance cost for research tasks
at this codebase scale (~13K LOC). The built-in developer__analyze achieves the same quality
with 29% fewer tokens and 31% less wall time.

Before archiving, consider:

1. **Test at larger scale (50K+ LOC).** developer__analyze may degrade on larger codebases
   where code-analyze-mcp's tree-sitter parsing could provide an advantage.
2. **Test SymbolFocus mode.** code-analyze-mcp's planned call-graph analysis (Wave 3) could
   differentiate on cross-module tracing tasks where developer__analyze has no equivalent.
3. **Reduce output verbosity.** If code-analyze-mcp's output were more compact, the token
   overhead would shrink, potentially making it competitive on efficiency.

## Artifacts

- `prompts/` -- condition A and B prompts, task description
- `results/runs/` -- raw JSON reports per run (A1-A5, B1-B5)
- `results/pilot/` -- pilot run reports (pilot-A, pilot-B)
- `results/blinded-scores.json` -- blinded scoring output
- `scores.json` -- unblinded scores with statistics
- `run-order.txt` -- randomized execution order
- `analysis.md` -- this document
