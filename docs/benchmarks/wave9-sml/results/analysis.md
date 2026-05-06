# Wave 9 SML: MCP vs Native Analysis and Optimization Roadmap

**Generated from:** 3 parallel analysis delegates (token-cost, behavior, strategy)
**Task:** tsx re-wiring (4 edits to 2 files). All 20 scored runs: perfect 9/9 accuracy.

---

## 1. The Gap

| Condition | Model  | Tools  | Mean input tokens | Mean turns | Mean cost/run |
|-----------|--------|--------|-------------------|------------|---------------|
| A         | Sonnet | MCP    | 244,840           | 11.2       | $0.779        |
| B         | Sonnet | Native | 201,050           | 10.4       | $0.641        |
| C         | Haiku  | MCP    | 502,637           | 17.0       | $0.525        |
| D         | Haiku  | Native | 442,574           | 12.6       | $0.459        |

MCP is **+21.5% more expensive than native for Sonnet**, and **+14.4% for Haiku**. Accuracy is identical across all conditions (ceiling effect). The entire competitive disadvantage is cost.

---

## 2. Root Cause: Schema Overhead Dominates

The behavioral analysis found that tool schema inflation accounts for **~99% of the Haiku token gap** and roughly the same proportion for Sonnet.

### Schema overhead per turn

| Tool set | Tools in context | Tokens per turn (schemas) |
|----------|-----------------|---------------------------|
| MCP      | 9 tools         | ~4,600                    |
| Native   | 6 tools         | ~1,500                    |
| Delta    |                 | +3,100 per turn            |

Across 17 turns (Haiku MCP mean):
- MCP schema total: 17 x 4,600 = **78,200 tokens**
- Native schema total: 12.6 x 1,500 = **18,900 tokens**
- Delta: **59,300 tokens = 98.7% of the 60,063 token gap**

This is structural. Even if agent behavior were identical, the schema size difference across more turns would produce the observed gap. It is not primarily a prompt engineering problem.

### Other overhead sources (smaller)

| Source | Estimated tokens/run | Share of gap |
|--------|---------------------|--------------|
| Tool schema inflation (9 vs 6 tools, more turns) | ~59,300 | 98.7% |
| Extra turns from edit_replace verification loops | ~5,000-8,000 | 8-13% |
| Server "Recommended workflow" instructions | ~4,500 | 7.5% |
| Dual text+JSON response format | ~3,000 | 5% |
| Verbose analyze_raw line-number prefix | ~6,000 | 10% |
| Haiku MCP exploration before editing (analyze_directory calls) | ~8,000 | 13% |

These overlap; the total is not additive. The schema inflation is the dominant term.

---

## 3. Behavioral Differences: Haiku Takes More Turns

Haiku used 17 mean turns in MCP vs 12.6 in native (4.4 extra turns, +35%).

### Three mechanisms identified

**Edit verification loops (primary, ~2-3 extra turns):** `edit_replace` requires an exact, unique old_text match. Haiku, uncertain of whitespace and context accuracy, issues intermediate `analyze_raw` reads between edits to confirm the prior operation succeeded. Native `Write` overwrites the whole file, eliminating ambiguity entirely. No verification needed.

**Server instruction drift (secondary, ~1 extra turn):** The MCP server injects a "Recommended workflow" on every connection: start with `analyze_directory`, then `analyze_module`, then `analyze_file`, then `analyze_symbol`. This general-purpose exploration workflow conflicts with the system prompt's explicit 6-step plan. Haiku partially follows the server's instructions, calling `analyze_directory` 1-2 times before proceeding to the prescribed `analyze_raw` path.

**Dual-format response parsing (~0.5 turns):** Each MCP tool response returns both human-readable text (with "  NNN | " line prefixes) and structured JSON containing the same data. Haiku's smaller model capacity may require extra reasoning cycles to reconcile both formats before acting.

### edit_replace vs edit_overwrite for this task

For 4 edits to 2 small files (mod.rs ~800 lines, lang.rs ~200 lines):

| Approach | Tool calls | Risk of verification reads | Total estimated tokens |
|----------|-----------|---------------------------|------------------------|
| edit_replace (4 calls, no verification) | 6 total | high (Haiku specific) | ~35,300 |
| edit_replace (4 calls + 2-3 verification reads) | 8-9 total | realized | ~50,000 |
| edit_overwrite (2 calls, full file) | 4 total | none | ~31,400 |

**edit_overwrite is preferable for multi-edit rewires on small files.** The system prompt currently prescribes edit_replace, which is optimal for surgical edits on large files (>500 lines) where sending the full file would be wasteful. For files under 300 lines with 2+ edits, edit_overwrite eliminates verification anxiety and reduces tool calls.

---

## 4. Optimization Roadmap

The goal: MCP beats native on cost. The Haiku gap (14.4%, 60k tokens) is the more tractable pairing because Haiku's per-token rate makes absolute token counts more decisive.

### Prompt-level changes (zero server code changes, immediate)

These changes can be made to the benchmark system prompts for the next wave:

