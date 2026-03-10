# v6 Output Comparison

Compare code-analyze-mcp tool responses between v5 (pre-compaction) and v6 (post-compaction)
to quantify the impact of the 5 lossless changes. Fill after benchmark runs complete.

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
Only #129 (relative paths) applies here.

| Metric | v5B | v6B | Delta | Notes |
|--------|:---:|:---:|:-----:|-------|
| Avg response chars | [AFTER RUNS] | [AFTER RUNS] | | #129: shorter path headers |
| Avg response lines | [AFTER RUNS] | [AFTER RUNS] | | |

## File Details Mode

File details mode was comparable between conditions in v5 (565 vs 613 chars for core.rs).
#129 (relative paths) applies to file headers.

| Metric | v5B | v6B | Delta | Notes |
|--------|:---:|:---:|:-----:|-------|
| Avg response chars | [AFTER RUNS] | [AFTER RUNS] | | #129: shorter path headers |
| Avg response lines | [AFTER RUNS] | [AFTER RUNS] | | |
| Path length (avg chars) | [AFTER RUNS] | [AFTER RUNS] | | Absolute vs relative |

## Symbol Focus Mode

Focus mode was the primary verbosity driver in v5: 17,028 chars / 604 lines (ours) vs
14,680 chars / 400 lines (native) for `from_path`. All 5 changes apply here.

| Metric | v5B | v6B | Delta | Notes |
|--------|:---:|:---:|:-----:|-------|
| Avg response chars | [AFTER RUNS] | [AFTER RUNS] | | All 5 changes combined |
| Avg response lines | [AFTER RUNS] | [AFTER RUNS] | | |
| Callee lines (total) | [AFTER RUNS] | [AFTER RUNS] | | #130 tree-indent, #134 dedup |
| Caller lines (total) | [AFTER RUNS] | [AFTER RUNS] | | #130 tree-indent, #132 test summary |
| Test caller lines | [AFTER RUNS] | [AFTER RUNS] | | #132: collapsed to count |
| Deduplicated chains | [AFTER RUNS] | [AFTER RUNS] | | #134: (xN) annotation |
| Header line | [AFTER RUNS] | [AFTER RUNS] | | #133: summary counts |

## Aggregate Response Sizes

Total tool response characters across all analyze calls in a session.

| Run | v5B chars | v6B chars | Delta (chars) | Delta (%) |
|-----|:---------:|:---------:|:-------------:|:---------:|
| B1 | [AFTER RUNS] | [AFTER RUNS] | | |
| B2 | [AFTER RUNS] | [AFTER RUNS] | | |
| B3 | [AFTER RUNS] | [AFTER RUNS] | | |
| B4 | [AFTER RUNS] | [AFTER RUNS] | | |
| B5 | [AFTER RUNS] | [AFTER RUNS] | | |
| **Median** | [AFTER RUNS] | [AFTER RUNS] | | |

## Compaction Delta Summary

| Dimension | v5B median | v6B median | Reduction (%) | Target |
|-----------|:----------:|:----------:|:-------------:|--------|
| **Overall tokens** | 31,620 | [AFTER RUNS] | | <10% gap (v6B vs v6A) |
| Focus response chars | [AFTER RUNS] | [AFTER RUNS] | | Largest contributor |
| File details response chars | [AFTER RUNS] | [AFTER RUNS] | | Moderate contributor |
| Overview response chars | [AFTER RUNS] | [AFTER RUNS] | | Minimal contributor |

## Gap Closure

| Metric | v5 | v6 | Status |
|--------|:--:|:--:|--------|
| B/A token ratio | 1.22 (22% overhead) | [AFTER RUNS] | Target: <1.10 |
| B median quality | 10 | [AFTER RUNS] | Target: >= 9 |
| B median total calls | 14 | [AFTER RUNS] | Expect: stable |

## Notes

- v5 baseline data from docs/benchmarks/v5/scores.json and v5/output-comparison.md
- Per-mode measurements extracted from session tool response content
- All 5 compaction changes are lossless: identical semantic information, reduced encoding
- v5 root cause: callee explosion in focus mode (528 flat lines vs 306 tree-indented for same info)
