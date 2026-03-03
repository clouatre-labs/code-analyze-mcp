---
name: Refactor
about: Improve code quality, maintainability, or performance
title: "[REFACTOR] "
labels: refactor
assignees: ""
---

## Summary
<!-- 1-2 sentences: what to improve and why. Be specific about the pain point. -->

Example: "Extract query compilation logic from language handlers into a shared QueryCache. Reduces duplication and improves startup time."

## Motivation
<!-- Why now? What pain point triggered this? What will improve? -->

- Current pain point: describe the problem (e.g., code duplication, performance bottleneck, maintainability issue)
- Triggered by: link to related issue or observation
- Expected benefit: what improves (performance, readability, testability, etc.)

Example: "Each language handler recompiles tree-sitter queries on every analysis run. This is wasteful and slows down batch processing. A shared cache would eliminate redundant work and improve throughput by ~30% (estimated)."

## Current State
<!-- Show the problem with code references. Include file paths, line ranges, and code snippets. -->

File: `src/language/rust.rs` (lines 10-30)
```rust
// Current pattern: queries compiled on every call
fn analyze(&self, source: &str) -> Result<Overview> {
    let mut query = self.grammar.query(QUERY_STRING)?;  // Recompiled every time
    // ... analysis logic
}
```

File: `src/language/python.rs` (lines 15-35)
```rust
// Same pattern repeated in Python handler
fn analyze(&self, source: &str) -> Result<Overview> {
    let mut query = self.grammar.query(QUERY_STRING)?;  // Recompiled every time
    // ... analysis logic
}
```

Problem: `QUERY_STRING` is parsed and compiled on every call, even though it never changes.

## Proposed Changes
<!-- What to change and how. Reference design patterns and architecture. -->

1. **Create QueryCache struct** in `src/language/cache.rs` (new file)
   - Use `once_cell::sync::Lazy` or `std::sync::OnceLock` for thread-safe initialization
   - Store compiled queries keyed by language name
   - Provide `get_query(language: &str) -> Result<&Query>` method

2. **Update language handlers** to use QueryCache
   - File: `src/language/rust.rs` (lines 10-30)
   - File: `src/language/python.rs` (lines 15-35)
   - Change: replace inline query compilation with `QueryCache::get_query("rust")?`

3. **Register cache in LanguageInfo**
   - File: `src/language/mod.rs`
   - Add `cache: Arc<QueryCache>` field to `LanguageInfo`
   - Initialize once at startup

### Design Pattern
Reference: singleton pattern with lazy initialization (see `std::sync::OnceLock` docs)

### Integration Notes
- No public API changes; cache is internal to language handlers
- Backward compatible: behavior unchanged, only performance improves
- Thread-safe: use `Arc<Mutex<>>` if mutation needed, or `OnceLock` for immutable cache

## Constraints
<!-- What must NOT change. Backward compatibility, public APIs, behavior. -->

- Public API of `LanguageInfo` must remain unchanged
- Behavior must be identical: same analysis results, same error handling
- No breaking changes to MCP tool interface
- Existing tests must pass without modification

## Acceptance Criteria
<!-- Checkbox list of verifiable outcomes. -->

- [ ] QueryCache implemented in `src/language/cache.rs`
- [ ] All language handlers updated to use QueryCache
- [ ] Startup time measured and documented (before/after)
- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] No behavior changes: analysis results identical to before
- [ ] Commit GPG signed and DCO signed-off
