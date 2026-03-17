# v10 Benchmark Methodology

## Research Questions

**H1 (Fix Efficacy):** v10 Condition B (Haiku + MCP, post-fix) achieves median research_calls ≤ 5 (vs v9 B ~11) while maintaining quality_score non-inferior to v9 B baseline.

**H2 (Model Size):** Condition A (Sonnet + native) produces higher quality_score than Condition A2 (Haiku + native).

**H3 (Tool Type):** Condition B (Haiku + MCP) produces higher quality_score than Condition A2 (Haiku + native).

**H4 (Open-Weight Models):** Conditions D (Minimax) and E (Mistral Small) achieve quality_score ≥ v9 B baseline on structural_accuracy and cross_module_tracing dimensions.

**H5 (Cost Efficiency):** Condition E (Mistral Small + MCP) achieves effective_cost_per_qp < Condition C (Sonnet + MCP).

**H0 (Null):** No significant differences across 15 pairwise condition comparisons (Mann-Whitney U, Bonferroni α=0.0033).

## Experimental Design

### Conditions

| Condition | Model | Provider | Tool Set | N | Notes |
|-----------|-------|----------|----------|---|-------|
| A | claude-sonnet-4-6 | gcp_vertex_ai | Native (Glob, Grep, Read, Bash) | 4 | New runs: A1, A2, A3, A4 |
| A2 | claude-haiku-4-5 | gcp_vertex_ai | Native (Glob, Grep, Read, Bash) | 4 | New runs: A2_1, A2_2, A2_3, A2_4 |
| B | claude-haiku-4-5 | gcp_vertex_ai | MCP (analyze_directory, analyze_file, analyze_symbol) | 4 | New runs: B1, B2, B3, B4 (server-side tool behavior changed post C1/C2/C3 fixes) |
| C | claude-sonnet-4-6 | gcp_vertex_ai | MCP (analyze_directory, analyze_file, analyze_symbol) | 4 | New runs: C1, C2, C3, C4 |
| D | minimax/minimax-m2.5 | openrouter | MCP (analyze_directory, analyze_file, analyze_symbol) | 4 | New runs: D1, D2, D3, D4 |
| E | mistralai/mistral-small-2603 | openrouter | MCP (analyze_directory, analyze_file, analyze_symbol) | 4 | New runs: E1, E2, E3, E4 |

**Total N = 24 runs (4 per condition).**

### Participants

- N = 24 runs total: 4 per condition (A: 4, A2: 4, B: 4, C: 4, D: 4, E: 4)
- Run order: randomized with seed = 42
- Blinding: scorer blind to condition assignment during scoring; label map revealed post-scoring
- Reasoning mode: disabled for all conditions in this benchmark

### Target Repository

**To be determined.** Repository selection criteria:

- Python project with 50,000+ lines of code
- Deep module hierarchy: 30+ files across 5+ top-level packages
- Complex dependency graph with semantic relationships not obvious from file names
- Representative of real-world code analysis tasks (e.g., multi-dialect SQL parser, ORM, framework)

Commit SHA will be pinned at benchmark execution and recorded in `run-order.txt`.

### Run Order (seed = 42)

See `run-order.txt` for the complete randomized order with blinding map.

## Validity Controls

### No v9 Reuse

All 24 runs are executed fresh against the current server version. v9 A and C runs are not reused because multiple `src/` changes landed between v9 execution (commit `ff4e299`, 2026-03-14) and v10:

- `#295` -- pruned `call_frequency`, `field_accesses`, `assignments` from `analyze_file` output; agents using MCP see different data
- `#316` -- path prefix matching fix; directory listings may differ
- `#312` -- `analyze_module` FILE header now includes function/import counts
- `#320` -- C1/C2/C3 pagination fixes (the primary change under test)
- `#327` -- cursor error on `summary=true` in `analyze_directory`
- `#330` -- Python wildcard import parsing fix

Reusing v9 A/C runs would compare native-tool outputs produced against a different server version against MCP outputs produced against the fixed server, invalidating cross-condition comparisons.

### Exclusion of v9 B

v9 B runs are **NOT reused and NOT rerun** in v10. Rationale:

- v9 B runs (N=5) were confounded by pagination bugs in C1/C2/C3 (server-side tool behavior).
- Post-fix server behavior differs; v9 B results are not comparable to v10 B.
- v10 B is a fresh 4-run condition with corrected server-side tool behavior.
- This ensures H1 (fix efficacy) can be tested: v10 B median research_calls vs v9 B ~11.

### C1/C2/C3 Fix Context

Conditions C1, C2, C3 (and by extension B) were affected by pagination bugs in the MCP server's analyze_directory and analyze_file tools. These bugs caused:

- Incomplete result sets in large directories
- Missed intermediate types in cross-module traces
- Inflated research_calls due to retry loops

Post-fix (v10), the server correctly handles pagination. All v10 B, C, D, and E runs are executed fresh against the fixed server.

## Rubric (0–3 per dimension, max 12)

All dimensions calibrated to target repository. Anchor descriptions below are generic templates; refine after pilot runs.

### Structural Accuracy (0–3)

