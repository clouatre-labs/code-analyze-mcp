# Lightweight AB Test Protocol

## Purpose

Detect regressions or improvements in MCP tool quality for small models without running a full benchmark. Replaces multi-day blind-scored benchmarks for pre-commit regression checks.

## When to use

- Before merging changes to tool descriptions, server instructions, or output formatting
- After adding new tool parameters or error messages
- As a sanity check before scheduling a full benchmark (v11+)

## Protocol

### Setup

- Target repository: django/django pinned at 6b90f8a8d6994dc62cd91dde911fe56ec3389494 (same as v10)
- Task: same benchmark task from docs/benchmarks/v10/PROMPT.md
- Models: claude-haiku-4-5 (condition B), mistral-small-2603 (condition E), or minimax-m2.5 (condition D)
- DISABLE_PROMPT_CACHING=1 for all runs

### Conditions

| Condition | Tools | Runs |
|---|---|---|
| Treatment | MCP (this server, new version) | 3 |
| Control | MCP (previous version) OR native (Glob, Grep, Read) | 3 |

Total: 6 runs. Execute in randomized order.

### Proxy metrics (read from session output or metrics JSONL)

| Metric | Source | Proxy for |
|---|---|---|
| `research_calls` | session JSON (tool_calls_detail) | quality_score (Spearman r = -0.78, N=24) |
| `error_rate` | metrics JSONL (result='error' / total) | tool misuse rate |
| `first_tool_correct_rate` | session JSON (first tool call = analyze_directory) | approach_quality |
| `output_chars_total` | metrics JSONL sum per session | token pressure |

### Decision rule

- Effect threshold: delta >= 5 research_calls between treatment and control medians
- Require: treatment median research_calls <= control median
- Require: no increase in error_rate
- If both conditions met: change is safe to merge

### Proxy metric definitions

first_tool_correct_rate = first_tool is "analyze_directory".
Rate is computed as: (sessions where first_tool == "analyze_directory") / total sessions.
- If treatment research_calls increase > 2: flag for full benchmark before merge

### Computing first_tool_correct_rate from session metrics JSONL

After installing the new server version and running sessions, group events by session_id:

```bash
jq -s 'group_by(.session_id)[] | {session: .[0].session_id, first_tool: (sort_by(.seq) | .[0].tool)}' ~/.local/share/code-analyze-mcp/metrics-$(date +%Y-%m-%d).jsonl
```

first_tool_correct = first_tool is "analyze_directory".

### Time budget

- 6 runs x ~2 min each = 12 min compute
- Session capture + metrics extraction = 5 min
- Optional spot-check of one treatment vs one control output = 10-15 min
- **Total: 20-35 min**

### Limitations

- Spearman r = -0.78 at N=24; at N=3 per arm, only detects large effect sizes (delta >= 5 calls)
- Does not replace full benchmark for Wave boundary validation
- D/E (MiniMax, Mistral) failures are capability-driven; AB harness diagnoses but cannot fix
