# Benchmark v3: Tool Isolation and Effectiveness

## Experiment Design

This benchmark compares two conditions for analyzing code structure:
- **Condition A (Control):** Uses `developer__analyze` (built-in goose tool)
- **Condition B (Treatment):** Uses `analyze` from code-analyze-mcp (MCP server)

Both conditions perform the same task on the same codebase (bat repository) and are scored against identical ground truth.

## Methodology

### Task
Map the complete data flow from user input to terminal output in the bat repository.

### Model and Parameters
- Model: Claude Haiku 4.5 (`claude-haiku-4-5@20251001`)
- Temperature: 0.5
- Session isolation: Each run in a separate goose session
- Sample size: n=5 per condition (10 total runs)

### Conditions

#### Condition A: developer__analyze (Control)
- Uses goose's built-in `developer__analyze` tool for structural queries
- Queries directory overview, file details, and symbol focus
- No access to code-analyze-mcp

#### Condition B: code-analyze-mcp (Treatment)
- Uses code-analyze-mcp's `analyze` tool for structural queries
- Same query patterns as Condition A but via MCP server
- Requires code-analyze-mcp to be registered as a goose extension

### Execution Procedure

1. Generate randomized run order from `run-order.json`
2. For each run:
   - Create a new goose session
   - Load the appropriate prompt (control or treatment)
   - Execute the prompt with the delegate model
   - Record: session_id, tool_calls, token counts, wall time
   - Store output in `results/run-{id}.json`
3. Blind the results: remove condition labels, randomize order
4. Score each run against ground truth using the rubric
5. Perform Mann-Whitney U test on blinded scores
6. Document results in `analysis.md`

### Scoring

- **Rubric:** Four dimensions (structural_accuracy, cross_module_tracing, approach_quality, tool_efficiency)
- **Scale:** 0-3 per dimension
- **Blinding:** Runs are scored without knowing which condition they belong to
- **Procedure:** See `rubric.md` for detailed blinding steps

## Differences from v2

1. **Fresh prompts:** Both prompts are newly written (not copied from PR #65)
2. **Tool isolation:** Condition A explicitly uses developer__analyze; Condition B uses code-analyze-mcp
3. **Blinding procedure:** Documented explicitly in rubric.md
4. **Run order:** Pre-registered and deterministic (seed-based randomization)
5. **Artifact structure:** Separate prompts/, results/, and analysis files

## Reproducibility

### Prerequisites
- goose CLI installed and configured
- code-analyze-mcp registered as a goose extension (for Condition B)
- bat repository cloned at commit `cc5f782d28a8e6156b8ebd3346b0a7f7c49256e2` (see ground-truth.md for details)

### Tool Isolation Configuration

**Condition A (Control):** Uses `developer__analyze` (built-in goose tool)
- Standard goose configuration with developer extension enabled
- Command: `goose prompt --session "benchmark-v3-{run_id}" < prompts/prompt-control.md`

**Condition B (Treatment):** Uses `analyze` from code-analyze-mcp
- Requires code-analyze-mcp registered as a goose extension
- Must disable or exclude developer__analyze to ensure tool isolation
- **Known limitation:** goose CLI does not provide a built-in flag to disable a specific tool within an extension
- **Workaround options:**
  1. Use separate goose profiles: create a profile with only code-analyze-mcp enabled, no developer extension
  2. Use `--no-profile` with explicit `--with-extension code-analyze-mcp` (if supported by goose version)
  3. Verify tool isolation by checking session logs: `sqlite3 ~/.local/share/goose/sessions/sessions.db "SELECT * FROM messages WHERE session_id = '{session_id}' AND content_json LIKE '%developer__analyze%'"`
- Command: `goose prompt --session "benchmark-v3-{run_id}" < prompts/prompt-treatment.md`

### Execution Command

```bash
# For each run in run-order.json:
goose session create --name "benchmark-v3-{run_id}" --working-dir /path/to/bat

# Condition A (Control):
goose prompt --session "benchmark-v3-{run_id}" < prompts/prompt-control.md

# Condition B (Treatment) - with tool isolation:
# Use separate profile or verify isolation via session logs (see above)
goose prompt --session "benchmark-v3-{run_id}" < prompts/prompt-treatment.md
```

### Collecting Results

After each run:
1. Extract session metadata: `goose session list --json | jq '.[] | select(.name == "benchmark-v3-{run_id}")'`
2. Query session database for tool calls and tokens: `sqlite3 ~/.local/share/goose/sessions/sessions.db`
3. Record in `conditions.json` template

## File Manifest

- `README.md` - This file
- `prompts/prompt-control.md` - Condition A prompt (developer__analyze)
- `prompts/prompt-treatment.md` - Condition B prompt (code-analyze-mcp)
- `ground-truth.md` - Expected answers and scoring reference
- `rubric.md` - Scoring dimensions, scale, and blinding procedure
- `run-order.json` - Pre-registered randomized run order (A1-A5, B1-B5)
- `conditions.json` - Template for recording run metadata
- `scores.json` - Template for blinded scoring results
- `analysis.md` - Template for statistical analysis
- `results/` - Directory for storing run outputs (run-{id}.json)

## Notes

- Single scorer (no inter-rater reliability)
- Small sample size (n=5) may limit statistical power
- Temperature 0.5 introduces variability; results are not deterministic
- Tool isolation depends on goose's ability to disable developer__analyze for Condition B
