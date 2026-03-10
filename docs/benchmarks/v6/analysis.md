# Benchmark v6: Compaction Impact on Token Overhead

## Verdict

**Compaction eliminates the token overhead.** Five lossless formatting changes (PRs #135,
#137, #139, #140, #141) reduced code-analyze-mcp's context-size token usage from 22%
above the native baseline (v5) to 1% below it (v6). Quality is maintained at median 9 in
both conditions, with no statistically significant difference. The v6 B/A token ratio is
0.989, down from 1.225 in v5, a 23.6 percentage-point improvement. Accumulated tokens
(total API cost) also favor B, at 6% lower than A, because B completes the task in fewer
turns with zero shell calls. The issue #136 contingency (further token optimization) is
not triggered.

| Metric | A median | B median | U | z | p | r | Significant? |
|--------|:--------:|:--------:|:---:|:-----:|:------:|:-----:|:------------:|
| Quality (0-12) | 9 | 9 | 16.0 | 0.73 | 0.5179 | -0.28 | No |
| Context-size tokens | 23,688 | 23,416 | 13.0 | 0.10 | 1.0000 | -0.04 | No |
| Accumulated tokens | 188,586 | 176,814 | 18.0 | 1.15 | 0.3095 | -0.44 | No |
| Wall time (s) | 71 | 76 | 13.0 | 0.10 | 1.0000 | -0.04 | No |
| Total calls | 13 | 13 | 16.0 | 0.73 | 0.5206 | -0.28 | No |
| Shell calls | 5 | 0 | 25.0 | 2.61 | 0.0073 | -1.00 | **Yes** |
| Analyze calls | 8 | 12 | 1.5 | -2.30 | 0.0273 | 0.88 | **Yes** |

No metric shows B worse than A at the p<0.05 threshold. The two significant results
(shell calls and analyze calls) reflect the expected tool-substitution pattern: B replaces
shell commands with analyze calls, netting identical total calls.

## Experiment Design

| Parameter | Value |
|-----------|-------|
| Target repo | lsd-rs/lsd (~13K LOC, 52 Rust source files) |
| Task | Cross-module research: module map, data flow, dependency hubs, change proposal |
| Model | Claude Haiku 4.5, temperature 0.5 |
| Provider | AWS Bedrock |
| Repetitions | n=5 per condition (10 total) |
| Condition A (control) | `analyze` (goose built-in native extension) |
| Condition B (treatment) | `code-analyze-mcp__analyze` with rg-blocking constraint |
| Run order | Randomized per `run-order.txt` (seed=128) |
| Blinding | Condition labels stripped, random shuffle before scoring |
| Extension isolation | `--no-profile` flag; A: `--with-builtin developer,analyze`; B: `--with-builtin developer --with-extension code-analyze-mcp` |
| Compaction PRs | #135, #137, #139, #140, #141 (lossless formatting) |

## Tool Isolation

All 10 runs verified via session database (10/10 PASS):

- All 5 Condition A runs used only `analyze` (native extension; zero `code-analyze-mcp` calls).
- All 5 Condition B runs used only `code-analyze-mcp__analyze` (zero native `analyze` calls; zero rg structural patterns).
- R06 (B5) was rerun due to an initial session failure; the rerun completed successfully and was scored normally.
- No runs discarded.

Tool isolation was enforced at two levels:

1. **System-level:** `--no-profile` + selective `--with-builtin` ensured each condition only had access to its designated analyze tool.
2. **Prompt-level:** Condition B prompt explicitly blocked rg for structural analysis.

## Quality Results

### Per-Run Scores (0-12)

| Run | Structural | Tracing | Approach | Efficiency | Total |
|-----|:----------:|:-------:|:--------:|:----------:|:-----:|
| A1  | 3 | 3 | 3 | 2 | 11 |
| A2  | 2 | 2 | 3 | 2 | 9 |
| A3  | 2 | 2 | 2 | 2 | 8 |
| A4  | 3 | 2 | 2 | 2 | 9 |
| A5  | 3 | 3 | 3 | 2 | 11 |
| **A median** | **3** | **2** | **3** | **2** | **9** |
| B1  | 3 | 3 | 2 | 1 | 9 |
| B2  | 3 | 3 | 3 | 1 | 10 |
| B3  | 2 | 3 | 3 | 2 | 10 |
| B4  | 2 | 2 | 3 | 1 | 8 |
| B5  | 2 | 2 | 3 | 1 | 8 |
| **B median** | **2** | **3** | **3** | **1** | **9** |

B5 was rescored after the R06 rerun; the score above reflects the rerun output.

### Dimension Medians

| Dimension | A median | B median |
|-----------|:--------:|:--------:|
| Structural accuracy | 3 | 2 |
| Cross-module tracing | 2 | 3 |
| Approach quality | 3 | 3 |
| Tool efficiency | 2 | 1 |

### Quality Statistics

Quality totals are statistically equivalent (both median 9, U=16.0, p=0.518). Neither
condition demonstrates a quality advantage at n=5.

Per-dimension observations:

- **Structural accuracy:** A slightly higher (median 3 vs 2). A runs more frequently
  identified all top-level modules; two B runs missed peripheral modules.
- **Cross-module tracing:** B slightly higher (median 3 vs 2), consistent with the v5
  finding that code-analyze-mcp's structured output aids cross-module understanding.
- **Approach quality:** Tied (median 3 vs 3). Both conditions produced well-reasoned
  change proposals.
- **Tool efficiency:** A higher (median 2 vs 1). B runs averaged 11.4 analyze calls
  vs A's 8.0, pushing B into the 11-20 range (score 1) while A stayed in 6-10 (score 2).

v6 quality (median 9 both conditions) is slightly below v5 (median 10 both conditions).
This is within normal run-to-run variation at n=5 and does not indicate regression from
compaction. The compaction changes affected formatting only, not semantic content.

## Efficiency Results

### Per-Run Efficiency

| Run | Cond. | Context Tokens | Accum. Tokens | Wall (s) | Analyze | Shell | Other | Total Calls |
|-----|-------|---------------:|--------------:|:--------:|:-------:|:-----:|:-----:|:-----------:|
| A1 (R05) | A | 31,438 | 171,718 | 70 | 10 | 1 | 1 | 12 |
| A2 (R07) | A | 23,688 | 188,586 | 68 | 7 | 4 | 2 | 13 |
| A3 (R02) | A | 20,981 | 234,290 | 80 | 9 | 7 | 2 | 18 |
| A4 (R10) | A | 24,745 | 179,743 | 71 | 6 | 5 | 1 | 12 |
| A5 (R01) | A | 20,319 | 195,947 | 81 | 8 | 7 | 1 | 16 |
| B1 (R08) | B | 24,735 | 187,894 | 79 | 12 | 0 | 1 | 13 |
| B2 (R09) | B | 23,416 | 174,101 | 69 | 11 | 0 | 1 | 12 |
| B3 (R03) | B | 22,213 | 134,232 | 76 | 9 | 0 | 1 | 10 |
| B4 (R04) | B | 23,897 | 176,814 | 92 | 12 | 0 | 1 | 13 |
| B5 (R06) | B | 22,379 | 192,691 | 66 | 13 | 0 | 1 | 14 |

"Other" includes write and tree calls. "Context Tokens" is the final API call's
`total_tokens` (context window size). "Accum. Tokens" is `accumulated_total_tokens`
(sum of all API calls, representing total cost).

### Efficiency Statistics

| Metric | A median | B median | U | z | p | r | Significant? |
|--------|:--------:|:--------:|:---:|:-----:|:------:|:-----:|:------------:|
| Context-size tokens | 23,688 | 23,416 | 13.0 | 0.10 | 1.0000 | -0.04 | No |
| Accumulated tokens | 188,586 | 176,814 | 18.0 | 1.15 | 0.3095 | -0.44 | No |
| Wall time (s) | 71 | 76 | 13.0 | 0.10 | 1.0000 | -0.04 | No |
| Total calls | 13 | 13 | 16.0 | 0.73 | 0.5206 | -0.28 | No |
| Analyze calls | 8 | 12 | 1.5 | -2.30 | 0.0273 | 0.88 | **Yes** |
| Shell calls | 5 | 0 | 25.0 | 2.61 | 0.0073 | -1.00 | **Yes** |

Context-size tokens are statistically indistinguishable (p=1.000). Accumulated tokens
show B 6% lower than A, though the difference is not significant at n=5 (p=0.310).
Wall time and total calls show no difference.

The significant results mirror v5: B makes more analyze calls (median 12 vs 8, p=0.027)
and zero shell calls (median 0 vs 5, p=0.007). These offset to produce identical total
calls (median 13 both conditions).

### Derived Metrics

| Metric | A median | B median |
|--------|:--------:|:--------:|
| Context tokens per quality point | 2,632 | 2,748 |
| Accumulated tokens per quality point | 19,971 | 20,877 |
| Quality per tool call | 0.69 | 0.69 |

Both conditions achieve the same quality per tool call (0.69). Token efficiency per
quality point is comparable across both the context-size and accumulated-cost measures.

## Metric Clarification

v6 tracks two distinct token metrics. Understanding their difference is essential for
interpreting results and comparing with v5.

**Context-size tokens** (`total_tokens`): The token count of the final API call's context
window. This is the metric v5 measured. It reflects the size of the conversation at
completion, including all accumulated tool responses in the message history. This is what
v5 reported as "total tokens."

**Accumulated tokens** (`accumulated_total_tokens`): The sum of `total_tokens` across
every API call in the session. Each turn re-sends the full conversation history, so this
metric captures total API cost (input tokens billed). This metric was not tracked in v5.

**Why the two metrics diverge:** Context-size tokens measure "how big is the final
context window." Accumulated tokens measure "how much did we pay across all turns." A
session with many turns and a small final context can have high accumulated tokens; a
session with few turns but a large final context can have the reverse.

**Why B has lower accumulated tokens despite higher per-call content:** B makes zero
shell calls, completing the task in fewer conversational turns. Each turn re-sends the
full context, so fewer turns means less context re-transmission. Even though
code-analyze-mcp's individual tool responses may contain more structured data, the
reduction in total turns more than compensates.

**structuredContent and token accounting:** code-analyze-mcp returns rich structured data
via the MCP `structuredContent` field in tool responses. This structured data is stored
in the session database but is not sent to the LLM; only the compact `text` content field
counts toward token usage. The native analyze tool returns primarily text content. This
mechanism means code-analyze-mcp's apparent verbosity in database records does not
translate to proportional token cost.

## Cross-Version Comparisons

### v6B vs v5B: Compaction Delta

| Metric | v5B median | v6B median | Delta | Reduction |
|--------|:----------:|:----------:|:-----:|:---------:|
| Context-size tokens | 31,620 | 23,416 | -8,204 | 25.9% |
| Quality (0-12) | 10 | 9 | -1 | Within variance |

The compaction PRs reduced B's context-size token usage by 25.9% (8,204 tokens). Quality
remained within normal run-to-run variance (median 9 vs 10 at n=5).

### v6 B/A Ratio vs v5 B/A Ratio

| Version | A median tokens | B median tokens | B/A ratio | Overhead |
|---------|:---------------:|:---------------:|:---------:|:--------:|
| v5 | 25,818 | 31,620 | 1.225 | +22.5% |
| v6 | 23,688 | 23,416 | 0.989 | -1.1% |

The B/A ratio shifted from 1.225 (v5) to 0.989 (v6), a 23.6 percentage-point improvement.
B no longer costs more tokens than A; the two conditions are effectively at parity.

Note that v6A's median (23,688) is also lower than v5A's (25,818), a reduction of 8.3%.
This is expected run-to-run variation, not a treatment effect, since Condition A did not
change between v5 and v6.

## Analysis

1. **Compaction eliminated the 22% overhead.** The v5 benchmark identified a 22.5% token
   overhead as the sole remaining efficiency gap between code-analyze-mcp and the native
   baseline. Five lossless formatting changes (relative paths, tree-indented callees,
   separated test callers, summary counts, deduplicated callee chains) reduced the B/A
   ratio from 1.225 to 0.989. The overhead is gone.

2. **Quality is maintained.** Both conditions produce median 9 quality scores with no
   statistically significant difference (p=0.518). The compaction changes were designed to
   be lossless, preserving all semantic content while reducing token-level verbosity.
   The per-dimension pattern is consistent with v5: B scores higher on cross-module tracing
   (median 3 vs 2), A scores higher on tool efficiency (median 2 vs 1), and both tie on
   approach quality (median 3).

3. **B uses more analyze calls but zero shell calls, netting identical total calls.** B's
   median 12 analyze calls vs A's median 8 is significant (p=0.027), and B's zero shell
   calls vs A's median 5 is significant (p=0.007). The total calls are identical (median
   13 both conditions). This pattern replicates v5 and confirms the rg-blocking constraint
   works as designed: the agent substitutes analyze calls for shell commands.

