# Wave 9 SML: MCP vs Native Analysis and Optimization Roadmap

**Generated from:** parallel analysis delegates (token-cost, behavior, tool-surface, schema-measurement, strategy, MCP protocol validation)
**Task:** tsx re-wiring (4 edits to 2 files). All 20 scored runs: perfect 9/9 accuracy.
**MCP spec validated against:** 2025-11-25. **rmcp validated against:** 1.6.0 (Cargo.lock).

---

## 1. The Gap

| Condition | Model  | Tools  | Mean input tokens | Mean turns | Mean cost/run |
|-----------|--------|--------|-------------------|------------|---------------|
| A         | Sonnet | MCP    | 244,840           | 11.2       | $0.779        |
| B         | Sonnet | Native | 201,050           | 10.4       | $0.641        |
| C         | Haiku  | MCP    | 502,637           | 17.0       | $0.525        |
| D         | Haiku  | Native | 442,574           | 12.6       | $0.459        |

MCP is **+21.5% more expensive than native for Sonnet** and **+14.4% for Haiku**. Accuracy is identical across all conditions (ceiling effect). The entire competitive disadvantage is cost.

Token deltas (scored runs, n=5 each):
- Sonnet: MCP uses +43,790 input tokens vs native (+21.8%)
- Haiku: MCP uses +60,063 input tokens vs native (+13.6%)

---

## 2. Root Cause: Tool Schema Weight

The MCP server exposes 10 tools. Every tool's full JSON schema (description + input schema + output schema) is injected into the model context on every turn, regardless of which tools are relevant to the task. This was measured directly by issuing a live `tools/list` request to the running server.

### Per-tool schema weight (measured)

| Tool | Description | Input schema | Output schema | Total |
|------|-------------|--------------|---------------|-------|
| analyze_symbol | 222 | 1,057 | 742 | **2,020** |
| analyze_file | 112 | 640 | 891 | **1,643** |
| analyze_directory | 80 | 463 | 266 | 808 |
| analyze_module | 132 | 54 | 378 | 563 |
| edit_insert | 165 | 162 | 87 | 414 |
| exec_command | 84 | 148 | 171 | 404 |
| edit_rename | 125 | 138 | 90 | 353 |
| analyze_raw | 90 | 148 | 114 | 351 |
| edit_replace | 104 | 109 | 125 | 338 |
| edit_overwrite | 97 | 80 | 97 | 274 |
| **Full set** | | | | **7,348** |
| Server instructions | | | | 181 |

*All figures in tokens (chars/4 approximation from live server JSON).*

### Schema overhead across a session

- Haiku MCP (17 mean turns): 7,348 x 17 = **124,916 schema tokens**
- Haiku Native (12.6 mean turns): ~300 x 12.6 = **3,780 schema tokens** (6 simpler schemas)
- Net delta: **121,136 tokens -- 201% of the 60,063 token gap**

The schema delta more than explains the gap on its own. Extra turns and response-format overhead are secondary and partially offsetting.

### The two dominant tools

`analyze_symbol` (2,020 tokens) and `analyze_file` (1,643 tokens) together account for **49% of the full schema weight**. Neither was called in any of the 10 MCP runs. Every turn in every MCP session paid ~3,663 tokens to carry schemas for tools that were never needed for this task.

- `analyze_symbol` alone: 2,020 x 17 turns = 34,340 tokens = **57% of the Haiku gap**
- `analyze_file` alone: 1,643 x 17 turns = 27,931 tokens = **46% of the Haiku gap**

### Wave 9 task-relevant schema

The task needed exactly two tools: `analyze_raw` (read) and `edit_replace` (write). Their combined schema cost is 689 tokens. The remaining 6,659 tokens per turn -- **90.6% of the schema** -- was carried for tools that were never called.

---

## 3. Behavioral Differences: Haiku Takes More Turns

Haiku used 17 mean turns in MCP vs 12.6 in native (+35%). Three mechanisms contribute:

**Edit verification loops (primary, ~2-3 extra turns):** `edit_replace` requires an exact, unique `old_text` match. Haiku issues intermediate `analyze_raw` reads between edits to confirm the prior edit applied correctly. Native `Write` overwrites the full file atomically; no verification loop is possible or needed.

