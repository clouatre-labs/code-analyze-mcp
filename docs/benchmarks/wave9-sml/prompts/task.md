## Task: Kotlin Language Support Implementation

You are implementing Kotlin grammar support in the aptu-coder repository to enable symbol extraction
for .kt and .kts source files.

Repository: clouatre-labs/aptu-coder
Working directory: REPO_PATH_PLACEHOLDER

Issue #649 specifies this feature. The correct tree-sitter crate is `tree-sitter-kotlin-ng = "1.1.0"`
(NOT tree-sitter-kotlin 0.3.8 which is incompatible with the workspace tree-sitter version).

Your task:

1. Add `tree-sitter-kotlin-ng = "1.1.0"` to `[workspace.dependencies]` in the root `Cargo.toml`.

2. In `crates/aptu-coder-core/Cargo.toml`:
   - Add `tree-sitter-kotlin-ng = { workspace = true, optional = true }` to `[dependencies]`
   - Add `lang-kotlin = ["dep:tree-sitter-kotlin-ng"]` to `[features]`
   - Add `lang-kotlin` to the `default` feature set

3. Create `crates/aptu-coder-core/src/languages/kotlin.rs` with:
   - SPDX-FileCopyrightText header (Apache-2.0, same format as java.rs)
   - `ELEMENT_QUERY` constant (function_declaration, class_declaration, interface_declaration, enum_class_declaration, object_declaration node kinds)
   - `CALL_QUERY` constant (call_expression node kind)
   - `REFERENCE_QUERY` constant (identifier node kind)
   - `IMPORT_QUERY` constant (import_header node kind)
   - `DEFUSE_QUERY` constant (modeled after java.rs)
   - `extract_inheritance` function (walks delegation_specifiers; distinguish superclass with parens from interfaces without parens)
   - At least 3 unit tests under `#[cfg(all(test, feature = "lang-kotlin"))]` covering: element extraction, inheritance extraction, call extraction

4. In `crates/aptu-coder-core/src/languages/mod.rs`:
   - Add `#[cfg(feature = "lang-kotlin")] pub mod kotlin;`
   - Add `"kotlin"` arm to `get_language_info()` returning a complete `LanguageInfo` struct
   - Add `"kotlin"` arm to `get_ts_language()`

5. In `crates/aptu-coder-core/src/lang.rs`:
   - Add `.kt` and `.kts` extensions to `EXTENSION_MAP` behind `#[cfg(feature = "lang-kotlin")]`
   - Add `"kotlin"` to `supported_languages()`

**Do not run `cargo test` or any build commands.** The benchmark infrastructure will verify compilation and test results externally after you complete your implementation.

All 5 query types (ELEMENT_QUERY, CALL_QUERY, REFERENCE_QUERY, IMPORT_QUERY, DEFUSE_QUERY) must be present in queries_written.

Output must be valid JSON matching this exact schema:

```json
{
  "run_id": "RUN_ID_PLACEHOLDER",
  "condition": "CONDITION_PLACEHOLDER",
  "files_created": [
    {"path": "path/relative/to/repo/root", "line_count": 0, "has_spdx_header": true}
  ],
  "files_modified": [
    {"path": "path/relative/to/repo/root", "changes_description": "what was changed"}
  ],
  "feature_flag_name": "lang-kotlin",
  "ts_crate_used": "tree-sitter-kotlin-ng",
  "ts_crate_version": "1.1.0",
  "ts_entry_point": "tree_sitter_kotlin_ng::LANGUAGE",
  "queries_written": [
    {"query_name": "ELEMENT_QUERY", "present": true, "node_kinds": ["function_declaration", "class_declaration", "interface_declaration", "enum_class_declaration", "object_declaration"]},
    {"query_name": "CALL_QUERY", "present": true, "node_kinds": ["call_expression"]},
    {"query_name": "REFERENCE_QUERY", "present": true, "node_kinds": ["identifier"]},
    {"query_name": "IMPORT_QUERY", "present": true, "node_kinds": ["import_header"]},
    {"query_name": "DEFUSE_QUERY", "present": true, "node_kinds": ["...modeled after java.rs"]}
  ],
  "extract_inheritance_present": true,
  "extension_registrations": [".kt", ".kts"],
  "test_names": ["test_kotlin_element_query", "..."],
  "compile_belief": "confident_pass",
  "compile_belief_reason": "used kotlin-ng 1.1.0 compatible with tree-sitter 0.26.x",
  "tool_calls_total": 0
}
```
