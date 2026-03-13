# Tool Isolation in Goose Benchmarks

## Overview

Goose has no runtime mechanism to prevent an agent from calling a specific tool by name.
Extension loading flags (`--no-profile`, `--with-builtin`, `--with-extension`) control which
extensions are loaded at session start, but once loaded, an agent can call any tool from any
loaded extension.

Benchmark experiments that require controlled tool usage rely on three mitigation layers instead
of a hard runtime block. This document describes those layers, their limitations, and the
recovery procedure for detected isolation breaches.

The upstream fix for this limitation is tracked at [block/goose#7808].

## Naming Convention

Goose uses a double-underscore separator between extension name and tool name. The format
differs by context:

| Context | Format | Example |
|---------|--------|---------|
| Extension name (CLI flag) | kebab-case | `developer`, `code-analyze-mcp` |
| Builtin list (CLI flag) | comma-separated extension names | `developer,analyze` |
| Full tool name (as seen by agent) | `{extension}__{tool}` | `developer__analyze` |
| MCP tool name (as seen by agent) | `{extension}__{tool}` | `code-analyze-mcp__analyze` |
| Prompt instructions | full double-underscore name | `code-analyze-mcp__analyze` |
| validate.py checks | full double-underscore name | `developer__analyze` |
| Sessions DB (`content_json`) | full double-underscore name | `code-analyze-mcp__analyze` |

Key distinction: CLI flags use extension names only; prompts, tool calls, and session records use
the full `{extension}__{tool}` name. The extension name in the CLI flag must match the kebab-case
name registered in `--with-builtin` or `--with-extension`.

Native analyze (developer builtin) appears as `analyze` in the sessions DB when the extension
name equals the tool name (no prefix added).

## Mitigation Layers

### Layer 1: Extension Loading (System-Level)

Extension loading flags at session start determine which tools the agent can see:

```bash
# Condition A: native analyze builtin only
goose run --no-profile --with-builtin developer,analyze

# Condition B: developer builtin + code-analyze-mcp extension
goose run --no-profile --with-builtin developer --with-extension code-analyze-mcp
```

This prevents Condition A sessions from seeing `code-analyze-mcp__analyze`, and prevents
Condition B sessions from seeing the native `analyze` tool (it is not loaded).

Limitation: Extension loading is the only hard enforcement. It does not prevent calls to tools
within loaded extensions.

### Layer 2: Prompt Guidance (Behavioral Instruction)

Condition B prompts include an explicit instruction:

> Do NOT use `developer__analyze`. It is not available to you.

Condition A prompts include the symmetric instruction:

> Do NOT use `code-analyze-mcp__analyze`. It is not available to you.

Limitation: Soft enforcement. The agent may ignore this instruction if it hallucinates tool
availability or misreads the prompt.

### Layer 3: Retrospective Validation (Post-Run Audit)

After each session, `validate.py` queries the goose sessions database and checks the tool call
records against the expected condition:

**Condition A checks:**
- Native `analyze` tool was used (canonical name: `analyze` without `code-analyze-mcp` prefix).
- No `code-analyze-mcp` tool calls are present.

**Condition B checks:**
- `code-analyze-mcp__analyze` was used.
- Native `analyze` was NOT used.
- No `rg` calls with structural patterns (fn, struct, impl, mod, use) that bypass MCP analysis.

Validation can run per-session immediately after completion rather than waiting for all runs:

```bash
python3 scripts/validate.py --session-name v7-benchmark-R01 --condition B
```

Limitation: Retrospective, not preventive. A violation detected at this stage means the run
must be discarded and rerun.

## Current Behavior on Isolation Breach

If an agent calls a tool it should not use, the call succeeds. Goose does not reject it, log a
warning, or notify the operator. The breach is only visible in the sessions database after the
fact. `validate.py` detects it by scanning `content_json` for tool call records.

## Recovery Procedure

1. Run `validate.py` for the suspect session:
   ```bash
   python3 scripts/validate.py --session-name v7-benchmark-R06 --condition B
   # Output: "ERROR: native analyze used (forbidden for Condition B)"
   ```

2. Review the session transcript to understand the cause (prompt ambiguity, hallucination,
   or misconfigured extension flags).

3. If the prompt or configuration was incorrect, correct it before rerunning.

4. Rerun the session using the same session name:
   ```bash
   goose run --session-name v7-benchmark-R06 --no-profile --with-builtin developer \
       --with-extension code-analyze-mcp --input prompts/condition-b-treatment.md
   ```

5. Re-validate:
   ```bash
   python3 scripts/validate.py --session-name v7-benchmark-R06 --condition B
   ```

6. Resume the benchmark at the next run in `run-order.txt`.

## Required Benchmark Environment Variables

All benchmark runs must set the following environment variables to ensure symmetric conditions:

```bash
export DISABLE_PROMPT_CACHING=1
```

**Rationale:** Bedrock and Claude Code both enable prompt caching by default. In benchmark runs, caches are not reused across independent runs — cache_write overhead accumulates with limited cross-run cache_read benefit. Disabling caching ensures cost measurements reflect tool efficiency, not cache overhead.

All conditions must have `DISABLE_PROMPT_CACHING=1` set to avoid confounding the analysis with platform-specific caching behavior.

## Future Work

The upstream fix is tracked at [block/goose#7808]. The proposed change adds `blocked_tools` to
`ExtensionConfig`, symmetric to the existing `available_tools` allowlist, and exposes it in
recipe YAML:

```yaml
extensions:
  - type: builtin
    name: developer
    blocked_tools:
      - shell
      - write_file
```

Blocked calls would return a structured error visible to the agent rather than silently
succeeding. This would eliminate the need for prompt-level guidance and retrospective validation
for isolation enforcement.

## References

- Upstream issue: [block/goose#7808](https://github.com/block/goose/issues/7808)
- v6 benchmark: [docs/benchmarks/v6/](v6/)
- v7 benchmark: [docs/benchmarks/v7/](v7/)
- validate.py (v6): [docs/benchmarks/v6/scripts/validate.py](v6/scripts/validate.py)
- validate.py (v7): [docs/benchmarks/v7/scripts/validate.py](v7/scripts/validate.py)
- Related issues: #142 (v6 execution), #143 (v6 followup), #128 (naming confusion), #159 (this doc)
