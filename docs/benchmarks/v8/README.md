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

## Results

**Status:** Complete. Pinned commit: `d5e6d965288c0929e0a4ef9a9db292fb28bbf3d1`. All 10 runs valid.

### Quality (rubric scores 0–12)

| Dimension | Med A | Med B | r |
|-----------|-------|-------|---|
| structural_accuracy | 2.0 | 2.0 | -0.20 |
| cross_module_tracing | 3.0 | 3.0 | -0.20 |
| approach_quality | 3.0 | 3.0 | 0.00 |
| tool_efficiency | 1.0 | 0.0 | -0.20 |
| **total** | **9.0** | **8.0** | **-0.60** |

Mann-Whitney U=20.0, z=1.57, r=-0.60 (medium-large effect favouring A). N=5 per condition; p not reportable.

### Cost (Bedrock on-demand, prompt caching included)

| Condition | Median cost | Median quality | Eff. cost/QP |
|-----------|-------------|----------------|--------------|
| A — native only | $0.194 | 9.0 | $0.0216 |
| B — native + MCP | $0.283 | 8.0 | $0.0353 |

B costs 63% more per quality point. Median total tokens: A=1.41M, B=1.63M (+16%). The cost premium comes from MCP responses writing more to the prompt cache (median cache_write: A=49K tokens, B=98K tokens), inflating cache_read costs on every subsequent turn.

**Caching Confound:** v8 did not disable prompt caching (both conditions ran with Bedrock default-on caching). In single-session benchmarks, the cache is never reused; every turn pays the cache_write tax with zero cache_read benefit. This inflates the cost premium for Condition B. See v9 benchmark (#268) which repeats v8 with `DISABLE_PROMPT_CACHING=1` to isolate the MCP efficiency signal from the platform-specific caching overhead.

### Hypotheses

| | Result |
|---|---|
| H1 (quality) | Not supported. Quality identical on 3 of 4 dimensions; difference is in tool_efficiency only. |
| H2 (efficiency) | Not supported. B used 16% more tokens. MCP calls replaced some native calls but agents issued redundant Bash/Grep calls alongside them. |
| H3 (cost-effectiveness) | Not supported. B effective cost/QP is 63% higher than A on Bedrock with prompt caching. |
| H0 (null) | Holds. No significant difference at N=5; directional evidence favours A on cost. |

### What this does and does not mean

**For Claude Code on Bedrock:** Adding MCP semantic tools does not improve quality and increases cost. Agents issued native fallback calls alongside MCP calls, negating the per-call efficiency advantage.

**It does not mean MCP has no value elsewhere:**

- **Non-Claude clients** (Goose, Cursor, others without built-in semantic analysis). v5/v6 Goose benchmarks showed MCP eliminated shell fallback calls when the baseline already relied on a weaker native analyze tool.
- **Providers without prompt caching.** The cost disadvantage is driven by large MCP responses inflating cache_write costs per turn. Without caching, this compounding effect does not apply.
- **Enforced turn budgets.** Every agent in v8 overran the 10-call research limit (actual range: 14-37 calls). MCP's higher information density per call is untested under strict budget conditions where that density would matter.
- **Structurally opaque codebases.** sqlglot has a clear directory structure that grep navigates well. MCP's call-graph tracing is most valuable where grep cannot surface semantic relationships (deep monorepos, cross-package type graphs).

### Cross-version context

The efficiency gains attributed to MCP in v5/v6 (Goose) referred specifically to **shell call elimination**: B condition agents issued 0 shell fallback calls vs 2-3 per run for Goose's native tool condition. Token cost was reportedly +22% for MCP even in v5, which led to the v6 compaction work.

v8 is the first benchmark with actual Bedrock cost data (all prior `cost_usd` and `total_tokens` fields in v3-v7 are null). It also tests a stricter baseline — no semantic tools at all, vs v5/v6 where condition A had Goose's own analyze tool.

### Anomalies

- **B1 (R04):** 39 total calls (5 MCP + 30 Bash). Agent issued MCP calls then continued with extensive Bash grep/head/ls, likely after mishandling a paginated MCP response. Genuine execution outlier; excluding it, B median calls = 21 vs A median = 20.
- **A2 (R01):** 40 calls (37 native). Excluding both outliers, conditions are indistinguishable on call count.
- **Budget overrun:** 10-call research budget not enforced by the runner; actual range 14-37 calls across all runs.

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
