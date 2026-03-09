# Methodology: V5 Benchmark (Tool Isolation & Efficiency)

## Experiment Overview

This document describes the design, execution, and analysis methodology for the v5 benchmark, which tests whether the `code-analyze-mcp` MCP server can match or exceed the `developer__analyze` baseline when combined with an explicit rg-blocking constraint.

## Experiment Design

| Aspect | Details |
|--------|---------|
| **Objective** | Validate code-analyze-mcp efficiency under tool isolation constraints; measure optimization delta (v5B vs v3B) and gap closure (v5B vs v3A) |
| **Conditions** | A (control): developer__analyze baseline; B (treatment): code-analyze-mcp__analyze with rg-blocking constraint |
| **Hypothesis** | Explicit tool isolation (prompt-enforced rg blocking + disabled native analyze) will improve Condition B efficiency without sacrificing quality |
| **Sample Size** | n=5 per condition (10 total runs) |
| **Randomization** | Blinded; run order shuffled with seed=124; scorers blind to condition labels |
| **Primary Outcome** | Tool efficiency (≤5 calls = 3, 6-10 = 2, 11-20 = 1, >20 = 0); secondary: quality (structural_accuracy, cross_module_tracing, approach_quality) |
| **Statistical Test** | Mann-Whitney U (non-parametric, small n); rank-biserial r for effect size |
| **Decision Threshold** | p < 0.05 for significance; r ≥ 0.3 for meaningful effect |

## Conditions

| Condition | Tool Choice | Constraints | Native Analyze Extension | rg Constraint | Expected Behavior |
|-----------|------------|-------------|--------------------------|---------------|-------------------|
| **A: Control** | developer__analyze (goose built-in) | Standard prompt; tool available | Enabled | None (standard) | Baseline efficiency; leverages native extension |
| **B: Treatment** | code-analyze-mcp__analyze | rg-blocking prompt; focused structural tool | Disabled in goose config | "Do NOT use rg or cat to understand code structure... Use code-analyze-mcp__analyze exclusively" | Forced reliance on MCP tool; should reveal efficiency gains from focused queries |

## Tool Isolation

**Critical Requirement:** Condition B must enforce tool isolation at two levels:

1. **Prompt-level (documented here):** Condition B prompt explicitly blocks rg for structural analysis and native analyze extension.
2. **System-level (during execution):** Goose configuration must have the native `developer__analyze` extension disabled for Condition B runs.

Validation script (`validate.py`) will check:
- **Condition A:** developer__analyze used; no code-analyze-mcp calls.
- **Condition B:** code-analyze-mcp__analyze used; no developer__analyze calls; no rg structural patterns (rg + keywords like 'fn ', 'struct ', 'impl ', 'mod ', 'use ').

## Scoring Rubric

All dimensions use 0-3 scale. Total maximum: 12 points.

### Structural Accuracy (0-3)

- **3:** Correctly identifies all major modules (core, display, meta, icon, color, flags, theme, sort); responsibilities clearly mapped; key types (Block, Blocks, Meta) accurately described; call sequence aligns with codebase.
- **2:** Identifies 5-6 major modules correctly; 1-2 responsibilities mischaracterized or generic; key types mostly correct; minor call sequence gaps.
- **1:** Identifies 3-4 modules; significant gaps in responsibility mapping or type understanding; call sequence partially correct or out of order.
- **0:** Identifies fewer than 3 modules; fundamentally incorrect structure or types; no meaningful sequence.

### Cross-Module Tracing (0-3)

- **3:** Traces Meta::from_path through sorting, icon resolution, color resolution, Block creation, and display_grid output; intermediate types (Vec<Meta>, Icons, Colors, Vec<Block>) named at each stage; call sites accurate.
- **2:** Traces 4-5 stages with correct types; 1-2 intermediate steps unclear or generalized; overall flow correct.
- **1:** Traces 2-3 stages; significant gaps in intermediate types or module boundaries; flow partially reconstructed.
- **0:** No meaningful trace or fundamentally incorrect; cannot reconstruct flow.

