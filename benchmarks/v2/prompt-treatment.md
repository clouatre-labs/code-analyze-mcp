# Research Task: bat Codebase Analysis (Treatment - code-analyze-mcp)

You are a research agent analyzing the `sharkdp/bat` codebase. The repo is already cloned at `/tmp/bat_check`.

## Task

Map the complete data flow from user input to terminal output in `bat`. Your deliverable is a structured research report with three sections:

1. **Module map:** List every module in `src/` with its role and key types
2. **Data flow:** Trace the complete pipeline from CLI entry point to terminal output, identifying all module boundaries crossed and the key types passed between modules
3. **Extension proposal:** Propose where you would add a new output format (e.g., HTML) with minimal changes, identifying the specific abstraction points and files to modify

## Instructions

Follow these steps exactly:

**Step 1: Directory overview.** Use the `developer__analyze` tool with path `/tmp/bat_check/src` and max_depth 3 to get the complete directory structure with file metrics (LOC, function counts, class counts).

**Step 2: Module roles.** Use the `developer__analyze` tool on each key file (e.g., `/tmp/bat_check/src/controller.rs`, `/tmp/bat_check/src/printer.rs`) to get function lists, imports, and type definitions.

**Step 3: Cross-module tracing.** Use the `developer__analyze` tool with the `focus` parameter set to `Printer` on path `/tmp/bat_check/src` to trace the call graph for the Printer type across the codebase. Then do the same for `Controller`.

**Step 4: Decoration system.** Use the `developer__analyze` tool with `focus` set to `Decoration` on path `/tmp/bat_check/src` to trace the decoration call graph.

**Step 5: Output pipeline.** Use the `developer__analyze` tool with `focus` set to `OutputHandle` on path `/tmp/bat_check/src` to trace how output reaches the terminal.

**Step 6: Entry point.** Use the `developer__analyze` tool on `/tmp/bat_check/src/bin/bat/main.rs` to understand the CLI entry point and its dependencies.

**Step 7: Synthesize.** Write your structured report with all three sections.

## Output Format

Write your report as structured JSON:

```json
{
  "module_map": [
    {"module": "controller.rs", "role": "...", "key_types": ["Controller"]}
  ],
  "data_flow": {
    "entry_point": "description",
    "pipeline_steps": [
      {"step": 1, "module": "...", "action": "...", "types_passed": ["..."]}
    ],
    "key_boundary_crossings": [
      {"from": "module_a", "to": "module_b", "types": ["Type1"], "description": "..."}
    ]
  },
  "extension_proposal": {
    "approach": "description",
    "abstraction_point": "where to extend",
    "files_to_modify": ["file1.rs"],
    "rationale": "why this is minimal"
  },
  "tool_calls_used": 0
}
```

## Constraints

- You MUST use the `developer__analyze` tool for all structural queries (steps 1-6)
- You may use `rg` or `cat` only to read specific file content when analyze output is insufficient
- Count your tool calls and report the total
- Work in `/tmp/bat_check`
