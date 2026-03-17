<!-- TARGET REPO: django/django @ 6b90f8a8d6994dc62cd91dde911fe56ec3389494 -->
<!-- LOCAL PATH: /tmp/benchmark-repos/django -->

# Benchmark Task: Cross-Module Code Research

## Target Repository

**Repository:** django/django
**Commit:** `6b90f8a8d6994dc62cd91dde911fe56ec3389494`
**Local path:** `/tmp/benchmark-repos/django`
**Size:** ~510K LOC total (162K in core `django/` package), 902 Python source files
**Top-level packages:** `apps`, `conf`, `contrib`, `core`, `db`, `dispatch`, `forms`, `http`, `middleware`, `template`, `templatetags`, `test`, `urls`, `utils`, `views`

## Task Description

Analyze the codebase to answer:

1. **Module map:** What are the top-level modules and their responsibilities? How do subsidiary packages relate to the core pipeline or API layer? What are the key abstractions (types, classes, protocols)?

2. **Pipeline trace:** Trace a primary data flow through the system: HTTP request → URL routing → view dispatch → ORM query → response. Identify the key types/classes passed between modules at each stage. Document intermediate representations and data structures.

3. **Cross-module hubs:** Which top-level modules have the most cross-module imports or dependencies? Identify the top 3 most-connected modules and explain why they are architectural hubs.

4. **Change proposal:** Propose adding an async-compatible structured logging middleware that captures per-request metadata (view name, ORM query count, response time) and emits it as a structured log entry. Identify:
   - Which files and modules would require modification
   - Which existing patterns or conventions to follow (e.g., existing middleware classes)
   - What new types or classes might be needed
   - How the new feature integrates with the middleware stack
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
