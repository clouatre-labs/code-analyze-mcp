# Condition B: Treatment (code-analyze-mcp, v4 post-optimization)

YOUR GOAL: Analyze the lsd codebase and write a structured JSON report to OUTPUT_PATH. Everything below serves that goal. Do not end without writing the file.

## Repository

The lsd codebase is at: TARGET_REPO_PATH

## Required Tool Usage

You MUST use the `code-analyze-mcp__analyze` tool for structural analysis:

1. **Directory overview:** `code-analyze-mcp__analyze` on `src/` to get the file tree with function/class counts.
2. **File details:** `code-analyze-mcp__analyze` on key files (display.rs, core.rs, meta/mod.rs) for functions, imports, types.
3. **Cross-module tracing:** `code-analyze-mcp__analyze` with `focus` parameter on key functions (from_path, display_grid) for call chains.

You may also use `developer__shell` for `rg` searches and `developer__text_editor` to read files, but `code-analyze-mcp__analyze` must be your primary structural analysis tool.

Do NOT use `developer__analyze`. It is not available to you.

**Turn budget:** You have approximately 30 turns total. Use at most 10 analysis/research tool calls, then write the report. Do not keep exploring -- write the report with what you have.

## Task

Analyze the lsd codebase to answer:

1. **Module map:** Top-level modules and responsibilities. How submodules (flags/, meta/, theme/) relate.
2. **Data flow:** Trace a file entry from metadata collection (Meta::from_path) through sorting, icon resolution, color resolution, to display output. Key types passed between modules at each stage.
3. **Cross-module dependencies:** Top 3 most-connected modules by cross-module imports. Why they are hubs.
4. **Change proposal:** Where to add a file checksum display column (SHA-256). Files to modify, patterns to follow, new types needed, Block/display integration, risks.

## Output Schema

Write to OUTPUT_PATH using `developer__text_editor` write command:

```json
{
  "run_id": "RUN_ID",
  "condition": "B-treatment",
  "module_map": [
    {"module": "name", "responsibility": "...", "submodules": ["..."], "key_types": ["..."]}
  ],
  "data_flow": [
    {"stage": "1. Metadata collection", "module": "meta/", "key_function": "...", "types_produced": ["..."]}
  ],
  "cross_module_hubs": [
    {"module": "name", "inbound_deps": 0, "outbound_deps": 0, "reason": "..."}
  ],
  "change_proposal": {
    "files_to_modify": ["path"],
    "new_types": ["type description"],
    "pattern_to_follow": "existing pattern description",
    "integration_point": "how it connects to Block/display",
    "risks": ["risk 1"]
  },
  "tool_usage": [
    {"tool": "tool_name", "params": "brief description"}
  ]
}
```

## Reminder

Your deliverable is the JSON report file at OUTPUT_PATH. You must write it before you finish. Use `developer__text_editor` write command to create the file. Do not end without writing the report.
