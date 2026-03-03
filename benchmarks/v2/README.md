# Benchmark v2: code-analyze-mcp vs Manual Tooling

## Design

**Target repo:** [sharkdp/bat](https://github.com/sharkdp/bat) (~10.8K LOC Rust, 40 files, multi-module)

**Task:** Map the complete data flow from user input to terminal output in `bat`.
Identify all module boundaries crossed, the key types passed between modules,
and propose where you would add a new output format (e.g., HTML) with minimal changes.

This task requires:
1. Understanding the full module graph (structural accuracy)
2. Tracing Controller -> Printer -> Decorations -> Output chains (cross-module tracing)
3. Proposing an approach that respects existing patterns (approach quality)

**Conditions:**

| Run | Extensions | Prompt |
|-----|-----------|--------|
| A (control) | `developer` only (no `developer__analyze`) | `prompt-control.md` |
| B (treatment) | `developer` (no analyze) + `code-analyze-mcp` | `prompt-treatment.md` |

**Repetitions:** 3 runs per condition (6 total), Haiku, temperature 0.5

## Rubric (0-3 per dimension, 0-12 total)

| Dimension | 0 | 1 | 2 | 3 |
|-----------|---|---|---|---|
| Structural accuracy | Wrong module map | Partial | Correct but incomplete | Complete and correct |
| Cross-module tracing | Missed dependencies | Found some | Found most | Found all with context |
| Approach quality | Infeasible | Works but naive | Good with tradeoffs | Elegant, minimal, safe |
| Tool efficiency | >25 calls | 16-25 | 8-15 | <8 calls |

## Known Issues

- The treatment prompt references `developer__analyze` without naming `code-analyze-mcp`. The `developer__analyze` tool is the tool name exposed by the code-analyze-mcp MCP server. The prompts are preserved as-is because they are historical records of what was sent to the delegates during the benchmark runs. Future benchmarks should name the tool's source extension explicitly in the prompt instructions.

## Files

- `conditions.json` - Complete reproducibility metadata (see "How to reproduce" below)
- `prompt-control.md` - Control condition prompt (rg + cat only)
- `prompt-treatment.md` - Treatment condition prompt (code-analyze-mcp forced)
- `ground-truth.md` - Expected answers for scoring
- `results.md` - Raw results and analysis

## How to Reproduce

All metadata needed to reproduce this benchmark is in `conditions.json`:

- **Goose version:** 1.26.1
- **Orchestrator session:** 20260303_120 (see `~/.local/share/goose/sessions/sessions.db`)
- **Target repo:** sharkdp/bat at commit cc5f782d28a8e6156b8ebd3346b0a7f7c49256e2
- **Model:** claude-haiku-4-5@20251001 via gcp_vertex_ai, temperature 0.5
- **Conditions:** See `conditions.json` for exact extension configuration per run
- **Session IDs:** Each run (A1-A3, B1-B3) has a session ID in `conditions.json`

To reproduce:

1. Clone bat at the specified commit: `git clone https://github.com/sharkdp/bat && git checkout cc5f782d28a8e6156b8ebd3346b0a7f7c49256e2`
2. For each run in `conditions.json`, configure goose with the specified extensions
3. Use the corresponding prompt file (`prompt-control.md` or `prompt-treatment.md`)
4. Score the output against `ground-truth.md` using the rubric above
5. Compare results to `results.md`
