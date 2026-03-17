# Condition D: Treatment (MiniMax M2.5 + MCP)

## Your Goal

Analyze the target repository and write a structured JSON report to OUTPUT_PATH. Use MCP semantic tools exclusively for file exploration. Do not end without writing the report.

## Repository

The codebase is at: TARGET_REPO_PATH

## Required Tool Usage

You MUST use `analyze_directory`, `analyze_file`, and `analyze_symbol` as your only exploration tools:

1. **Directory overview:** `analyze_directory` on the repo root (use `summary=true` for large output) to map top-level modules.
2. **File details:** `analyze_file` on key files to extract functions, classes, and imports.
3. **Symbol tracing:** `analyze_symbol` to trace call graphs and cross-module dependencies.

## Forbidden Tools

Do NOT use:
- `Glob` — file discovery by pattern
- `Grep` — text search
- `Read` — file content reading
- `Bash` — shell commands for file exploration (cat, rg, find, head, tail, sed, awk)

MCP tools are your only research tools in this condition. Do not use Bash at all; the runner disallows it entirely for this condition.

## Turn Budget

You have approximately 30 turns total. Use at most 10 research tool calls total (all MCP: analyze_directory, analyze_file, analyze_symbol). Do not keep exploring — write the report with what you have.

## MCP Tool Parameters

- `path` (required): directory or file to analyze
- `symbol` (optional): function or class name for call graph tracing (case-sensitive)
- `summary` (optional, bool): collapse verbose output to top-level summary — use for large files or directories
- `cursor` (optional, string): pass cursor from a previous paginated response to retrieve the next page
- `page_size` (optional, int): limit output size (default: 50000); reduce for shorter responses

## Task

See prompts/task.md for full task description.

## Output Schema

Write to OUTPUT_PATH:

```json
{
  "run_id": "RUN_ID",
  "condition": "D-treatment",
  "module_map": [...],
  "pipeline_trace": [...],
  "cross_module_hubs": [...],
  "change_proposal": {...},
  "tool_usage": [
    {"tool": "tool_name", "params": "brief description"}
  ]
}
```

## Reminder

Your deliverable is the JSON report file at OUTPUT_PATH. You must write it before finishing.

---

**This is condition D (MiniMax M2.5 + MCP via OpenRouter).** Model: minimax/minimax-m2.5 (provider: openrouter)
