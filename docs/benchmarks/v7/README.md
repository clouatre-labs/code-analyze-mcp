# v7 Benchmark: Parameter Discovery and Token Efficiency

## Goal

Measure whether agents can discover and use new tool parameters (summary, cursor, page_size) to reduce token consumption when analyzing code. This extends v6 by instrumenting the tool definition with parameter documentation and tracking parameter usage.

## Hypotheses

1. **Parameter Discovery:** When parameter documentation is included in the tool description, agents will discover and apply these parameters to optimize their analysis.
2. **Token Efficiency:** Using summary, cursor, and page_size effectively will reduce token overhead in Condition B (treatment) vs Condition A (control).
3. **Quality Preservation:** Agents using parameters will not regress on code analysis quality.

## v6 Baselines

From v6-benchmark, we establish reference token and quality metrics:

| Condition | Median Quality | Median Tokens | Tool Calls |
|-----------|----------------|---------------|-----------|
| A (baseline) | 10 | ~8000 | 8-12 |
| B (v6 treatment) | 10 | ~9000 (+12%) | 8-12 |

**Note:** v6 saw no quality difference (both median 10) but 12-22% token overhead in B.

## File Manifest

### Prompts

- **task.md:** Verbatim v6 task description (lsd cross-module analysis).
- **condition-a-control.md:** v6 control condition adapted for v7 (no parameter changes).
- **condition-b-treatment.md:** v6 treatment + new section documenting summary, cursor, page_size parameters with examples.

### Scripts

- **collect.py:** Adapted from v6; added extract_parameter_usage() to extract summary_count, cursor_calls, page_size_overrides, pagination_used from tool call inputs.
- **validate.py:** Adapted from v6; added parameter_usage tracking section for Condition B to detect parameter usage in tool call inputs.
- **analyze.py:** Adapted from v6; added --version flag. Default (v7) behavior runs all v6 analysis plus parameter usage frequency table and v7B vs v6B token delta.

### Templates and Docs

- **scores-template.json:** Extended v6 template with v7 hypothesis, parameter_usage_tracking section, v6_baselines, and per_run_scores placeholders for parameter_usage.
- **run-order.txt:** Randomized order (seed=256): 10 runs, 5 per condition, order: B5, B4, A2, B1, A1, A4, A3, B2, A5, B3.
- **README.md:** This file.
- **methodology.md:** Detailed hypothesis, design, rubric, statistical test, and instrumentation.

### Directory Structure

```
docs/benchmarks/v7/
├── prompts/
│   ├── task.md
│   ├── condition-a-control.md
│   └── condition-b-treatment.md
├── scripts/
│   ├── collect.py
│   ├── validate.py
│   └── analyze.py
├── results/
│   └── runs/
│       ├── R01.json
│       ├── R02.json
│       ...
│       └── R10.json
├── scores-template.json
├── scores.json (filled during experiment)
├── run-order.txt
├── README.md (this file)
└── methodology.md
```

## Execution Checklist

- [ ] Directory structure created (docs/benchmarks/v7/ with all subdirs)
- [ ] All prompt files written and validated for completeness
- [ ] Scripts (collect.py, validate.py, analyze.py) pass --help with no errors
- [ ] run-order.txt verified to match seed=256 output
- [ ] scores-template.json extends v6 template with hypothesis and parameter_usage section
- [ ] All v6 changes prohibited (v6 scripts, docs, results untouched)
- [ ] Git branch created (feat/benchmark-v7) and ready for PR

## Pre-Run Pain Points and Mitigations

### Parameter Documentation Clarity

**Pain Point:** Agents may not discover or understand new parameters if documentation is vague.

**Mitigation:** condition-b-treatment.md includes concrete goose-compatible examples for each parameter:
- summary: "Shows file header, top 10 functions, import count" with example: `summary=true` for large files
- cursor: "Pass cursor from previous response to paginate" with example: `cursor: "<value>" from prior response`
- page_size: "Limit output size (default: 50000); reduce for shorter responses" with example: `page_size: 30000`

### Tool Isolation Enforcement

**Pain Point:** Sessions may not respect condition constraints (A/B parameter forbidding).

**Mitigation:** validate.py detects violations; run before collecting metrics.

### Parameter Extraction Accuracy

**Pain Point:** Tool call parameter extraction may miss non-standard input formats.

**Mitigation:** collect.py and validate.py handle both Anthropic-style and Goose-style message formats; fallback to 0 if parameters absent.

### Baseline Comparison Ambiguity

**Pain Point:** v7 token/quality differences may be confounded with v6 variance or model drift.

**Mitigation:** Preserve v6_baselines in scores-template.json; all v7 analysis explicitly compares v7B vs v6B and v7B vs v7A.

## Failure Recovery

### Session Not Found

If a session name (e.g., v7-benchmark-R01-B5) is not in the goose database:

1. Verify session was saved (goose session list).
2. Check session name format (must match run-order.txt).
3. If missing, re-run the benchmark run and save with correct name.

### Parameter Extraction Returns 0

If all B runs show parameter_usage all zeros:

1. Verify tool names in condition-b-treatment.md match goose tool registration.
2. Check that condition-b-treatment.md prompts were actually used in sessions (grep session DB).
3. Inspect R0X.json raw tool call inputs manually to debug parsing.

### Quality Regression in v7B

If v7B quality drops vs v6B:

1. Check run-order.txt randomization (seed must be 256).
2. Verify prompt text in condition-b-treatment.md did not accidentally remove task requirements.
3. Re-run validation (validate.py) to confirm tool isolation was maintained.

## Scoring and Analysis

1. Blind and shuffle run results (seed=256 mapping in scores-template.json).
2. Fill per_run_scores with quality dimensions (structural_accuracy, cross_module_tracing, approach_quality, tool_efficiency) on 0-3 scale.
3. Run collect.py for each session to extract metrics and parameter_usage.
4. Run analyze.py --version v7 to generate analysis with parameter usage frequency table.
5. Compare v7B vs v6B token delta and parameter usage frequency to test hypothesis.

## References

- [methodology.md](methodology.md) – Experiment design and statistical rationale
- [docs/benchmarks/v6/](../v6) – v6 baseline and results
- [docs/benchmarks/ISOLATION.md](../ISOLATION.md) – Tool isolation limitations and workaround strategy