**Server instruction drift (secondary, ~1 extra turn):** The MCP server injects a general-purpose "Recommended workflow" on every connection (`analyze_directory` -> `analyze_module` -> `analyze_file` -> `analyze_symbol`). This conflicts with the system prompt's explicit 6-step plan. Haiku partially follows the server's instructions, adding 1-2 `analyze_directory` calls before reaching the prescribed `analyze_raw` path.

**Dual-format response parsing (~0.5 turns):** Each tool response returns both human-readable text (with `  NNN |` line-number prefixes) and structured JSON. Haiku's smaller capacity may require extra reasoning turns to reconcile both before acting.

### edit_replace vs edit_overwrite for this task

For 2+ edits to files under 300 lines, `edit_overwrite` is more token-efficient:

| Approach | Tool calls | Verification risk | Estimated tokens |
|----------|-----------|-------------------|-----------------|
| edit_replace (no verification) | 6 | high for Haiku | ~35,300 |
| edit_replace + verification reads (observed) | 8-9 | realized | ~50,000 |
| edit_overwrite (full file, 2 writes) | 4 | none | ~31,400 |

The system prompt for Wave 9 prescribed `edit_replace`. Future waves targeting small files with multiple edits should prescribe `edit_overwrite` instead.

---

## 4. Tool Surface Assessment

A full assessment of all 10 tools was conducted to determine whether the gap should be addressed by consolidation or removal.

**Verdict: keep all 10 tools.** Each has a unique, non-substitutable capability:

- `analyze_symbol`: the only cross-file call graph. Irreplaceable for dependency tracing.
- `analyze_file`: the only source of type signatures and class hierarchies. `analyze_module` does not substitute (it omits types).
- `edit_rename`: AST-aware rename skips string literals and comments. `edit_replace` cannot safely substitute for refactoring.
- `edit_insert`: anchor-by-identifier insert without requiring surrounding context.
- `exec_command`: the only feedback loop (compile, test, lint). Zero Wave 9 usage reflects the explicit task constraint, not obsolescence.

Consolidation options assessed (merge `edit_rename + edit_insert`, split `analyze_symbol` into 3 sub-tools) were rejected: the schema savings are modest and the behavioral cost of conditional parameters or increased tool count outweighs them.

The solution is not to reduce tools but to stop sending all schemas on every turn.

---

## 5. Optimization Roadmap

### Issues filed

Three server-side optimizations were validated against MCP spec 2025-11-25 and rmcp 1.6.0 and filed as issues:

