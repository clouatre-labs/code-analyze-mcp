# Analysis: v7-benchmark

**Model:** claude-haiku-4-5 (temperature=0.5)
**Provider:** aws_bedrock
**Runs:** 10 (5 per condition, randomized order, seed=256)
**Target repository:** lsd-rs/lsd (~13K LOC, 52 Rust source files)
**Date:** 2026-03-11

---

## Experimental Design

### Conditions

- **Condition A (Control):** `code-analyze-mcp__analyze` without parameter documentation.
  Agents have access to the tool but are not informed of `summary`, `cursor`, or `page_size` parameters.
- **Condition B (Treatment):** `code-analyze-mcp__analyze` with extended tool description
  documenting `summary` (collapse verbose output), `cursor` (pagination), and `page_size` (output
  size limit), including usage examples.

Both conditions use the same task (cross-module analysis of lsd), the same tool binary, and the
same model. The sole manipulation is whether parameter documentation is present in the tool
description.

### Context in the benchmark lineage

| Version | Condition A | Condition B | Research question |
|---------|-------------|-------------|-------------------|
| v5 | `developer__analyze` (goose native) | `code-analyze-mcp`, rg-blocked | Is MCP quality equivalent to native? |
| v6 | `developer__analyze` (goose native) | `code-analyze-mcp`, rg-blocked + 5 compaction PRs | Do lossless formatting improvements close the 22% token overhead? |
| v7 | `code-analyze-mcp`, no param docs | `code-analyze-mcp`, param docs | Does documenting optional parameters reduce token consumption? |

v7 does **not** compare the two tools. It tests a documentation treatment applied to one tool.
The MCP-vs-native comparison was conducted in v5 and v6.

---

## Quality Analysis

Scores are 0-3 per dimension (blind scoring, condition labels stripped before evaluation):

| Dimension | A median | B median | U | z | p | r |
|-----------|----------|----------|---|---|---|---|
| structural_accuracy | 3.0 | 2.0 | 15.0 | 0.52 | 0.631 | -0.20 |
| cross_module_tracing | 3.0 | 2.0 | 17.5 | 1.04 | 0.270 | -0.40 |
| approach_quality | 3.0 | 3.0 | 10.0 | -0.52 | 0.600 | +0.20 |
| tool_efficiency | 2.0 | 2.0 | 17.5 | 1.04 | 0.177 | -0.40 |
| **total** | **10.0** | **9.0** | **17.5** | **1.04** | **0.332** | **-0.40** |

**Interpretation:** No statistically significant quality difference between conditions
(all p > 0.05, n=5 per group). The 1-point median gap on `total` corresponds to a
small-to-medium effect (r=-0.40) that does not reach significance at this sample size.

### Per-run scores

| Run label | Condition | struct | trace | approach | eff | total |
|-----------|-----------|--------|-------|----------|-----|-------|
| A1 (R05) | A | 2 | 2 | 2 | 2 | 8 |
| A2 (R03) | A | 3 | 3 | 2 | 2 | 10 |
| A3 (R07) | A | 3 | 3 | 3 | 2 | 11 |
| A4 (R06) | A | 2 | 2 | 3 | 2 | 9 |
| A5 (R09) | A | 3 | 3 | 3 | 2 | 11 |
| B1 (R04) | B | 2 | 2 | 2 | 2 | 8 |
| B2 (R08) | B | 3 | 2 | 3 | 2 | 10 |
| B3 (R10) | B | 2 | 2 | 3 | 1 | 8 |
| B4 (R02) | B | 2 | 2 | 3 | 2 | 9 |
| B5 (R01) | B | 3 | 3 | 3 | 1 | 10 |

---

## Token Efficiency

Token counts are from the goose sessions database (`sessions.total_tokens`), which records
model input + output tokens for the session excluding accumulated context from prior sessions.

| Run | Label | Cond | Tokens | Analyze calls | summary calls | Shell calls |
|-----|-------|------|--------|---------------|---------------|-------------|
| R01 | B5 | B | 28,589 | 11 | 0 | 1 |
| R02 | B4 | B | 28,157 | 10 | 9 | 1 |
| R03 | A2 | A | 34,082 | 8 | 0 | 7 |
| R04 | B1 | B | 27,946 | 10 | 9 | 1 |
| R05 | A1 | A | 31,259 | 6 | 0 | 7 |
| R06 | A4 | A | 31,054 | 7 | 0 | 4 |
| R07 | A3 | A | 31,876 | 10 | 0 | 7 |
| R08 | B2 | B | 28,933 | 10 | 8 | 1 |
| R09 | A5 | A | 31,837 | 7 | 0 | 6 |
| R10 | B3 | B | 30,626 | 11 | 7 | 1 |

