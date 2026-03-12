# v8 Benchmark: Native Claude Code Tools vs Native + code-analyze-mcp Semantic Tools (sqlglot)

## Goal

Measure whether adding code-analyze-mcp semantic tools to Claude Code improves agent performance on deep cross-module code analysis tasks, compared to native file-system tools alone.

## Conditions

| | Condition A — Control | Condition B — Treatment |
|---|---|---|
| **Glob, Grep, Read, Bash** | Available | Available |
| **analyze_directory / analyze_file / analyze_symbol** | Not available | Available (preferred) |

## Hypotheses

- **H1 (Quality):** Condition B produces higher total rubric scores than Condition A.
- **H2 (Efficiency):** Condition B achieves equal or better quality with fewer total tokens.
- **H3 (Cost-effectiveness):** Condition B achieves lower `cost_usd / (quality_score * reliability)`.
- **H0 (Null):** No significant difference (Mann-Whitney U, α = 0.05).

## File Manifest

```
docs/benchmarks/v8/
├── README.md
├── methodology.md
├── run-order.txt
├── scores-template.json
├── scores.json              (filled during experiment)
├── prompts/
│   ├── task.md
│   ├── condition-a-control.md
│   └── condition-b-treatment.md
├── scripts/
│   ├── collect.py           (Claude Code JSONL format)
│   ├── validate.py
│   └── analyze.py
└── results/
    └── runs/
        ├── R01.json … R10.json
```

## Execution Checklist

- [ ] Pin sqlglot commit SHA; record in `run-order.txt`
- [ ] Run 10 sessions in order from `run-order.txt` (seed=512)
- [ ] For each run: validate isolation with `validate.py`; extract metrics with `collect.py`
- [ ] Fill `scores.json` after blind scoring
- [ ] Run `analyze.py --scores-file scores.json` to generate tables

## Run Order (seed = 512)

1. A2, 2. A5, 3. B5, 4. B1, 5. B2, 6. A4, 7. B4, 8. A3, 9. B3, 10. A1

Blinding mapping (revealed post-scoring):
R01=A2, R02=A5, R03=B5, R04=B1, R05=B2, R06=A4, R07=B4, R08=A3, R09=B3, R10=A1

## Key Differences from v7

| | v7 | v8 |
|---|---|---|
| **Comparison** | MCP with vs without parameter docs | Native-only vs Native + MCP |
| **Target repo** | lsd-rs/lsd (Rust) | tobymao/sqlglot (Python) |
| **Session format** | Goose (`sessions.db`) | Claude Code (JSONL) |
| **Control condition** | MCP without parameters | No MCP at all |

## References

- [methodology.md](methodology.md) — full hypothesis, design, rubric, statistical plan
- [docs/benchmarks/v7/](../v7) — preceding benchmark
- [docs/benchmarks/ISOLATION.md](../ISOLATION.md) — tool isolation protocol