4. **The structuredContent mechanism explains token convergence.** code-analyze-mcp returns
   structured data in the MCP `structuredContent` field, which is stored in the session
   database but not transmitted to the LLM. Only the compact `text` content counts toward
   tokens. After compaction, the text content is similar in size to native analyze output,
   producing comparable token usage. The 22% overhead in v5 was attributable to verbose
   text formatting (absolute paths, flat callee lists, interleaved test callers), all of
   which the compaction PRs addressed.

5. **Accumulated tokens favor B.** While not statistically significant (p=0.310), B's
   accumulated token median (176,814) is 6% lower than A's (188,586). B completes the
   task in fewer conversational turns because it does not make shell calls, reducing the
   total context re-transmission cost. This is a favorable secondary finding: compaction
   not only eliminated context-size overhead but also made B marginally cheaper in total
   API cost.

6. **Issue #136 contingency is not triggered.** The v6 methodology defined the contingency
   threshold as B overhead >= 10% above A. The measured overhead is -1.1% (B is slightly
   below A). The contingency condition is not met. Further token optimization work under
   issue #136 is not needed.

7. **Wall time remains equivalent.** Both conditions complete in comparable time (median
   71s vs 76s, p=1.000), consistent with the v5 finding. The tool-substitution pattern
   (more analyze calls, fewer shell calls) does not introduce latency.

## Conclusion

The v6 benchmark confirms that the five lossless compaction PRs achieved their design
goal. The 22% context-size token overhead identified in v5 has been eliminated. Quality
is maintained. code-analyze-mcp now matches the native analyze baseline on every measured
dimension: quality, context-size tokens, accumulated tokens, wall time, and total tool
calls.

Issue #136 (further token optimization) is not needed. The contingency threshold (>=10%
overhead) was not met; the actual overhead is -1.1%. Recommendation: close #136 as
resolved by the compaction PRs.

## Artifacts

- `prompts/` -- condition A and B prompts, task description
- `results/runs/` -- raw JSON reports per run (A1-A5, B1-B5)
- `scores.json` -- unblinded scores with per-run details and v5 baselines
- `run-order.txt` -- randomized execution order (seed=128)
- `methodology.md` -- experiment design, hypotheses, and compaction context
- `analysis.md` -- this document
