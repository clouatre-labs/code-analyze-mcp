# v9 Benchmark Methodology

## Research Question

Does code-analyze-mcp semantic tools improve agent performance on deep cross-module code analysis when compared across model sizes and with prompt caching disabled? Does Sonnet model capability offset the efficiency advantage of MCP-assisted Haiku?

## Hypotheses

**H1 (MCP Quality):** Conditions B and C produce higher total rubric scores than Condition A.

**H2 (Model Quality):** Sonnet conditions (A, C) produce higher total rubric scores than Haiku condition (B).

**H3 (Cost-effectiveness):** Condition C achieves lower effective cost per quality point than Condition A, despite model cost premium.

**H0 (Null):** No significant differences in quality score distributions between conditions (Mann-Whitney U, Bonferroni α=0.017 for 3 pairwise contrasts).

## Experimental Design

### Conditions

| | Condition A — Control | Condition B — Treatment | Condition C — Treatment |
|---|---|---|---|
| **Name** | Sonnet + native tools | Haiku + MCP tools | Sonnet + MCP tools |
| **Model** | claude-sonnet-4-6 | claude-haiku-4-5 | claude-sonnet-4-6 |
| **Native tools** | Glob, Grep, Read, Bash | Not available | Glob, Grep, Read, Bash |
| **MCP tools** | Not available | Available (preferred) | Available (preferred) |
| **Prompt caching** | Disabled | Disabled | Disabled |

### Participants

- N = 15 runs total: 5 per condition (A: 5, B: 5, C: 5)
- Run order: randomized with seed = 512
- Blinding: scorer blind to condition assignment during scoring; label map revealed post-scoring
- Caching: `DISABLE_PROMPT_CACHING=1` set in runner environment (not shell profile)

### Target Repository

**To be determined.** Repository selection criteria:

- Python project with 50,000+ lines of code
- Deep module hierarchy: 30+ files across 5+ top-level packages
- Complex dependency graph with semantic relationships not obvious from file names
- Representative of real-world code analysis tasks (e.g., multi-dialect SQL parser, ORM, framework)

Commit SHA will be pinned at benchmark execution and recorded in `run-order.txt`.

### Run Order (seed = 512)

```
1. A4    2. C2    3. C5    4. A5    5. B4
6. A2    7. C3    8. B5    9. C1   10. B2
11. B1  12. A3   13. B3   14. C4   15. A1
```

Mapping (revealed post-scoring):
| Run ID | Condition | Rep |
|--------|-----------|-----|
| R01 | A | 4 |
| R02 | C | 2 |
| R03 | C | 5 |
| ... (etc) |
| R15 | A | 1 |

## Rubric (0–3 per dimension, max 12)

All dimensions to be calibrated to target repository. Anchor descriptions below are generic templates; refine after pilot runs.

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
| 3 | ≤ 5 research tool calls; focused exploration; clear synthesis path |
| 2 | 6–10 research tool calls; somewhat exploratory but reaches conclusions |
| 1 | 11–20 research tool calls; extensive exploration; delayed synthesis |
| 0 | > 20 research tool calls; inefficient; redundant or circular exploration |

## Metrics

Per-run metrics recorded in `RXX.json`:

| Metric | Description |
|---|---|
| `structural_accuracy` | Rubric dimension 1 (0–3) |
| `cross_module_tracing` | Rubric dimension 2 (0–3) |
| `approach_quality` | Rubric dimension 3 (0–3) |
| `tool_efficiency` | Rubric dimension 4 (0–3) |
| `quality_score` | Sum of 4 dimensions (0–12) |
| `input_tokens` | Session input token count |
| `output_tokens` | Session output token count |
| `total_tokens` | Sum |
| `tool_calls_total` | Total tool invocations (including Write, Edit, Bash housekeeping) |
| `research_calls` | `tool_calls_total` minus (Write + Edit + system Bash calls like mkdir, cd, cat, git) |
| `mcp_calls` | analyze_directory / analyze_file / analyze_symbol invocations |
| `native_calls` | Glob / Grep / Read / Bash invocations (file exploration only) |
| `cache_write_tokens` | Tokens written to prompt cache (parse from usage block if present, else 0) |
| `cache_read_tokens` | Tokens read from prompt cache (parse from usage block if present, else 0) |
| `cost_usd` | Estimated cost at model pricing (Sonnet or Haiku rates) |
| `protocol_violations` | Count of tool isolation violations detected by validate.py |
| `valid_output` | Boolean: did run produce valid JSON deliverable |
| `wall_time_s` | Duration in seconds |

### Research Calls Definition

A **research call** is a tool invocation that contributes to discovery: Glob, Grep, Read, Bash (for file exploration), or MCP tools (analyze_*).

**Excluded from research call count:**
- Write, Edit (output production)
- Bash with housekeeping intent (mkdir, cd, git, cat for verification, etc.)
- System messages and metadata turns

The 10-call budget is enforced **post-hoc** by `validate.py`, not as a hard gate in the runner. Runs exceeding 10 research calls are flagged as `FAIL` in validation but are not terminated.

## Statistical Analysis

### Primary Tests

**3 pairwise Mann-Whitney U tests on quality_score:**
- A vs B (native Sonnet vs MCP Haiku)
- A vs C (native Sonnet vs MCP Sonnet)
- B vs C (MCP Haiku vs MCP Sonnet)

**Bonferroni correction:** α = 0.05 / 3 = 0.017 per test.

**Effect size:** rank-biserial r; |r| ≥ 0.3 small, ≥ 0.5 medium, ≥ 0.7 large.

### Secondary Analyses

- **Tool call efficiency:** Median research_calls per condition; MCP vs native split in B and C.
- **Cost analysis:** Median cost_usd per condition; effective_cost_per_qp = cost_usd / (quality_score * reliability).
- **Protocol violations:** Count and type of tool isolation failures per condition.
- **Cache metrics:** Median cache_write_tokens and cache_read_tokens per condition (exploratory; caching disabled globally).
- **Token efficiency:** Median total_tokens per condition.

### Reporting

All analyses reported as **exploratory** due to small N=5 per condition. Statistical significance thresholds provided for reference only; conclusions weighted toward effect sizes and practical significance.

## Blinding

Randomized run order with seed=512 ensures neutral evaluation sequence. Blinding map in `scores-template.json` tracks real condition (A/B/C) but scorers do not see it during evaluation. Mapping revealed only after all scores submitted.

## Session Format and Tooling

v9 runs use Claude Code. Sessions stored as JSONL files under `~/.claude/projects/<project-slug>/<session-id>.jsonl`.

**collect.py** extends v8 to extract:
- `research_calls` = tool_calls_total - Write - Edit - housekeeping Bash
- `cache_write_tokens`, `cache_read_tokens` (default to 0 if not present)
- `protocol_violations` count

**validate.py** extends v8 to:
- Check Condition A: no MCP tools allowed
- Check Condition B: MCP tools required; no native file exploration tools
- Check Condition C: MCP tools required; native tools allowed
- Check all conditions: research_calls ≤ 10 (post-hoc validation, not enforcement)

**analyze.py** extends v8 to:
- Compute 3 pairwise Mann-Whitney U tests with Bonferroni α=0.017
- Report effect sizes (rank-biserial r)
- Summarize protocol violations per condition
- Report cache and token metrics

## References

- [README.md](README.md) — quick start and checklist
- [docs/benchmarks/ISOLATION.md](../ISOLATION.md) — tool isolation protocol
- [docs/benchmarks/v8/](../v8) — preceding benchmark (2-condition, prompt caching on)
