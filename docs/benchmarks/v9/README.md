# v9 Benchmark: Three-Way Comparison of Sonnet Native, Haiku+MCP, and Sonnet+MCP

## Goal

Measure whether code-analyze-mcp semantic tools improve agent performance on deep cross-module code analysis tasks when:

1. **Condition A (Control):** Sonnet with native tools only
2. **Condition B (Treatment):** Haiku with MCP tools only
3. **Condition C (Treatment):** Sonnet with MCP tools

Isolate the MCP efficiency signal from platform-specific overhead (prompt caching disabled) and compare both model and tool combinations.

## Hypotheses

| Hypothesis | Statement |
|---|---|
| **H1 (MCP Quality)** | Condition B and C produce higher total rubric scores than Condition A. |
| **H2 (Model Quality)** | Sonnet conditions (A, C) produce higher total rubric scores than Haiku condition (B). |
| **H3 (Cost-effectiveness)** | Condition C achieves lower effective cost per quality point than A, despite model cost premium. |
| **H0 (Null)** | No significant differences between conditions (Mann-Whitney U, Bonferroni α=0.017 for 3 pairwise tests). |

## Conditions

| | Condition A — Control | Condition B — Treatment | Condition C — Treatment |
|---|---|---|---|
| **Model** | Sonnet | Haiku | Sonnet |
| **Native tools** | Glob, Grep, Read, Bash | Not available | Glob, Grep, Read, Bash |
| **MCP tools** | Not available | analyze_directory / analyze_file / analyze_symbol | analyze_directory / analyze_file / analyze_symbol |
| **Caching** | Disabled (`DISABLE_PROMPT_CACHING=1`) | Disabled | Disabled |

## File Manifest

```
docs/benchmarks/v9/
├── README.md (this file)
├── methodology.md
├── run-order.txt
├── scores-template.json
├── scores.json              (filled during experiment)
├── prompts/
│   ├── task.md
│   ├── condition-a-control.md
│   ├── condition-b-treatment-haiku.md
│   └── condition-c-treatment-sonnet.md
├── scripts/
│   ├── collect.py           (extended: research_calls, cache tokens)
│   ├── validate.py          (extended: 3 conditions, budget validation)
│   └── analyze.py           (extended: 3-way Mann-Whitney U, Bonferroni)
└── results/
    └── runs/
        ├── R01.json … R15.json
```

## Execution Checklist

- [ ] Target repository chosen and committed; SHA recorded in `run-order.txt`
- [ ] `DISABLE_PROMPT_CACHING=1` set in runner shell (not in `~/.zshrc` or shell profile)
- [ ] Run `python3 scripts/validate.py --help` — verify script exits 0
- [ ] Run `python3 scripts/collect.py --help` — verify script exits 0
- [ ] Run `python3 scripts/analyze.py --help` — verify script exits 0
- [ ] Pilot: 1 run per condition (R01, R02, R03) using runner template below; review output format
- [ ] Run `python3 scripts/validate.py --session-file SESSION.jsonl --condition A` on pilot runs; verify PASS
- [ ] Full runs: Execute all 15 runs in order from `run-order.txt`
- [ ] For each run: record session JSONL file; extract metrics with `collect.py`; validate with `validate.py`
- [ ] Fill `scores.json` after blind scoring (scorer reads outputs, does not see condition labels)
- [ ] Run `python3 scripts/analyze.py --scores-file scores.json > results/analysis.md`

## Runner Template

For each run, execute in a shell with environment isolation:

```bash
# Example: Run R01 (Condition A4 per blinding map)
DISABLE_PROMPT_CACHING=1 claude \
  --system-prompt prompts/condition-a-control.md \
  < prompts/task.md \
  > results/runs/R01.json \
  2> results/runs/R01.log
```

Capture the Claude Code session JSONL at:
```
~/.claude/projects/<project-slug>/<session-id>.jsonl
```

Then extract metrics:
```bash
python3 scripts/collect.py \
  --session-file ~/.claude/projects/code-analyze-mcp/SESSION_ID.jsonl \
  --output-file results/runs/R01.json \
  > results/runs/R01-metrics.json
```

Validate isolation:
```bash
python3 scripts/validate.py \
  --session-file ~/.claude/projects/code-analyze-mcp/SESSION_ID.jsonl \
  --condition A
```

## Results

**Status:** Pending — benchmark not yet executed.

Target repository: **TBD** (to be confirmed before pilot runs).

Pilot runs: **0/3 complete**.

Full runs: **0/15 complete**.

See [methodology.md](methodology.md) for statistical design and rubric definitions.