### Approach Quality (0-3)

- **3:** Proposes checksum column with: correct files to modify (display.rs for rendering, meta.rs for type addition, likely block.rs for field); new type (Checksum struct or field); integration pattern matching existing color/icon handlers; 3+ identified risks (performance, display width, sorting implications).
- **2:** Proposes reasonable solution with 2-3 files, new type present; integration sketch present; 2 risks identified.
- **1:** Identifies some files and a type; integration vague; 1 risk or generic risks.
- **0:** Superficial proposal, missing files, or no type definition; no realistic integration path.

### Tool Efficiency (0-3)

- **3:** ≤5 analysis tool calls; focused queries that systematically reduce uncertainty; minimal backtracking.
- **2:** 6-10 analysis tool calls; mostly focused; some exploratory calls; reaches synthesis without excessive detours.
- **1:** 11-20 analysis tool calls; notable redundancy or broad exploration; delayed synthesis; writer apparent hesitation.
- **0:** >20 analysis tool calls; highly exploratory or repetitive; late synthesis or abandoned queries.

## Cross-Version Comparisons

### v5B vs v3B (Optimization Delta)

Measures whether the rg-blocking constraint forces efficiency improvements:

- **Quality (structural_accuracy, cross_module_tracing, approach_quality):** v5B should maintain or exceed v3B median.
- **Efficiency:** v5B should reduce median tool calls and wall time vs v3B.
- **Tokens:** v5B may use fewer tokens if forced focus improves clarity.

Success criteria:
- v5B quality ≥ v3B median (no regression).
- v5B tool calls < v3B tool calls (constraint-driven efficiency).
- v5B wall time ≤ v3B wall time (optimization realized).

### v5B vs v3A (Gap Closure)

Measures whether code-analyze-mcp can match the native analyze baseline:

- **Quality:** v5B median quality ≥ v3A median (can code-analyze-mcp reach parity?).
- **Efficiency:** v5B tool calls, tokens, wall time comparable to v3A (if quality matches, efficiency must be competitive).

Success criteria:
- v5B quality ≥ v3A quality (feature parity achieved).
- v5B efficiency metrics close to v3A (within 10-20% or statistically equivalent).

## Blinding Procedure

1. **Run order:** Randomized with seed=124 to avoid order bias.
2. **Labeling:** 10 runs labeled R01-R10 during scoring; condition mapping kept secret until final tally.
3. **Scoring:** Each run scored independently on rubric before condition is revealed.
4. **Mapping disclosure:** After all 10 runs scored, reveal condition mapping and separate A/B results.

