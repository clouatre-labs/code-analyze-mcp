# v8 Benchmark Methodology

## Research Question

Does augmenting Claude Code with code-analyze-mcp semantic tools improve agent performance on deep cross-module code analysis tasks compared to native file-system tools alone?

## Theoretical Grounding

This experiment is grounded in cognitive load theory (Sweller, 1988) applied to LLM tool use. The hypothesis is that semantic tools pre-extract structured information from code — function signatures, call graphs, import graphs — thereby reducing the extraneous cognitive load imposed on the model during analysis. Native file-system tools (Grep, Glob) require the model to filter, parse, and synthesize raw text, which competes for context budget with reasoning. Semantic tools shift this work to a deterministic pre-processor, potentially freeing context for higher-order synthesis.

The treatment condition is not a restriction but an augmentation: agents in B have a strictly larger tool set. Any quality difference is therefore attributable to the availability and use of semantic tools, not to reduced access to information.

## Hypotheses

**H1 (Quality):** Condition B produces higher total rubric scores than Condition A.

**H2 (Efficiency):** Condition B achieves equal or better quality with fewer total tokens.

**H3 (Cost-effectiveness):** Condition B achieves a lower effective cost per quality point:
`effective_cost_per_qp = cost_usd / (quality_score * reliability)`

**H0 (Null):** No significant difference in quality score distributions between conditions (Mann-Whitney U, α = 0.05).

## Experimental Design

### Conditions

| | Condition A — Control | Condition B — Treatment |
|---|---|---|
| **Name** | Claude Code (native tools only) | Claude Code + code-analyze-mcp |
| **Glob, Grep, Read, Bash** | Available | Available |
| **analyze_directory / analyze_file / analyze_symbol** | Not available | Available (preferred) |

### Participants

- N = 10 runs total, 5 per condition
- Model: `claude-haiku-4-5`, temperature = 0.5
- Run order: randomized with seed = 512
- Blinding: scorer blind to condition assignment during scoring; label map revealed post-scoring

### Target Repository

**tobymao/sqlglot** — Python SQL parser and transpiler. Commit pinned at benchmark execution time and recorded in `run-order.txt`.

### Run Order (seed = 512)

1. A2, 2. A5, 3. B5, 4. B1, 5. B2, 6. A4, 7. B4, 8. A3, 9. B3, 10. A1

Mapping: R01=A2, R02=A5, R03=B5, R04=B1, R05=B2, R06=A4, R07=B4, R08=A3, R09=B3, R10=A1

## Metrics

Per-run metrics recorded in `RXX.json`:

| Metric | Description |
|---|---|
| `quality_score` | Sum of 4 rubric dimensions (0–12) |
| `structural_accuracy` | Rubric dim 1 (0–3) |
| `cross_module_tracing` | Rubric dim 2 (0–3) |
| `approach_quality` | Rubric dim 3 (0–3) |
| `tool_efficiency` | Rubric dim 4 (0–3) |
| `input_tokens` | Session input token count |
| `output_tokens` | Session output token count |
| `total_tokens` | Sum |
| `wall_time_s` | Duration in seconds |
| `tool_calls_total` | Total tool calls |
| `mcp_calls` | `analyze_*` calls (B only) |
| `native_calls` | Glob/Grep/Read/Bash calls (both conditions) |
| `valid_output` | Boolean: did run produce valid JSON deliverable |
| `cost_usd` | Estimated cost at haiku-4-5 pricing |

## Rubric (0–3 per dimension, max 12)

### Structural Accuracy (0–3)

| Score | Criteria |
|---|---|
| 3 | Correctly identifies all major modules (`expressions.py`, `tokens.py`, `parser.py`, `generator.py`, `dialects/`, `optimizer/`, `planner.py`, `executor/`); responsibilities clearly defined; key types accurately described |
| 2 | Identifies most modules; minor omissions; core pipeline modules present |
| 1 | Partial coverage; missing optimizer or executor; vague on Expression hierarchy depth |
| 0 | Major components missing; fundamental misunderstanding of project structure |

### Cross-Module Tracing (0–3)

| Score | Criteria |
|---|---|
| 3 | Complete trace from SQL string through `Tokenizer` → `Parser` → `Expression` tree → `Generator` output; intermediate types identified at each stage; dialect override points noted |
| 2 | Key stages present; minor gaps (e.g., optimizer pass not mentioned, or type at one stage missing) |
| 1 | Partial trace; missing multiple stages or intermediate types |
| 0 | No meaningful trace or entirely incorrect |

### Approach Quality (0–3)

| Score | Criteria |
|---|---|
| 3 | Change proposal identifies: correct files (`expressions.py` for new class, `generator.py` for `levenshtein_sql` method, dialect file for overrides); new `Levenshtein` Expression subclass; follows existing scalar function pattern (e.g., `Soundex`); integration via dialect's `FUNCTIONS` map; realistic risks |
| 2 | Reasonable proposal; most files and types identified; risks partially addressed |
| 1 | Incomplete; missing files, wrong integration point, or no risk analysis |
| 0 | Superficial or incorrect proposal |

### Tool Efficiency (0–3)

| Score | Criteria |
|---|---|
| 3 | ≤ 5 tool calls; focused exploration; clear synthesis path |
| 2 | 6–10 tool calls; somewhat exploratory but reaches conclusions |
| 1 | 11–20 tool calls; extensive exploration; delayed synthesis |
| 0 | > 20 tool calls; inefficient; redundant or circular exploration |

## Statistical Analysis

- **Primary test:** Mann-Whitney U on `quality_score` distributions (A vs B), two-tailed, α = 0.05
- **Secondary tests:** Mann-Whitney U on `total_tokens`, `cost_usd`, `effective_cost_per_qp`
- **Tool call analysis:** Median tool calls per condition; `mcp_calls` vs `native_calls` ratio in B
- **Report:** U-statistic, z-score, p-value, rank-biserial r (|r| ≥ 0.3 small, ≥ 0.5 medium, ≥ 0.7 large)

## Session Format Note

v8 runs use Claude Code (not Goose). Sessions are stored as JSONL files under `~/.claude/projects/<project-slug>/<session-id>.jsonl`. `collect.py` reads this format directly. Token usage is extracted from `usage.input_tokens` / `usage.output_tokens` fields in assistant messages.

## Blinding

Randomized order with seed=512 ensures neutral evaluation sequence. Mapping in `scores-template.json` tracks real condition (A/B) but scorers are blind during evaluation; mapping revealed only after all scores submitted.

## References

- Sweller, J. (1988). Cognitive load during problem solving. *Cognitive Science*, 12(2), 257–285.
- [docs/benchmarks/v7/](../v7) — preceding benchmark (parameter discovery)
- [docs/benchmarks/ISOLATION.md](../ISOLATION.md) — tool isolation protocol
