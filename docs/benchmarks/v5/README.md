# Benchmark v5: Tool Isolation Experiment

## Result

**code-analyze-mcp achieves parity with native analyze when tool isolation is enforced.**

The v3 benchmark found code-analyze-mcp produced equivalent quality but cost 29% more tokens
and 31% more wall time. v5 adds an rg-blocking constraint that prevents the agent from
supplementing code-analyze-mcp with structural shell commands. This eliminates the efficiency
gap on tool calls (14 vs 14, previously 18 vs 14) and wall time (73s vs 70s, previously 80s
vs 61s).

Token overhead persists (22%) due to code-analyze-mcp's more verbose output format.

## Key Numbers

| Metric | A (native) | B (code-analyze-mcp) | v3B | Change from v3 |
|--------|:----------:|:--------------------:|:---:|:--------------:|
| Quality (0-12) | 10 | 10 | 10 | Same |
| Total calls | 15 | 14 | 18 | -22% |
| Shell calls | 4 | 1 | 6 | -83% |
| Tokens | 25,818 | 31,620 | 31,005 | Same |
| Wall time (s) | 70 | 73 | 80 | -9% |

## What Changed from v3

1. **rg-blocking constraint** in Condition B prompt: "Do NOT use rg or cat to understand code
   structure. Use code-analyze-mcp__analyze exclusively."
2. **System-level isolation** via `--no-profile --with-builtin developer --with-extension code-analyze-mcp`
   ensuring the native analyze extension is unavailable, not just discouraged.
3. **Updated tool_efficiency rubric**: 3 = 5 or fewer calls, 2 = 6-10, 1 = 11-20, 0 = 20+
   (v3 had a ceiling effect where all runs scored 3/3).

## Conclusion

The v3 finding that "code-analyze-mcp adds no value" was an artifact of tool redundancy, not
tool quality. When the agent cannot fall back to rg for structural queries, it uses
code-analyze-mcp more effectively and achieves the same total call count as native analyze.

**Recommendation:** Deploy code-analyze-mcp with tool isolation constraints for structural
analysis tasks. Address the 22% token overhead by reducing output verbosity.

## Files

- [analysis.md](analysis.md) -- full statistical analysis and cross-version comparisons
- [methodology.md](methodology.md) -- experiment design and decision framework
- [scores.json](scores.json) -- complete scoring data
- `results/runs/` -- 10 raw run outputs
- `scripts/` -- analysis, collection, and validation tooling
