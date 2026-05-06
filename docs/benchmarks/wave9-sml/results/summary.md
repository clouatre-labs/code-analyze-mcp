# Wave 9 SML Benchmark Results

**Task:** TSX registry re-wiring (restore tsx language support across `mod.rs` and `lang.rs`)
**Date:** 2026-05-05
**Runs:** 24 total (4 pilots + 20 scored; 5 per condition)
**Models:** Conditions A/B = Sonnet 4.6; Conditions C/D = Haiku 4.5
**Tools:** Conditions A/C = MCP (aptu-coder tools); Conditions B/D = native (Bash/Grep/Read/Write)

---

## Scored Runs (n=5 per condition)

| Condition | Model  | Tools  | Mean Score (/9) | Mean Input Tokens | Mean Output Tokens | Mean Cost (USD) | Total Cost (USD) |
|-----------|--------|--------|-----------------|-------------------|--------------------|-----------------|-----------------|
| A         | Sonnet | MCP    | **9.00**        | 244,840           | 2,969              | $0.779          | $3.895          |
| B         | Sonnet | Native | **9.00**        | 201,050           | 2,546              | $0.641          | $3.207          |
| C         | Haiku  | MCP    | **9.00**        | 502,637           | 4,434              | $0.525          | $2.624          |
| D         | Haiku  | Native | **9.00**        | 442,574           | 3,272              | $0.459          | $2.295          |

## Per-Run Scores

| Run ID       | Condition | Pilot | NS | EXT | SF | Total |
|--------------|-----------|-------|----|-----|----|-------|
| A-pilot      | A         | yes   | 3  | 3   | 3  | 9     |
| B-pilot      | B         | yes   | 3  | 3   | 3  | 9     |
| C-pilot      | C         | yes   | 3  | 3   | 2  | 8     |
| D-pilot      | D         | yes   | 3  | 3   | 3  | 9     |
| A-scored-1   | A         | no    | 3  | 3   | 3  | 9     |
| A-scored-2   | A         | no    | 3  | 3   | 3  | 9     |
| A-scored-3   | A         | no    | 3  | 3   | 3  | 9     |
| A-scored-4   | A         | no    | 3  | 3   | 3  | 9     |
| A-scored-5   | A         | no    | 3  | 3   | 3  | 9     |
| B-scored-1   | B         | no    | 3  | 3   | 3  | 9     |
| B-scored-2   | B         | no    | 3  | 3   | 3  | 9     |
| B-scored-3   | B         | no    | 3  | 3   | 3  | 9     |
| B-scored-4   | B         | no    | 3  | 3   | 3  | 9     |
| B-scored-5   | B         | no    | 3  | 3   | 3  | 9     |
| C-scored-1   | C         | no    | 3  | 3   | 3  | 9     |
| C-scored-2   | C         | no    | 3  | 3   | 3  | 9     |
| C-scored-3   | C         | no    | 3  | 3   | 3  | 9     |
| C-scored-4   | C         | no    | 3  | 3   | 3  | 9     |
| C-scored-5   | C         | no    | 3  | 3   | 3  | 9     |
| D-scored-1   | D         | no    | 3  | 3   | 3  | 9     |
| D-scored-2   | D         | no    | 3  | 3   | 3  | 9     |
| D-scored-3   | D         | no    | 3  | 3   | 3  | 9     |
| D-scored-4   | D         | no    | 3  | 3   | 3  | 9     |
| D-scored-5   | D         | no    | 3  | 3   | 3  | 9     |

*NS = namespace_correctness, EXT = extension_registration, SF = structural_fidelity*

## Rubric Scores Key

- **namespace_correctness (0-3):** 3 = all 5 grep checks pass with correct `typescript::` namespace
- **extension_registration (0-3):** 3 = both EXTENSION_MAP and supported_languages entries present and correct
- **structural_fidelity (0-3):** 3 = all arms present, correct structure; 2 = all arms present, minor issues (e.g. extraneous edits)

## Pilot Notes

C-pilot scored 8/9 (structural_fidelity=2): the Haiku+MCP agent wrote 5 tree-sitter queries
beyond the task scope (task required re-wiring only, not new query authorship). All other pilots
and all 20 scored runs achieved perfect 9/9.

## Interpretation

All four conditions achieved a perfect mean score of 9/9 on the scored runs, indicating that the
tsx re-wiring task is fully solved by both Sonnet and Haiku regardless of tool set (MCP vs. native
Bash/Grep/Read/Write). The primary differentiator across conditions is cost: Haiku conditions (C/D)
cost 33-41% less per run than Sonnet (A/B), with Haiku+native (D) being the cheapest at $0.459/run.
MCP tool conditions consumed significantly more input tokens than native (C: 503k vs. D: 443k;
A: 245k vs. B: 201k), likely because the MCP server returns structured, verbose JSON payloads per
call; this inflated token cost partially offsets the per-token price advantage of Haiku. Given
ceiling-level accuracy across all conditions, this task does not discriminate between tool sets or
model tiers at the accuracy dimension; future waves should increase task difficulty to expose
capability gaps.
