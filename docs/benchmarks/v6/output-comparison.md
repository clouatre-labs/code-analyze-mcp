# v6 Output Comparison

Compare code-analyze-mcp tool responses between v5 (pre-compaction) and v6 (post-compaction)
to quantify the impact of the 5 lossless changes.

## Compaction Changes Under Test

| Issue | PR | Change | Primary mode affected |
|-------|-----|--------|----------------------|
| #129 | #135 | Relative paths in all output modes | All modes (path headers) |
| #130 | #137 | Tree-indent callees and callers in focus mode | Focus (callee/caller sections) |
| #132 | #139 | Separate test callers into counted summary | Focus (caller section) |
| #133 | #140 | Add summary counts to FOCUS header | Focus (header) |
| #134 | #141 | Deduplicate repeated callee chains with (xN) | Focus (callee section) |

## Overview Mode

Overview mode was not a significant verbosity contributor in v5 (1,652 vs 1,533 chars).
Only #129 (relative paths) applies here; however, overview already used relative paths in v5,
so no change was expected or observed.

| Metric | v5B | v6B | Delta | Notes |
|--------|:---:|:---:|:-----:|-------|
| Avg response chars | 1,652 | 1,652 | 0 (0.0%) | Overview already used relative paths |
| Avg response lines | 62 | 62 | 0 | Identical output; same codebase |

All 5 v6B runs produced the identical 1,652-char / 62-line overview response, matching the v5B
value exactly. Overview mode was not a target of the compaction changes.

## File Details Mode

File details mode was comparable between conditions in v5 (565 vs 613 chars for core.rs).
#129 (relative paths) applies to file headers, replacing absolute paths with relative ones.

| Metric | v5B | v6B | Delta | Notes |
|--------|:---:|:---:|:-----:|-------|
| Avg response chars | 1,279 | 1,309 | +30 (+2.3%) | Different file selections across runs |
| Avg response lines | n/a | 34 | | v5 per-response lines not recorded |
| Path length (avg chars) | ~50 | 8 | -42 (-84.0%) | Absolute vs relative |

The per-response average is similar because file details content is dominated by function
signatures and import lists, not paths. The path header itself shrank by ~42 chars per response
(e.g., `/Users/hugues.clouatre/git/lsd-rs/lsd/src/core.rs` to `core.rs`). For the matched
`core.rs` file: v5B = 565 chars, v6B = 523 chars (-7.4%).

v5B reference: 12,786 total file details chars across 10 calls (from v5 B5 run).
v6B: 33 file details responses across 5 runs, average 1,309 chars each.

## Symbol Focus Mode

Focus mode was the primary verbosity driver in v5: 17,028 chars / 604 lines for `from_path`
alone. All 5 changes apply here. Measurements below compare the v5 B5 `from_path` response
against the v6B median `from_path` response (5 runs).

| Metric | v5B | v6B | Delta | Notes |
|--------|:---:|:---:|:-----:|-------|
| Avg response chars | 17,028 | 6,544 | -10,484 (-61.6%) | All 5 changes combined |
| Avg response lines | 604 | 338 | -266 (-44.0%) | |
| Callee lines (total) | 528 | 294 | -234 (-44.3%) | #130 tree-indent, #134 dedup |
| Caller lines (total) | 68 | 33 | -35 (-51.5%) | #130 tree-indent, #132 test summary |
| Test caller lines | 50 | 1 | -49 (-98.0%) | #132: 50 interleaved lines collapsed to 1 summary |
| Deduplicated chains | 0 | 88 | +88 | #134: (xN) annotations replace repeated flat lines |
| Header line | `FOCUS: from_path` | `FOCUS: from_path (2 defs, 15 callers, 27 callees)` | +38 chars | #133: summary counts added |

Focus mode comparison uses `from_path` as the matched query, present in all 5 v6B runs and
the v5 B5 detailed analysis. The v6B median values (depth 2) are shown; depth-3 runs (B1, B3)
produced 7,074 chars / 366 lines.

### Per-symbol v6B focus breakdown

| Symbol | Runs | Avg chars | Avg lines | Avg callees | Avg callers |
|--------|:----:|:---------:|:---------:|:-----------:|:-----------:|
| from_path | 5 | 6,756 | 349 | 306 | 32 |
| get_output | 4 | 4,919 | 264 | 242 | 12 |
| grid | 3 | 4,044 | 203 | 169 | 6 |
| sort | 1 | 572 | 34 | 15 | 6 |

## Aggregate Response Sizes

Total tool response characters across all analyze calls in a session. v5B values are from the
single detailed B5 run (42,228 chars / 13 calls); v5 sessions for B1-B4 are no longer available.

| Run | v5B chars | v6B chars | Delta (chars) | Delta (%) |
|-----|:---------:|:---------:|:-------------:|:---------:|
| B1 | -- | 28,040 | | |
| B2 | -- | 24,779 | | |
| B3 | -- | 20,263 | | |
| B4 | -- | 21,818 | | |
| B5 | 42,228 | 22,705 | -19,523 | -46.2% |
| **Median** | 42,228 (B5 only) | **22,705** | **-19,523** | **-46.2%** |

v5B per-run session data expired before v6 benchmarks ran. The v5 B5 reference is the only
available per-run character breakdown from v5. The -46.2% reduction reflects both compaction
changes and natural variation in which files/symbols the agent chose to analyze.

## Compaction Delta Summary

| Dimension | v5B median | v6B median | Reduction (%) | Target |
|-----------|:----------:|:----------:|:-------------:|--------|
| **Overall tokens** | 31,620 | 23,416 | 25.9% | <10% gap (v6B vs v6A) |
| Focus response chars | 27,790 (B5) | 12,026 | 56.7% | Largest contributor |
| File details response chars | 12,786 (B5) | 8,778 | 31.4% | Moderate contributor |
| Overview response chars | 1,652 | 1,652 | 0.0% | Minimal contributor |

v5B median token value (31,620) is from v5/scores.json across all 5 B runs.
v6B median token value (23,416) is from the sessions DB across all 5 B runs.
Per-mode v5B chars are from the single B5 detailed run; per-mode v6B chars are medians across
5 runs.

## Gap Closure

| Metric | v5 | v6 | Status |
|--------|:--:|:--:|--------|
| B/A token ratio | 1.22 (22% overhead) | 0.99 (-1.1% overhead) | Target met: <1.10 |
| B median quality | 10 | 9 | Target met: >= 9 |
| B median total calls | 14 | 13 | Stable |

The 22.5 percentage-point overhead reduction (from +22% to -1.1%) confirms the compaction
changes eliminated the token gap between code-analyze-mcp and native analyze. The v6 B/A ratio
of 0.989 means the tool condition now uses slightly fewer tokens than the control condition,
though the difference is not statistically significant (MWU p = 1.0).

Quality dropped by 1 point (median 10 to 9), remaining above the >= 9 target. The difference
is not statistically significant (MWU p = 0.518).

## Notes

- v5 baseline data from docs/benchmarks/v5/scores.json and v5/output-comparison.md
- Per-mode measurements extracted from session tool response content in the goose sessions DB
- All 5 compaction changes are lossless: identical semantic information, reduced encoding
- v5 root cause: callee explosion in focus mode (528 flat lines vs 294 tree-indented for same info)
- v5 sessions expired before v6 benchmarks; per-run v5B character data limited to the B5 detailed analysis
- 13 focus responses across 5 v6B runs; 33 file details responses; 5 overview responses
- Test callers collapsed from 50 interleaved lines to 1 counted summary line across all `from_path` queries
- 855 total (xN) dedup annotations across all 13 v6B focus responses (avg 66 per response)
