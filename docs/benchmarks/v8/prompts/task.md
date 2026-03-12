# Benchmark Task: Cross-Module Research on sqlglot

## Target Repository

- **Repo:** tobymao/sqlglot (https://github.com/tobymao/sqlglot)
- **Language:** Python
- **Approx LOC:** ~70,000
- **Approx files:** ~100 Python files
- **Key modules:** `expressions.py`, `tokens.py`, `dialects/`, `parser.py`, `generator.py`, `optimizer/`, `planner.py`, `executor/`
- **Selection rationale:** Large-scale Python project with a deep AST hierarchy (Expression subclasses ~300+), a complex dialect system (50+ SQL dialects), and a multi-stage pipeline. The Expression hierarchy in a single file creates a challenging signal-to-noise problem for raw grep approaches.

## Task Description

Analyze the sqlglot codebase to answer:

1. **Module map:** What are the top-level modules and their responsibilities? How do `dialects/`, `optimizer/`, and `executor/` relate to the core `expressions.py`/`parser.py` pipeline?

2. **Pipeline trace:** Trace a SQL string through the full parsing pipeline:
   `SQL string → Tokenizer → Parser → AST (Expression tree) → Dialect generator → SQL output`
   Identify the key types passed between stages at each step (Token, Expression subclass, etc.).

3. **Cross-module hubs:** Which modules have the most cross-module imports? Identify the top 3 most-connected modules and explain why they are hubs.

4. **Change proposal:** Propose where to add a new scalar SQL function `LEVENSHTEIN(str1, str2)` returning edit distance. Identify:
   - Which files need modification (dialect base class, expression definition, generator methods)
   - Which existing patterns to follow (e.g., how `SOUNDEX` or `EDITDISTANCE` are implemented)
   - What new types or classes are needed (Expression subclass)
   - How the new function integrates with the dialect registration system
   - Potential risks (dialect compatibility, transpilation correctness, type inference)

## Deliverable Format

Produce a structured JSON report:

```json
{
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
  }
}
```
