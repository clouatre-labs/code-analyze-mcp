<!-- TARGET REPO: TBD -- update task description below before running -->

# Benchmark Task: Cross-Module Code Research

## Target Repository

Repository to be determined. Selection criteria:
- Large project (500,000+ lines of code)
- Deep module hierarchy with 5+ top-level packages
- Complex dependency graph
- Realistic code analysis scenarios (e.g., multi-dialect parser, ORM, framework)

Once selected, pin commit SHA in `run-order.txt` and update this section.

## Task Description

Analyze the codebase to answer:

1. **Module map:** What are the top-level modules and their responsibilities? How do subsidiary packages relate to the core pipeline or API layer? What are the key abstractions (types, classes, protocols)?

2. **Pipeline trace:** Trace a primary data flow through the system (e.g., user input → parsing → transformation → output). Identify the key types/classes passed between modules at each stage. Document intermediate representations and data structures.

3. **Cross-module hubs:** Which top-level modules have the most cross-module imports or dependencies? Identify the top 3 most-connected modules and explain why they are architectural hubs.

4. **Change proposal:** Propose where to add a new feature or capability. Identify:
   - Which files and modules would require modification
   - Which existing patterns or conventions to follow
   - What new types or classes might be needed
   - How the new feature integrates with the module system
   - Potential risks (maintainability, compatibility, performance)

## Deliverable Format

Produce a structured JSON report:

```json
{
  "module_map": [
    {
      "module": "module_name",
      "responsibility": "description of what this module does",
      "key_types": ["list", "of", "types", "or", "classes"],
      "depends_on": ["list", "of", "upstream", "modules"]
    }
  ],
  "pipeline_trace": [
    {
      "stage": "1. Input",
      "module": "module_name",
      "description": "what happens at this stage",
      "key_types": ["types", "produced", "or", "consumed"],
      "next_stage": "2. Processing"
    }
  ],
  "cross_module_hubs": [
    {
      "module": "module_name",
      "inbound_deps": 0,
      "outbound_deps": 0,
      "reason": "why this module is a hub"
    }
  ],
  "change_proposal": {
    "feature_summary": "brief description of feature to add",
    "files_to_modify": ["list", "of", "file", "paths"],
    "new_types": ["type", "descriptions", "if", "needed"],
    "pattern_to_follow": "description of existing pattern that applies",
    "integration_point": "where and how to integrate with existing modules",
    "risks": ["risk", "1", "risk", "2"]
  }
}
```

## Notes

- Focus on semantic understanding, not surface-level file listing.
- Identify data dependencies and flow, not just import statements.
- Be specific about which files and functions matter, not generic descriptions.
- Risks should be realistic for this codebase's design and maturity.