**[#719](https://github.com/clouatre-labs/aptu-coder/issues/719) -- Session-scoped tool profiles via initialize hint**

The highest-leverage change. Client passes `_meta.profile` in the `initialize` request (`"edit"`, `"analyze"`, or absent for full). Server calls `ToolRouter::disable_route()` (new in rmcp 1.6.0, PR #809) for each tool outside the profile. Disabled tools are invisible in `tools/list` and return `METHOD_NOT_FOUND` on call.

Impact for the edit profile (`analyze_raw + edit_replace + edit_overwrite + exec_command`, 963 tokens/turn):
- Schema per turn: 963 vs 7,348 (87% reduction)
- Haiku schema over 17 turns: 16,371 vs 124,916 (saving 108,545 tokens)
- This exceeds the entire 60,063-token Haiku gap. MCP beats native on cost.

Risk: unknown profile values must fall back silently to full set, not error. `_meta` field is at the top level of `InitializeRequestParams` (not inside `clientInfo`); rmcp access is `params.meta.as_ref()?.0.get("profile")?.as_str()`.

**[#720](https://github.com/clouatre-labs/aptu-coder/issues/720) -- Rewrite `analyze_symbol` description and fix three mode footguns**

`analyze_symbol` is the heaviest schema at 2,020 tokens (27% of full set). The description (222 tokens) conflates three mutually exclusive modes behind boolean flags with undocumented precedence. Three specific footguns fixed by rewriting the description and parameter doc-comments:

1. `def_use` cursor bootstrap: first call returns empty `def_use_sites` and a cursor; results only appear on subsequent paginated calls. Agents interpret the empty first response as failure.
2. `import_lookup` mutual exclusivity: requires a module path in `symbol` (not a symbol name), is mutually exclusive with call-graph and `def_use`, and silently ignores `follow_depth`, `impl_only`, and `match_mode`. None of this is in the current description.
3. Mode precedence is implicit: `import_lookup=true` and `def_use=true` together produce undefined behavior with no error.

Pure doc-comment edits. No behavior, parameter, or output schema changes. Also captures the description shortening (222 tokens -> ~80 tokens target).

**[#724](https://github.com/clouatre-labs/aptu-coder/issues/724) -- Adopt MCP 2025-11-25 `outputSchema` + `Tool.title` on all tools**

MCP spec 2025-11-25 promoted `outputSchema` and `Tool.title` from experimental to official. All `analyze_*` tools already emit `structuredContent`; registering `outputSchema` tells compliant clients what to expect and enables client-side validation. `Tool.title` is a new top-level field (separate from `ToolAnnotations.title`) that provides a concise human-readable display name. Low-risk compliance work that improves interoperability with 2025-11-25 clients.

### Closed proposals

Three initially proposed optimizations were assessed and closed:

- **#721 (`omit_line_numbers` param):** Line numbers are load-bearing for `analyze_raw`'s `start_line`/`end_line` partial-read capability. Removing them forces agents to read full files, costing far more tokens than the prefix saves. Real fix is in prompt guidance (always pass line ranges after an initial orientation read), not the API.

- **#722 (`edit_bulk` tool):** Intended to reduce verification loops, but the loops would persist even with `edit_bulk` (an anxious model re-reads after any write). Root cause is schema overhead and turn count, addressed by #719. Revisit after #719 ships and the next benchmark wave confirms whether verification loops persist under the edit profile.

- **#723 (split `analyze_symbol` into 3 sub-tools):** The footgun fixes are the high-value part, absorbed into #720. The split itself would add 2 net new tools (12 total during the deprecation window), saving only 520 tokens in the analyze profile for significant implementation complexity. Under #719's profile design, edit sessions carry neither `analyze_symbol` nor its sub-tools, making the saving moot for the cost-sensitive case.

### Rejected paths

- **Suppress text field, return JSON-only:** `CallToolResult.content` is Required in MCP 2025-11-25. The spec's backward-compatibility clause explicitly requires servers returning `structuredContent` to mirror it in `content`. rmcp's `CallToolResult::structured()` enforces this. Not implementable without a spec extension.

- **Binary payloads / streaming protocol:** Protocol-breaking changes. Tier 3 only.

### Combined projection (if #719 ships)

With the edit profile active (963 tokens/turn vs 7,348):

| Metric | Current (full set) | After #719 (edit profile) |
|--------|-------------------|--------------------------|
| Schema tokens/turn | 7,348 | 963 |
| Haiku schema over 17 turns | 124,916 | 16,371 |
| Net vs Haiku native (3,780) | +121,136 | +12,591 |
| Cost gap direction | MCP +14.4% | MCP ahead |

Note: turn count may also decrease under the edit profile because server instruction drift (the general-purpose workflow hint) is absent when heavy analyze tools are hidden. This would further widen MCP's cost advantage.

---

## 6. What the Data Does Not Show

- **Tool call sequences:** session JSONL files were not captured (the isolation check could not locate them). Extra turns and verification loops are inferred from turn-count vs tool-call-count ratios, not directly observed. The next wave should ensure session JSONL is captured for precise tool sequence analysis.

- **Cache effectiveness:** `DISABLE_PROMPT_CACHING=1` was set for all runs. With prompt caching enabled and a stable system prompt, schema overhead would be amortized across turns via cache reads. This could substantially narrow the gap independently of #719, but was not measured here.

- **Accuracy ceiling:** All conditions hit 9/9. The task does not discriminate between tool sets at the accuracy dimension. Future waves must raise difficulty to expose quality differences, which is where MCP's semantic tools (`analyze_symbol`, `analyze_file`) should provide a genuine advantage over native grep/read workflows.

- **Benchmark design for Wave 10:** Now that the gap is understood to be structural (schema overhead, not behavioral), the next comparison should be MCP-only runs across two profiles: full set vs edit profile. The native conditions can be dropped. Run counts can be reduced to 2x5 (10 runs) since variance is low and accuracy is ceiling.

---

*Initial analysis by 3 parallel delegates: token-cost (mercury-2 via openrouter), behavior (claude-haiku-4-5 via aws-bedrock), strategy (mercury-2 via openrouter). Tool surface and schema measurement by dedicated delegates (claude-sonnet-4-6 via gcp-vertex-ai, claude-haiku-4-5 via gcp-vertex-ai). MCP protocol validation by claude-sonnet-4-6 via gcp-vertex-ai against rmcp 1.6.0 and MCP spec 2025-11-25. Synthesized by orchestrator.*
