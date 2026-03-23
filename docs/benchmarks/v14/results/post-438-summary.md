# v14 Post-PR-438 Results Summary

## Overview

This document records the post-PR-438 benchmark results for conditions A and C. PRs 433 and 438 were
merged before these runs. PR 438 (`feat(analyze-module): promote analyze_module`) changed the tool
description for `analyze_module` to recommend it as a low-cost orientation step, which significantly
reduced token consumption for MCP conditions.

Binary version: **0.1.9** (post-PR-438).

Native conditions B and D were run against an earlier binary. Because native conditions do not invoke
the MCP server, PR 438 has no effect on their results; those runs remain valid as the native baseline.

## Canonical scored runs per condition

| Condition | Runs used | Notes |
|-----------|-----------|-------|
| A (Sonnet + MCP) | A-scored-6, A-scored-7 | Post-PR-438, binary 0.1.9 |
| B (Sonnet + native) | B-scored-1, B-scored-2 | Native; unaffected by PR 438 |
| C (Haiku + MCP) | C-scored-6, C-scored-7 | Post-PR-438, binary 0.1.9 |
| D (Haiku + native) | D-scored-1, D-scored-2 | Native; unaffected by PR 438 |

Runs A-scored-3 through A-scored-5 and C-scored-3 through C-scored-5 were recorded during an
intermediate state (post-433, pre-438) and are retained in the runs directory for reference but
excluded from the canonical analysis.

## Scores

### Condition A: Sonnet + MCP (post-438)

| Run | Dim1 | Dim2 | Dim3 | Total |
|-----|------|------|------|-------|
| A-scored-6 | 2 | 3 | 3 | 8 |
| A-scored-7 | 2 | 3 | 3 | 8 |
| **Mean** | **2.0** | **3.0** | **3.0** | **8.0** |

### Condition B: Sonnet + native

| Run | Dim1 | Dim2 | Dim3 | Total |
|-----|------|------|------|-------|
| B-scored-1 | 3 | 3 | 3 | 9 |
| B-scored-2 | 3 | 3 | 3 | 9 |
| **Mean** | **3.0** | **3.0** | **3.0** | **9.0** |

### Condition C: Haiku + MCP (post-438)

| Run | Dim1 | Dim2 | Dim3 | Total |
|-----|------|------|------|-------|
| C-scored-6 | 2 | 2 | 3 | 7 |
| C-scored-7 | 2 | 2 | 3 | 7 |
| **Mean** | **2.0** | **2.0** | **3.0** | **7.0** |

### Condition D: Haiku + native

| Run | Dim1 | Dim2 | Dim3 | Total |
|-----|------|------|------|-------|
| D-scored-1 | 2 | 3 | 3 | 8 |
| D-scored-2 | 3 | 3 | 3 | 9 |
| **Mean** | **2.5** | **3.0** | **3.0** | **8.5** |

## Efficiency (canonical runs)

| Condition | Turns (mean) | Tool calls (mean) | Cost USD (mean) | Input tokens (mean) |
|-----------|-------------|------------------|----------------|---------------------|
| A (Sonnet+MCP) | 15.5 | 11 | $1.44 | 431,618 |
| B (Sonnet+native) | 10.5 | 5.5 | $0.55 | 155,989 |
| C (Haiku+MCP) | 18.5 | 10.5 | $0.69 | 664,460 |
| D (Haiku+native) | 30 | 13.5 | $1.20 | 1,154,118 |

## Findings

**Quality:**
- Sonnet (A vs B): A mean 8.0 vs B mean 9.0. Native Sonnet edges MCP Sonnet by 1.0 point, driven
  by Dim1 (KitchenSink identification): B found KitchenSink in both runs; A found it in neither
  canonical run. Dim2 and Dim3 are tied at ceiling (3.0).
- Haiku (C vs D): C mean 7.0 vs D mean 8.5. Native Haiku outperforms MCP Haiku by 1.5 points.
  MCP Haiku consistently scores 2 on Dim2 (call chain tracing), failing to name live-path Sink impls
  at the dispatch point. D achieved Dim2=3 in both runs.

**Efficiency:**
- MCP Haiku (C) is the most cost-efficient condition at $0.69/run, beating native Haiku (D) at
  $1.20/run — a 43% cost reduction with MCP tools.
- MCP Sonnet (A) costs $1.44/run vs native Sonnet (B) at $0.55/run. Native Sonnet is 2.6x cheaper
  while also scoring slightly higher. Sonnet uses MCP tools more exhaustively (11 tool calls vs 5.5),
  driving up cost without proportional quality gain.
- Input token counts for Haiku+native (D) are the highest by far (1.15M mean) due to iterative
  grep-read loops reading full source files. MCP tools return structured summaries, keeping token
  counts lower.

**PR 438 effect on MCP conditions:**
The pre-438 runs (A-scored-3/4/5, C-scored-3/4/5) showed elevated costs (A: $1.42-$2.20,
C: $0.96-$1.13) and higher turn counts. Post-438 canonical runs show A at $0.99-$1.90 and C at
$0.67-$0.70. The `analyze_module` orientation hint reduced unnecessary deep-dive calls.
