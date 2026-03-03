# Benchmarks

This directory contains reproducible benchmarks comparing code-analyze-mcp to manual tooling approaches.

## Structure

- `v1/` - Original proof-of-concept benchmark (flawed, preserved for reference)
- `v2/` - Properly designed benchmark with 3 repetitions per condition

## Conventions

### Reproducibility

Each benchmark version includes a `conditions.json` file with complete metadata:

- Goose version and orchestrator session ID
- Target repository, commit SHA, and local path
- Model name, provider, and temperature
- Exact extension configuration per run
- Session IDs for each run (queryable in `~/.local/share/goose/sessions/sessions.db`)
- Token counts, wall time, tool call counts
- Scores per dimension and total

This allows exact reproduction of any run.

### Prompts

Prompts are immutable after runs complete. Document issues in the benchmark's README, not by editing prompts.

### Scoring

All benchmarks use a consistent rubric with four dimensions, each scored 0-3:

1. **Structural accuracy** - Correctness and completeness of module/type discovery
2. **Cross-module tracing** - Ability to trace dependencies and data flow across module boundaries
3. **Approach quality** - Quality of proposed solutions (elegance, minimalism, safety)
4. **Tool efficiency** - Number of tool calls used (lower is better)

Total score is the sum of all four dimensions (0-12 scale).

### Prompts

Each benchmark includes two prompt files:

- `prompt-control.md` - Control condition (manual tools only: rg, cat, text_editor)
- `prompt-treatment.md` - Treatment condition (with code-analyze-mcp)

Both prompts define the same task and output format to ensure fair comparison.

### Ground Truth

Each benchmark includes `ground-truth.md` with:

- Expected answers for each dimension
- Scoring rubric with examples
- Rationale for each score

### Results

Each benchmark includes `results.md` with:

- Raw scores for each run
- Per-dimension means and differences
- Tool call counts and efficiency analysis
- Key findings and limitations

## How to Use

1. **To understand a benchmark:** Start with the version's README.md
2. **To reproduce a run:** See the "How to reproduce" section in the version's README.md
3. **To compare conditions:** See the "Summary Statistics" section in results.md
4. **To understand scoring:** See the rubric in the version's README.md and ground-truth.md

## Versions

### v1 (Flawed)

- Single run per condition (no statistical power)
- Proof-of-concept only
- See `v1/README.md` for why it was flawed

### v2 (Current)

- 3 runs per condition (6 total)
- Proper statistical design
- Complete reproducibility metadata
- See `v2/README.md` for design and results