Mapping (R## → A/B condition):
- Generated by seed=124 shuffle; stored in scores-template.json blinding.mapping after execution.

## Statistical Analysis Plan

### Mann-Whitney U Test

Used to compare quality and efficiency between Condition A and B:

1. **Null hypothesis:** No difference in quality (or efficiency) between conditions.
2. **Alternative:** Conditions differ (two-tailed).
3. **Calculation:** Rank all 10 observations combined; compute U = n1*n2 + n1(n1+1)/2 - R1 (R1 = sum of ranks for group 1).
4. **Critical value:** α=0.05, two-tailed, n1=n2=5 → critical U = 2.
5. **p-value:** Use exact distribution for small n (scipy.stats.mannwhitneyu if available).
6. **Effect size:** Rank-biserial r = 1 - 2U/(n1*n2). |r| ≥ 0.3 = small effect, ≥ 0.5 = medium, ≥ 0.7 = large.

### Dimensions Tested

1. **Quality:** Pool structural_accuracy, cross_module_tracing, approach_quality; compare A vs B totals.
2. **Efficiency:** Mann-Whitney on tool_calls (analyze_calls + shell_calls for structural work); tokens; wall time.
3. **Tool efficiency score:** Direct comparison of tool_efficiency dimension (rubric-driven, already 0-3).

### Interpretation

- **Significant efficiency gain (p<0.05, r>0.5):** Condition B tools isolate is effective; recommend for production.
- **No significant difference:** Conditions equivalent; choose based on cost/availability.
- **Regression in B:** Investigate whether constraint is too strict; refine prompt.

## Cross-Version Comparison Strategy

Using v3 baselines (embedded in scores-template.json):

1. **Extract v3 per-run scores** from v3/scores.json.
2. **Compute v3 medians:** A=10 (range 9-10), B=10 (range 9-11).
3. **After v5 runs complete, compute v5 medians:** A and B.
4. **Calculate deltas:**
   - v5B vs v3B quality delta = v5B_quality - v3B_quality (should be ≥0).
   - v5B vs v3B efficiency delta = v5B_median_calls - v3B_median_calls (should be ≤0 if improvement).
   - v5B vs v3A quality gap = v5B_quality - v3A_quality (should be ≥0 for parity).
5. **Publish** in analysis.md with decision recommendation.

## Decision Framework

| Scenario | Quality (v5B vs baseline) | Efficiency (v5B vs v3B) | Recommendation |
|----------|-------------------------|------------------------|-----------------|
| v5B ≥ v3A quality AND v5B < v3B calls | Parity achieved + optimization | Approve; recommend v5B | Deploy code-analyze-mcp as recommended MCP tool |
| v5B ≥ v3B quality AND v5B < v3B calls | Improvement over prior treatment | Strong approval | Deploy; updated treatment baseline |
| v5B ≥ v3B quality AND v5B ≥ v3B calls | No efficiency gain | Conditional; weigh quality | Investigate constraint tightness; revise if needed |
| v5B < v3A quality | Regression to baseline | Failure | Revise prompt/constraint; rerun v5 or stay with v3A |

## v3 Baseline Reference

Provided in scores-template.json under `v3_baselines`:

- **Condition A:** Median quality 10, range [9-10], median tool calls 14
- **Condition B:** Median quality 10, range [9-11], median tool calls 18

(Full per-run scores and efficiency metrics in v3/scores.json)

## Execution Checklist

- [ ] Create docs/benchmarks/v5/prompts/task.md (copy from v3, identical task)
- [ ] Create docs/benchmarks/v5/prompts/condition-a-control.md (copy from v3, identical control)
- [ ] Create docs/benchmarks/v5/prompts/condition-b-treatment.md (fork v3, add rg-blocking text)
- [ ] Create docs/benchmarks/v5/run-order.txt with randomized 10-run order (seed=124)
- [ ] Create docs/benchmarks/v5/scores-template.json with rubric, v3 baselines, null placeholders
- [ ] Disable native analyze extension in goose config for Condition B runs
- [ ] Run 10 goose sessions following run-order.txt (alternating A/B)
- [ ] Collect session metrics using scripts/collect.py (tokens, wall time, tool calls)
- [ ] Score each run using scores-template.json rubric (blind to condition until mapped)
- [ ] Validate tool isolation using scripts/validate.py for each run
- [ ] Run scripts/analyze.py on completed scores.json to generate Mann-Whitney U, r, cross-version comparisons
- [ ] Write docs/benchmarks/v5/analysis.md with findings, decision recommendation, cross-version insights
- [ ] Commit all results to GitHub with PR for review

## Output Artifacts

- **docs/benchmarks/v5/prompts/task.md** – Experiment task (reused from v3)
- **docs/benchmarks/v5/prompts/condition-a-control.md** – Control prompt (reused from v3)
- **docs/benchmarks/v5/prompts/condition-b-treatment.md** – Treatment prompt (rg-blocking variant)
- **docs/benchmarks/v5/run-order.txt** – Randomized run execution sequence
- **docs/benchmarks/v5/scores-template.json** – Scoring template (filled during execution)
- **docs/benchmarks/v5/results/runs/R##.json** – Individual session outputs (10 total)
- **docs/benchmarks/v5/scripts/analyze.py** – Statistical analysis script
- **docs/benchmarks/v5/scripts/validate.py** – Tool isolation validation script
- **docs/benchmarks/v5/scripts/collect.py** – Metrics extraction script
- **docs/benchmarks/v5/analysis.md** – Final results and interpretation
