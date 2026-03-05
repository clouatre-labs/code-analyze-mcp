# Benchmark Task: Cross-Module Research on lsd

## Target Repository

- **Repo:** lsd-rs/lsd (https://github.com/lsd-rs/lsd)
- **Size:** ~13K LOC, 52 Rust source files
- **Structure:** 4 module groups (flags/, meta/, theme/, root)
- **Selection rationale:** Mid-size Rust CLI with clear module boundaries, rich cross-module
  dependencies (display.rs imports from meta/, flags/, color, theme, icon), and a data pipeline
  pattern (collect metadata -> sort -> resolve icons/colors -> render). The project is well-known
  but unlikely to be in any session's prior context.

## Task Description

Analyze the lsd codebase to answer:

1. **Module map:** What are the top-level modules and their responsibilities? How do the
   submodules (flags/, meta/, theme/) relate to each other?

2. **Data flow:** Trace the path of a file entry from initial metadata collection (`Meta::from_path`
   or equivalent) through sorting, icon resolution, color resolution, and final display output.
   Identify the key types passed between modules at each stage.

3. **Cross-module dependencies:** Which modules have the most cross-module imports? Identify the
   top 3 most-connected modules and explain why they are hubs.

4. **Change proposal:** Propose where you would add a new display column showing file checksums
   (e.g., SHA-256 of file content). Identify:
   - Which files need modification
   - Which existing patterns to follow
   - What new types or structs are needed
   - How the new column integrates with the existing Block/display system
   - Potential risks or complications

## Deliverable Format

Produce a structured JSON report:

```json
{
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
    "pattern_to_follow": "description of existing pattern",
    "integration_point": "how it connects to Block/display",
    "risks": ["risk 1"]
  }
}
```