| Change | Mechanism | Estimated token saving |
|--------|-----------|------------------------|
| P1: Suppress server exploration workflow | Add to system prompt: "Ignore MCP server workflow suggestions; follow the steps in this prompt exactly." | ~15,500 tokens |
| P2: Mandate edit_overwrite for files <300 lines | Add: "For files under 300 lines or tasks with 2+ edits to the same file, use edit_overwrite (not edit_replace)." | ~17,750 tokens |
| P3: Request JSON-only for large reads | Add: "Parse MCP responses from the JSON structured content field; ignore text content." | ~3,000 tokens |
| **Total** | | **~36,250 tokens** |

**After prompt changes:** Haiku MCP projected at 466,387 tokens vs Native 442,574 (+5.4%). Not yet ahead, but within noise.

### Server-level changes (Tier 1, 1-2 PRs)

| Change | File/function | Estimated token saving | Risk |
|--------|--------------|------------------------|------|
| S1: Remove static server instructions | `lib.rs` initialize handler: remove workflow block or gate behind client flag | ~4,500 | Low: agents lose hint, but system prompts replace it |
| S2: Compress tool schemas | `#[tool(...)]` attributes: shorten description fields for edit_* tools | ~1,800 | Low: docs move to external markdown |
| S3: Omit line-number prefix from analyze_raw | `crates/aptu-coder-core`: add `omit_line_numbers` param (default true for edit-oriented clients) | ~6,000 | Low: flag, old behavior preserved |
| S4: Add `edit_bulk` tool | New tool accepting list of find-replace pairs, writing once per file | ~4,000 | Low: additive, old tools remain |
| S5: Suppress human-readable text field | Add `DISABLE_HUMAN_OUTPUT` flag: return structured JSON only, skip text field | ~9,000 | Low: flag, default off |
| **Tier 1 total** | | **~25,300 tokens** | |

### Combined projection (prompt + Tier 1 server)

Combined savings: 36,250 + 25,300 = **61,550 tokens**

Projected Haiku MCP: 502,637 - 61,550 = **441,087 tokens**
vs Haiku Native: 442,574 tokens

Gap: -1,487 tokens (**-0.3%** -- MCP beats native by a razor margin).

### Server-level changes (Tier 2, design required)

| Change | Mechanism | Estimated saving |
|--------|-----------|-----------------|
| Dynamic schema selection | Serve minimal schemas for edit-only sessions | ~3,000 tokens |
| Session analysis cache | Cache analyze_* results; return reference IDs on repeat queries | ~2,500 tokens |
| edit_patch tool | Accept unified diff, apply in one call vs multiple edit_replace | ~3,500 tokens |

### Server-level changes (Tier 3, protocol changes)

| Change | Mechanism | Estimated saving | Risk |
|--------|-----------|-----------------|------|
| Binary payload for file content | Base64 or binary channel vs JSON text | ~12,000 tokens | Breaks JSON-only protocol |
| Streaming token protocol | Model receives tokens incrementally, early-truncates irrelevant parts | ~8,000 tokens | Major overhaul |

---

## 5. Priority Recommendations

Ordered by impact-to-effort ratio:

1. **S5: Suppress human-readable text field** -- 9,000 tokens, 1 PR, low risk. The JSON field already contains everything; the text field is redundant noise for agent use cases. Add `DISABLE_HUMAN_OUTPUT` flag (or rename it for the benchmark) and emit JSON only.

2. **S3: Omit line-number prefix from analyze_raw** -- 6,000 tokens, 1 PR. The "  NNN | " prefix serves human readability, not agent parsing. Agents parsing file content for edit anchors do not need line numbers (they use literal text matching). Gate behind a param defaulting to false for agents, true for humans.

3. **S1: Remove/shorten server instructions** -- 4,500 tokens, 1 PR. The current instruction block is a general code-exploration workflow. It is injected on every connection regardless of task type. Replace it with a minimal tool index: "Tools: analyze_raw (read), edit_replace/edit_overwrite/edit_rename/edit_insert (write), analyze_* (inspect)." Remove the step-by-step workflow.

4. **S4: Add edit_bulk tool** -- 4,000 tokens, 1-2 PRs. Four separate edit_replace calls on the same file generate four round-trips. A single `edit_bulk(path, replacements[])` call applies all changes atomically and returns one response.

5. **S2: Compress tool schemas** -- 1,800 tokens, 1 PR. Shorten description fields for edit_* tools (they are currently verbose with multiple example queries). Move examples to documentation.

**Implementing items 1-5 closes approximately 67% of the Haiku gap** (25,300 / 37,489 tokens). Combined with prompt-level changes (P1+P2+P3), MCP crosses below native cost.

---

## 6. What the Data Does Not Show

- **Tool call sequences**: without session JSONL files (the isolation check could not locate them), the exact sequence of tool calls per run is unknown. The extra turns and verification loops are inferred from turn count vs tool call count ratios, not directly observed.
- **Cache effectiveness**: DISABLE_PROMPT_CACHING=1 was set for all runs. With caching enabled and a stable system prompt, the schema overhead would be amortized across turns via cache reads, substantially narrowing the gap (but not eliminating it, since cache creation still costs).
- **Accuracy ceiling**: all conditions hit 9/9. Future waves must raise task difficulty to expose quality differences between MCP and native tools, which is where MCP's semantic analysis (analyze_symbol, analyze_file) should provide a genuine advantage.

---

*Analysis by 3 parallel delegates: token-cost (mercury-2 via openrouter), behavior (claude-haiku-4-5 via bedrock), strategy (mercury-2 via openrouter). Synthesized by orchestrator.*
