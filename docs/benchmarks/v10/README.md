# v10 Benchmark: Fix Validation and Open-Weight Model Evaluation

## Goal

1. **Validate C1/C2/C3 fixes:** Confirm that server-side pagination fixes reduce research_calls in Condition B (Haiku + MCP) to ≤5 while maintaining quality_score non-inferior to v9 B baseline.

2. **Evaluate open-weight models:** Assess whether open-weight models (Minimax, Mistral Small) on OpenRouter achieve competitive quality_score and cost-efficiency compared to proprietary models (Sonnet, Haiku).

## Conditions

| ID | Model | Provider | Tools | N | Runs | Notes |
|----|-------|----------|-------|---|------|-------|
| A | claude-sonnet-4-6 | gcp_vertex_ai | Native | 4 | A1, A2, A3, A4 | New runs |
| A2 | claude-haiku-4-5 | gcp_vertex_ai | Native | 4 | A2_1, A2_2, A2_3, A2_4 | New runs (baseline for H2, H3) |
| B | claude-haiku-4-5 | gcp_vertex_ai | MCP | 4 | B1, B2, B3, B4 | New runs (post-fix; server behavior changed) |
| C | claude-sonnet-4-6 | gcp_vertex_ai | MCP | 4 | C1, C2, C3, C4 | New runs |
| D | minimax/minimax-m2.5 | openrouter | MCP | 4 | D1, D2, D3, D4 | New runs (open-weight baseline) |
| E | mistralai/mistral-small-2603 | openrouter | MCP | 4 | E1, E2, E3, E4 | New runs (cost-efficient open-weight) |

**Total: 24 runs (4 per condition).**

## Hypotheses

- **H1 (Fix Efficacy):** v10 B median research_calls ≤ 5 (vs v9 B ~11); quality_score non-inferior to v9 B.
- **H2 (Model Size):** A (Sonnet+native) quality_score > A2 (Haiku+native).
- **H3 (Tool Type):** B (Haiku+MCP) quality_score > A2 (Haiku+native).
- **H4 (Open-Weight):** D and E achieve quality_score ≥ v9 B baseline on structural_accuracy and cross_module_tracing.
- **H5 (Cost Efficiency):** E (Mistral Small+MCP) effective_cost_per_qp < C (Sonnet+MCP).
- **H0 (Null):** No significant differences (15 pairwise Mann-Whitney U, Bonferroni α=0.0033).

## Files

| File | Purpose |
|------|---------|
| `methodology.md` | Full experimental design, rubric, statistical method, provider cost model |
| `run-order.txt` | 24 randomized runs (seed=42) with blinding map |
| `scores-template.json` | Template for scoring all 24 runs; fill in per_run_scores after blind evaluation |
| `README.md` | This file; quick-start guide and execution checklist |

## Prerequisites

- **goose CLI:** `goose --version` (for session management)
- **OPENROUTER_API_KEY:** Set in environment for Conditions D and E
- **Django clone:** Target repository pinned at commit `6b90f8a8d6994dc62cd91dde911fe56ec3389494`
- **Claude Code:** Sessions stored as JSONL under `~/.claude/projects/<project-slug>/`

## Execution

### Step 1: Execute All Runs

All 24 runs are fresh. Execute in the order specified in `run-order.txt`:

1. Open Claude Code
2. Clone target repository at pinned commit
3. For each run in order:
   - Load the appropriate condition (model, provider, tool set)
   - Execute the benchmark task (deep cross-module code analysis)
   - Record session ID and output JSON
   - Collect metrics: tool_calls_total, research_calls, cost_usd, total_tokens, etc.

**Environment variables:**
- `DISABLE_PROMPT_CACHING=1` (all conditions)
- `OPENROUTER_API_KEY=<key>` (Conditions D and E only)

### Step 3: Blind Scoring

1. Randomize output order using blinding map (R01–R24 in run-order.txt)
2. Score each output independently (15–30 minutes per output)
3. Rate each dimension 0–3 per rubric; compute quality_score = sum
4. Record notes and reasoning_mode ("disabled" for all v10 runs)
5. Consistency check: review first 3 outputs after scoring all 24 for calibration drift

### Step 4: Fill scores-template.json

After blind scoring, fill per_run_scores with:
- `structural_accuracy`, `cross_module_tracing`, `approach_quality`, `tool_efficiency` (0–3 each)
- `quality_score` (sum of 4 dimensions)
- `notes` (free-form scoring rationale)
- `reasoning_mode` ("disabled")

Example entry:
```json
"A1": {
  "structural_accuracy": 3,
  "cross_module_tracing": 3,
  "approach_quality": 3,
  "tool_efficiency": 2,
  "quality_score": 11,
  "notes": "All 12 major modules present...",
  "reasoning_mode": "disabled"
}
```

### Step 5: Run Analysis

```bash
python scripts/analyze.py --scores-file scores-template.json
```

Outputs:
- 15 pairwise Mann-Whitney U tests (Bonferroni α=0.0033)
- Effect sizes (rank-biserial r)
- Summary statistics per condition
- Cost analysis (effective_cost_per_qp)

## Scoring

### Blind Evaluation

- Scorer does not see condition labels during evaluation
- Blinding map (R01–R24) reveals condition assignment post-scoring
- Ensures unbiased assessment across all 6 conditions

### Rubric

See `methodology.md` for full rubric with anchor descriptions.

**Dimensions (0–3 each):**
1. **Structural Accuracy:** Module identification, type accuracy, boundary clarity
2. **Cross-Module Tracing:** End-to-end pipeline trace, intermediate types, data flow
3. **Approach Quality:** Proposal correctness, pattern adherence, risk analysis
4. **Tool Efficiency:** Research tool call count (≤5 = 3 points, 6–10 = 2, 11–20 = 1, >20 = 0)

**Quality Score:** Sum of 4 dimensions (0–12).

## References

- [methodology.md](methodology.md) — full experimental design and statistical method
- [v9 Benchmark](../v9/) — preceding benchmark (3-condition, N=5 per condition)
- [gap-analysis-v9b.md](../gap-analysis-v9b.md) — analysis of v9 B pagination bugs
- [PR #320](https://github.com/anthropics/code-analyze-mcp/pull/320) — server-side pagination fixes
