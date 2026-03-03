---
name: Feature
about: Propose a new feature or enhancement
title: "[FEATURE] "
labels: enhancement
assignees: ""
---

## Summary
<!-- 1-2 sentences describing what to build. Be specific: what capability is missing, what user need does it address? -->

Example: "Add Python language support to the analyzer using tree-sitter-python grammar. This enables the Overview and FileDetails modes for Python projects."

## Context
<!-- Why does this matter? What depends on it? Link to parent issues, roadmap, or design docs. Help LLM agents understand the broader system impact. -->

- What problem does this solve?
- Which users or workflows benefit?
- Links to related issues, discussions, or architecture docs (e.g., ARCHITECTURE.md, issue #1 roadmap)
- Any blocking dependencies?

Example: "Python is the 2nd most-requested language (issue #1). Implementing it unblocks Wave 2 completion and enables testing of the language handler abstraction before Wave 3 (call graphs)."

## Prerequisites
<!-- List any issues that must be completed first. Use "Depends on: #N" format. -->

- Depends on: #N (if applicable)

## Implementation Notes
<!-- The meat of the issue. LLM agents parse this to understand exactly what to build. Include code examples, API references, verified crate versions, design decisions, and integration points. -->

### Strategy
<!-- Numbered approaches or key decisions. Reference AGENTS.md and ARCHITECTURE.md where relevant. -->

1. **Add tree-sitter-python grammar crate** to Cargo.toml
   - Verified version: check `Cargo.lock` for installed version
   - API pattern: `use tree_sitter_python::LANGUAGE`
   - Integration: register in `LanguageInfo` registry (see `src/language/mod.rs`)

2. **Implement Python-specific queries** for Overview and FileDetails modes
   - File: `src/language/python.rs` (new)
   - Reference existing Rust implementation: `src/language/rust.rs` (lines 1-50 for pattern)
   - Queries needed: function definitions, class definitions, imports
   - Use tree-sitter query syntax; test with `tree-sitter query` CLI

3. **Register Python handler** in language registry
   - File: `src/language/mod.rs`
   - Pattern: add `LanguageInfo` entry with grammar, queries, and semantic extractor
   - See existing Rust entry for reference

### Code Examples
<!-- Show expected patterns, API usage, and integration points. -->

```rust
// Expected pattern for language handler registration
let python_info = LanguageInfo {
    name: "python",
    extensions: vec!["py"],
    grammar: tree_sitter_python::LANGUAGE.into(),
    queries: PythonQueries { /* ... */ },
};
```

### Integration Notes
- Parallel processing: use `rayon` for file analysis (see `src/analyzer.rs`)
- Error handling: use `thiserror` for Python-specific errors
- Logging: use `tracing` macros, not `println!`
- Testing: add tests in `tests/integration/python.rs`

### API References
- tree-sitter: https://docs.rs/tree-sitter/latest/tree_sitter/
- tree-sitter-python: check `Cargo.lock` for version, then https://docs.rs/tree-sitter-python/
- ARCHITECTURE.md: language handler system (section "Language Handler System")

## Acceptance Criteria
<!-- Checkbox list of verifiable outcomes. LLM agents use this to validate completion. -->

- [ ] Python grammar crate added to Cargo.toml and Cargo.lock
- [ ] `src/language/python.rs` implements Overview and FileDetails modes
- [ ] Python handler registered in `src/language/mod.rs`
- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Integration tests cover happy path and edge cases (e.g., syntax errors, empty files)
- [ ] No `unwrap()`, `expect()`, or `println!` in library code
- [ ] Commit GPG signed and DCO signed-off

## Not In Scope
<!-- Explicit boundaries to prevent scope creep. -->

- SymbolFocus mode (call graphs) - planned for Wave 3
- Performance optimization or caching - planned for Wave 4
- Support for other languages (Python only in this issue)
- Changes to MCP protocol or tool interface

## Additional Context
<!-- Optional: diagrams, links, related discussions, or examples. -->

- Related: issue #1 (roadmap), ARCHITECTURE.md (language handler system)
- Reference implementation: Rust handler in `src/language/rust.rs`