| Condition | Token median | Token range | Analyze calls median | Shell calls median |
|-----------|-------------|-------------|----------------------|--------------------|
| A | 31,837 | 31,054 -- 34,082 | 7 | 6 |
| B | 28,589 | 27,946 -- 30,626 | 10 | 1 |
| **Delta (B vs A)** | **-3,248 (-10.2%)** | | **+3** | **-5** |

**Mechanism:** B agents used `summary=true` on most analyze calls, which reduced per-call
output size and eliminated the need for supplementary shell (`rg`/`cat`) exploration. Despite
making more analyze calls (median 10 vs 7), the net token consumption was 10% lower because
shell calls dropped from a median of 6 to 1.

---

## Parameter Usage (Condition B)

| Parameter | Runs used | Adoption rate | Notes |
|-----------|-----------|---------------|-------|
| `summary=true` | 4/5 | 80% | B5 (R01) is an exception; see note below |
| `cursor` | 0/5 | 0% | lsd at ~13K LOC never triggered pagination thresholds |
| `page_size` | 0/5 | 0% | No agent encountered output large enough to motivate this |

**B5 note (R01):** This run used the extension under the old name `code-analyze`
(before the extension was renamed to `code-analyze-mcp` to match script expectations).
The tool calls are functionally identical; the session validated FAIL on the name check
but the data is included as valid because tool isolation was maintained -- the agent
exclusively used `code-analyze__analyze`, not the goose native `developer__analyze`.

### Summary call counts per B run

| Run | Label | summary_count |
|-----|-------|---------------|
| R01 | B5 | 0 (name mismatch; see note) |
| R02 | B4 | 9 |
| R04 | B1 | 9 |
| R08 | B2 | 8 |
| R10 | B3 | 7 |

---

## Cross-Version Comparison

### Quality across versions (same task, same model)

| Version | Condition A | A median | Condition B | B median | A vs B |
|---------|-------------|----------|-------------|----------|--------|
| v5 | developer__analyze | 10 | code-analyze-mcp, rg-blocked | 10 | no difference |
| v6 | developer__analyze | 9 | code-analyze-mcp, rg-blocked + compaction | 9 | no difference |
| v7 | code-analyze-mcp, no param docs | 10 | code-analyze-mcp, param docs | 9 | no significant difference (p=0.332) |

No version has produced a statistically significant quality difference between conditions.
v7A median (10) is one point above v7B median (9), within the range of individual run variance
observed across all versions (runs range 8--11 throughout).

### What the MCP-vs-native comparison shows (v5/v6)

v5 established that `code-analyze-mcp` produces equivalent quality output to the goose native
`developer__analyze` (both median 10). v6 replicated this finding after 5 compaction changes
(both median 9, p not significant). Neither version showed a quality advantage for either tool.

v7 cannot be used to make claims about MCP vs native: both conditions use the MCP. The
comparison is parameter-documented vs undocumented use of the same tool.

### Token comparison: v7B vs v6B

v6 did not instrument session-level token counts (runs stored task output only). The v6
methodology cites a 22% token overhead for B vs A derived from v5 measurements and projected
forward. v7B token data (median 28,589) cannot be directly compared to a v6B token figure
because no v6B token baseline exists in the data.

The measurable claim is: **v7B is 10.2% more token-efficient than v7A**. Whether this represents
an improvement over v6B requires token instrumentation of a v6 re-run, which was not conducted.

---

## Hypothesis Evaluation

| Hypothesis | Result |
|-----------|--------|
| Agents discover and use `summary` in >40% of B runs | **Supported** (80% adoption, 4/5 runs) |
| Token efficiency improves vs control | **Supported** (-10.2% vs v7A) |
| No quality regression in B vs A | **Supported** (p=0.332, not significant) |
| Agents discover `cursor` for pagination | **Not supported** (0% adoption; codebase too small to trigger) |
| Agents discover `page_size` | **Not supported** (0% adoption; same reason) |

---

## Limitations

1. **Sample size:** n=5 per condition. The experiment is adequately powered to detect large
   effects (r >= 0.7) but not medium effects (r=0.40 observed on total quality). Conclusions
   about quality equivalence are tentative.
2. **Single codebase:** lsd at ~13K LOC is mid-size. `cursor` and `page_size` are unlikely to
   be triggered at this scale. Testing on a larger repository is needed to evaluate those
   parameters.
3. **B5 name anomaly:** One B run (R01) used the tool under the old extension name. Functional
   tool isolation was maintained but the validate.py check flagged it as FAIL. The data point
   is included but the summary=0 for B5 may reflect the naming context rather than agent
   behavior.
4. **No v6B token baseline:** The cross-version token efficiency claim cannot be fully evaluated
   without re-running v6 with session-level token instrumentation.
5. **Single model:** All runs used claude-haiku-4-5. Results may not generalize to other models.

---

*Mann-Whitney U test, two-sided. rank-biserial r: |r| >= 0.3 small, >= 0.5 medium, >= 0.7 large.*
*p-values computed with scipy.stats.mannwhitneyu where available, otherwise approximated via normal distribution.*
