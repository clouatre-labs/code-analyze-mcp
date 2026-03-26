---
applyTo: "src/**/*.rs"
excludeAgent: "coding-agent"
description: "rmcp API patterns and Rust conventions for code-analyze-mcp source files"
---

## rmcp API correctness

Verify these against the actual `Cargo.lock` version of `rmcp` -- do not rely on training data.

- Use `Content`, not `RawContent`. `RawContent` does not exist in this codebase.
- Every `#[tool(...)]` attribute requires `output_schema = schema_for_type::<T>()`. Tool titles must be provided via `annotations(title = "...")`, consistent with the existing tool definitions in `src/lib.rs`. Flag any tool missing `output_schema` or using a top-level `title = "..."` field unless the `rmcp` macros are explicitly updated to support it.
- Tool methods must take a `RequestContext<RoleServer>` as the second parameter after `&self`, named either `context` or `_context` depending on whether it is used. Flag methods that omit this second parameter, use a different type, or place it in a different position.
- `#[tool_router]` goes on the `impl CodeAnalyzer` block. `#[tool_handler]` goes on `impl ServerHandler for CodeAnalyzer`. Flag if either attribute appears on the wrong impl block.
- Every `CallToolResult::success(...)` call must be followed by `.with_meta(Some(no_cache_meta()))`. Flag any success response missing `.with_meta(...)`.
- Transport entry point must follow: `let (stdin, stdout) = stdio(); let service = serve_server(analyzer, (stdin, stdout)).await?; service.waiting().await?`. Flag deviations.

## Error handling

- Use `thiserror` for error types (library crate). Flag use of `anyhow` in `src/`.
- Prefer `?` over `.unwrap()` or `.expect()` outside of test code. Flag `.unwrap()` calls in non-test `src/` files unless the comment explains why it cannot fail.
- `err_to_tool_result` is the canonical converter from `ErrorData` to `CallToolResult`. Flag direct construction of error `CallToolResult` that bypasses it.

## Observability

- Apply `#[instrument]` to all `async fn` that perform I/O or tree-sitter parsing. Flag async functions in `src/analyze.rs`, `src/parser.rs`, and `src/traversal.rs` that lack it.
- Log events go through `LogEvent` (see `src/logging.rs`). Flag direct `eprintln!` or `println!` in `src/`.

## Scope

- Flag any new dependency added to `Cargo.toml` that is not referenced in the PR description.
- Flag `unsafe` blocks unconditionally.
- Flag changes to files outside the scope described in the PR description.

## Tool parameter invariants

These are enforced at runtime and must be preserved in any refactor:

- `summary=true` and `cursor` passed together must return `INVALID_PARAMS`. Flag any change to the `summary_cursor_conflict` function or callers that weakens this check.
- `impl_only=true` is Rust-only; passing it for a non-Rust directory must return `INVALID_PARAMS`. Flag any change that allows `impl_only` to silently no-op on non-Rust paths instead of erroring.
- `analyze_module` accepts `path` only; `pagination`, `summary`, `force`, and `verbose` are not supported. Flag any PR that adds these parameters to `AnalyzeModuleParams`.
