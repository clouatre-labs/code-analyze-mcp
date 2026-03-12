# Condition B: Treatment (native Claude Code tools + code-analyze-mcp)

YOUR GOAL: Analyze the sqlglot codebase and write a structured JSON report to OUTPUT_PATH. Everything below serves that goal. Do not end without writing the file.

## Repository

The sqlglot codebase is at: TARGET_REPO_PATH

## Required Tool Usage

You MUST use `analyze_directory`, `analyze_file`, and `analyze_symbol` as your primary exploration tools:

1. **Directory overview:** `analyze_directory` on the repo root (use `summary=true` for large output) to map top-level modules.
2. **File details:** `analyze_file` on key files (`expressions.py`, `tokens.py`, `parser.py`, `generator.py`) to extract functions, classes, and imports.
3. **Symbol tracing:** `analyze_symbol` with a function or class name to trace call graphs and cross-module dependencies.

You may also use native tools (Glob, Grep, Read, Bash) for targeted lookups the MCP tools do not cover — specific line ranges, config files, or pattern searches. Native tools are fallback, not primary.

**Turn budget:** You have approximately 30 turns total. Use at most 10 research tool calls total, then write the report. Do not keep exploring — write the report with what you have.

## MCP Tool Parameters

- `path` (required): directory or file to analyze
- `symbol` (optional): function or class name for call graph tracing (case-sensitive)
- `summary` (optional, bool): collapse verbose output to top-level summary — use for large files or directories
- `cursor` (optional, string): pass cursor from a previous paginated response to retrieve the next page
- `page_size` (optional, int): limit output size (default: 50000); reduce for shorter responses

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
  "condition": "B-treatment",
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
