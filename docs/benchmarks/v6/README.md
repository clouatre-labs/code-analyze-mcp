# v6 Benchmark

## Goal

Measure the impact of 5 lossless compaction changes (#129-#134) on token overhead. v5 found that code-analyze-mcp produces equivalent quality output but with 22% higher token usage. v6 tests whether formatting improvements close this efficiency gap without quality regression.

## What Changed Since v5

Five lossless formatting improvements were implemented to reduce redundant whitespace and boilerplate:

- #129: Relative paths in all output modes (PR #135)
- #130: Tree-indent callees and callers in focus mode (PR #137)
- #132: Separate test callers into counted summary (PR #139)
- #133: Add summary counts to FOCUS header (PR #140)
- #134: Deduplicate repeated callee chains with (xN) annotation (PR #141)

All changes preserve semantic information. The primary target was focus mode, where v5 showed
528 flat callee lines vs native analyze's 306 tree-indented lines for identical content.

## File Manifest

```
docs/benchmarks/v6/
  prompts/
    task.md                      # Task prompt (copied verbatim from v5)
    condition-a-control.md       # Control prompt
    condition-b-treatment.md     # Treatment prompt
  scripts/
    collect.py                   # Extract session metrics
    validate.py                  # Validate run data
    analyze.py                   # Statistical analysis with --base-version flag
  results/
    runs/                        # Run output directories (created during execution)
  run-order.txt                  # Randomized 10-run order (seed=128)
  scores-template.json           # Template with v5 baselines, null placeholders
  methodology.md                 # Experiment design and rationale
  output-comparison.md           # Template for post-run measurements
  README.md                      # This file
```

## Execution Checklist

### Pre-Run
- [ ] lsd-rs/lsd cloned locally at TARGET_REPO_PATH
- [ ] Goose configured with `--no-profile` for tool isolation
- [ ] Condition A config: `--with-builtin developer,analyze`
- [ ] Condition B config: `--with-builtin developer --with-extension code-analyze-mcp`

### Run Execution
- [ ] 10 runs executed in run-order.txt sequence
- [ ] Each run is a single goose session with one condition
- [ ] Raw session output saved to results/runs/R{01-10}.json
- [ ] Session metrics extracted via `python3 scripts/collect.py`

### Post-Run Scoring
- [ ] Score each R01-R10 blind (condition labels stripped)
- [ ] Enter scores into scores-template.json per_run_scores
- [ ] Reveal blinding mapping, separate into A/B
- [ ] Validate tool isolation via `python3 scripts/validate.py`

### Post-Run Analysis
- [ ] `python3 scripts/analyze.py --scores-file scores.json --base-version v5`
- [ ] Fill output-comparison.md with per-mode measurements from session logs
- [ ] Write analysis.md with findings and decision on #136

## Methodology

See [methodology.md](methodology.md) for full design:

- **Design:** 10 runs, 5 per condition, Mann-Whitney U test, rank-biserial r effect size
- **Hypothesis:** v6B token overhead reduces to <10% above v6A (from 22% in v5)
- **Rubric:** 4 dimensions (structural accuracy, cross-module tracing, approach quality, tool efficiency), 0-3 points each
- **Blinding:** Seed=128 randomization, condition labels stripped before scoring

## v5 Baseline

v5 found:
- Quality: Condition A median = 10, Condition B median = 10 (no regression)
- Tokens: Condition B 22% higher than Condition A

v6 tests whether compaction changes narrow this gap.

## References

- [methodology.md](methodology.md) – Experiment design and statistical rationale
- [output-comparison.md](output-comparison.md) – Template for post-run output measurements
- [docs/benchmarks/v5/](../v5) – v5 baseline and unchanged methodology details
- [docs/benchmarks/ISOLATION.md](../ISOLATION.md) – Tool isolation limitations and workaround strategy
