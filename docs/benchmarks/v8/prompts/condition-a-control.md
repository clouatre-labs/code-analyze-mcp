# Condition A: Control (native Claude Code tools only)

YOUR GOAL: Analyze the sqlglot codebase and write a structured JSON report to OUTPUT_PATH. Everything below serves that goal. Do not end without writing the file.

## Repository

The sqlglot codebase is at: TARGET_REPO_PATH

## Required Tool Usage

You MUST use only native Claude Code tools for all analysis:

- **Glob** — discover files and directory structure
- **Grep** — search for patterns, imports, function definitions
- **Read** — read file content and specific line ranges
- **Bash** — run `rg` searches or inspect config files

Do NOT use `analyze_directory`, `analyze_file`, or `analyze_symbol`. These tools are not available in this condition.

**Turn budget:** You have approximately 30 turns total. Use at most 10 research tool calls, then write the report. Do not keep exploring — write the report with what you have.

## Task

Analyze the sqlglot codebase to answer:

1. **Module map:** Top-level modules and responsibilities. How `dialects/`, `optimizer/`, and `executor/` relate to the core `expressions.py`/`parser.py` pipeline.

2. **Pipeline trace:** Trace a SQL string through the full parsing pipeline:
   `SQL string → Tokenizer → Parser → AST (Expression tree) → Dialect generator → SQL output`
   Identify the key types passed between stages (Token, Expression subclass, etc.).

3. **Cross-module hubs:** Top 3 most-connected modules by cross-module imports. Explain why they are hubs.

4. **Change proposal:** Where to add a new scalar SQL function `LEVENSHTEIN(str1, str2)`. Files to modify, patterns to follow (e.g., how `SOUNDEX` or `EDITDISTANCE` are implemented), new types needed, dialect integration, risks.

## Output Schema

Write to OUTPUT_PATH:

```json
{
  "run_id": "RUN_ID",
  "condition": "A-control",
  "module_map": [
    {"module": "name", "responsibility": "...", "key_types": ["..."]}
  ],
  "pipeline_trace": [
    {"stage": "1. Tokenization", "module": "tokens.py", "key_function": "...", "types_produced": ["..."]}
  ],
  "cross_module_hubs": [
    {"module": "name", "inbound_deps": 0, "outbound_deps": 0, "reason": "..."}
  ],
  "change_proposal": {
    "files_to_modify": ["path"],
    "new_types": ["type description"],
    "pattern_to_follow": "...",
    "integration_point": "...",
    "risks": ["..."]
  },
  "tool_usage": [
    {"tool": "tool_name", "params": "brief description"}
  ]
}
```

## Reminder

Your deliverable is the JSON report file at OUTPUT_PATH. You must write it before you finish. Do not end without writing the report.
