# Condition A2: Haiku Native Control

## Your Goal

Analyze the target repository and write a structured JSON report to OUTPUT_PATH. Complete the task using only native Claude Code tools. Do not end without writing the report.

## Repository

The codebase is at: TARGET_REPO_PATH

## Required Tool Usage

You MUST use only native Claude Code tools:

- **Glob** — discover files and directory structure
- **Grep** — search for patterns, imports, function definitions
- **Read** — read file content and specific line ranges
- **Bash** — file exploration (rg, find, ls, cat for verification)

## Forbidden Tools

Do NOT use:
- `analyze_directory`
- `analyze_file`
- `analyze_symbol`

These MCP tools are not available in this condition.

## Turn Budget

You have approximately 30 turns total. Use at most 10 research tool calls total (Glob, Grep, Read, Bash for file exploration). Do not keep exploring — write the report with what you have.

## Task

See prompts/task.md for full task description.

## Output Schema

Write to OUTPUT_PATH:

```json
{
  "run_id": "RUN_ID",
  "condition": "A2-haiku-native-control",
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

**This is condition A2 (Haiku native control).** Model: claude-haiku-4-5