| Score | Criteria |
|---|---|
| 3 | Correctly identifies all major top-level modules; responsibilities clearly defined; key types/classes accurately described; module boundaries respect design intent |
| 2 | Identifies most modules; minor omissions; core functional areas present |
| 1 | Partial coverage; missing 1-2 major modules or unclear responsibilities; vague on key abstractions |
| 0 | Major components missing; fundamental misunderstanding of project structure |

### Cross-Module Tracing (0–3)

| Score | Criteria |
|---|---|
| 3 | Complete end-to-end trace through the primary pipeline; intermediate types/classes identified at each stage; data flow clear; integration points noted |
| 2 | Key stages present; minor gaps (one intermediate stage unclear, or one type missing); general flow understandable |
| 1 | Partial trace; missing multiple stages or intermediate types; flow not end-to-end |
| 0 | No meaningful trace or entirely incorrect |

### Approach Quality (0–3)

| Score | Criteria |
|---|---|
| 3 | Change proposal identifies: correct files and classes; follows existing patterns; integration point appropriate; realistic risks identified; minimal disruption to existing code |
| 2 | Reasonable proposal; most files and types identified; risks partially addressed |
| 1 | Incomplete; missing files, unclear integration point, or no risk analysis |
| 0 | Superficial or incorrect proposal |

### Tool Efficiency (0–3)

| Score | Criteria |
|---|---|
| 3 | 5 or fewer research tool calls; focused exploration; clear synthesis path |
| 2 | 6-10 research tool calls; somewhat exploratory but reaches conclusions |
| 1 | 11-20 research tool calls; extensive exploration; delayed synthesis |
| 0 | More than 20 research tool calls; inefficient; redundant or circular exploration |

## Statistical Analysis

### Primary Tests

**15 pairwise Mann-Whitney U tests on quality_score:**

All pairwise comparisons across 6 conditions (A, A2, B, C, D, E):
- A vs A2, A vs B, A vs C, A vs D, A vs E
- A2 vs B, A2 vs C, A2 vs D, A2 vs E
- B vs C, B vs D, B vs E
- C vs D, C vs E
- D vs E

**Bonferroni correction:** α = 0.05 / 15 = 0.0033 per test.

**Effect size:** rank-biserial r; |r| ≥ 0.3 small, ≥ 0.5 medium, ≥ 0.7 large.

### Secondary Analyses

- **Tool call efficiency:** Median research_calls per condition; MCP vs native split in B and A2.
- **Cost analysis:** Median cost_usd per condition; effective_cost_per_qp = cost_usd / (quality_score * reliability).
- **Protocol violations:** Count and type of tool isolation failures per condition.
- **Token efficiency:** Median total_tokens per condition.

### Reporting

All analyses reported as **exploratory** due to small N=4 per condition. Statistical significance thresholds provided for reference only; conclusions weighted toward effect sizes and practical significance.

## Blinding

Randomized run order with seed=42 ensures neutral evaluation sequence. Blinding map in `scores-template.json` tracks real condition (A/A2/B/C/D/E) but scorers do not see it during evaluation. Mapping revealed only after all scores submitted.

## Provider Cost Model

| Model | Provider | Input $/M | Output $/M | Notes |
|-------|----------|-----------|-----------|-------|
| claude-sonnet-4-6 | gcp_vertex_ai | $3.00 | $15.00 | Standard Sonnet pricing |
| claude-haiku-4-5 | gcp_vertex_ai | $0.25 | $1.25 | Corrected Haiku pricing (v9 used $0.80/$4.00) |
| minimax/minimax-m2.5 | openrouter | $0.30 | $1.10 | Open-weight model pricing |
| mistralai/mistral-small-2603 | openrouter | $0.15 | $0.60 | Open-weight model pricing |

## Session Format and Tooling

v10 runs use Claude Code. Sessions stored as JSONL files under `~/.claude/projects/<project-slug>/<session-id>.jsonl`.

**collect.py** extends v9 to extract:
- `research_calls` = tool_calls_total - Write - Edit - housekeeping Bash
- `reasoning_mode` (always "disabled" for v10)

**validate.py** extends v9 to:
- Check Condition A: no MCP tools allowed
- Check Condition A2: no MCP tools allowed
- Check Condition B: MCP tools required; no native file exploration tools
- Check Condition C: MCP tools required; native tools not allowed
- Check Condition D: MCP tools required; no native file exploration tools
- Check Condition E: MCP tools required; no native file exploration tools
- Check all conditions: research_calls ≤ 10 (post-hoc validation, not enforcement)

**analyze.py** extends v9 to:
- Compute 15 pairwise Mann-Whitney U tests with Bonferroni α=0.0033
- Report effect sizes (rank-biserial r)
- Summarize protocol violations per condition
- Report token metrics

## References

- [v9 Methodology](../v9/methodology.md) — preceding benchmark (3-condition, prompt caching disabled)
- [gap-analysis-v9b.md](../gap-analysis-v9b.md) — analysis of v9 B pagination bugs and fix rationale
- [PR #320](https://github.com/anthropics/code-analyze-mcp/pull/320) — server-side pagination fixes
- [docs/benchmarks/ISOLATION.md](../ISOLATION.md) — tool isolation protocol
