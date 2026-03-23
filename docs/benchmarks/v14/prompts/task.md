## Task: ripgrep Sink Trait Implementation Audit

You are onboarding to the ripgrep codebase to add a new output format (e.g., CSV printer). ripgrep
is a line-oriented search tool optimized for speed and correctness. The Sink trait in
`crates/searcher/src/sink.rs` is the abstraction that drives all search output. Every output format
(standard, JSON, summary) implements this trait.

Your task:

1. Identify all concrete types that implement the Sink trait. For each implementation, provide the
   type name, the file where the impl block is defined, and the approximate line number of the impl
   block. Distinguish between live-path implementations (used in production output dispatch) and
   convenience/test-only implementations.

2. Trace the call chain from `SearchWorker::search` (in `crates/core/search.rs`) through to
   `Searcher::search_reader` (in `crates/searcher/src/searcher/mod.rs`). Identify which Sink
   implementations are instantiated at the dispatch point. For each function in the chain, provide
   the file and approximate line number.

3. Produce a change-impact map: which files and line ranges must be modified to add a new Sink
   implementation (e.g., a CSV printer) and integrate it into the dispatch path. Include all layers:
   new printer file, trait implementation, enum variant, match arm, and re-export.

Output must be valid JSON. Example structure:

```json
{
  "run_id": "RUN_ID_PLACEHOLDER",
  "condition": "CONDITION_PLACEHOLDER",
  "sink_impls": [
    {"type": "TypeName", "file": "path/relative/to/ripgrep/root", "impl_line": 0}
  ],
  "call_chain": [
    {"name": "FunctionName", "file": "path/relative/to/ripgrep/root", "approx_line": 0}
  ],
  "change_impact_map": [
    {"file": "path/relative/to/ripgrep/root", "line_range": "start-end", "change": "description"}
  ],
  "tool_calls_total": 0
}
```

Repository: BurntSushi/ripgrep, commit 4649aa9700619f94cf9c66876e9549d83420e16c
