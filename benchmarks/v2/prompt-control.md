# Research Task: bat Codebase Analysis (Control - Manual Tools Only)

You are a research agent analyzing the `sharkdp/bat` codebase. The repo is already cloned at `/tmp/bat_check`.

## Task

Map the complete data flow from user input to terminal output in `bat`. Your deliverable is a structured research report with three sections:

1. **Module map:** List every module in `src/` with its role and key types
2. **Data flow:** Trace the complete pipeline from CLI entry point to terminal output, identifying all module boundaries crossed and the key types passed between modules
3. **Extension proposal:** Propose where you would add a new output format (e.g., HTML) with minimal changes, identifying the specific abstraction points and files to modify

## Instructions

Follow these steps exactly:

**Step 1: Directory structure.** Use `rg --files src/ | head -50` and `wc -l src/*.rs src/**/*.rs` to understand the file layout and sizes.

**Step 2: Module roles.** Use `cat` or `head` to read `src/lib.rs` for the module declarations and re-exports. Then read the first 30 lines of each key module to understand its role.

**Step 3: Cross-module dependencies.** Use `rg '^use crate::' src/ --no-heading` to map all internal imports. Identify which modules depend on which.

**Step 4: Entry point tracing.** Read `src/bin/bat/main.rs` to find the entry point. Use `rg` to trace the call chain: find where `Controller` is created, where `run()` is called, and what it delegates to.

**Step 5: Printer system.** Use `rg 'trait Printer|impl Printer|struct.*Printer' src/` to find the printer abstraction. Read the trait definition and its implementations.

**Step 6: Decoration system.** Use `rg 'trait Decoration|impl Decoration' src/` to find the decoration system. Understand how it connects to the printer.

**Step 7: Output pipeline.** Use `rg 'OutputType|OutputHandle' src/ -l` to trace how output reaches the terminal.

**Step 8: Synthesize.** Write your structured report with all three sections.

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

- You may only use: `rg`, `cat`, `head`, `tail`, `wc`, `grep`, `sed`, `awk`, and the text_editor view command
- Do NOT use any structural analysis tools
- Count your tool calls and report the total
- Work in `/tmp/bat_check`
