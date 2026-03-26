---
applyTo: "src/**/*.rs"
excludeAgent: "coding-agent"
description: "rmcp API patterns and Rust conventions for code-analyze-mcp source files"
---

## rmcp API correctness

Verify these against the actual `Cargo.lock` version of `rmcp` -- do not rely on training data.

- Use `Content`, not `RawContent`. `RawContent` does not exist in this codebase.
- Every `#[tool(...)]` attribute requires both `output_schema = schema_for_type::<T>()` and `title = "..."`. Flag any tool missing either field.
- Tool methods must take `_context: RequestContext<RoleServer>` as the second parameter after `&self`. Flag methods that omit it or use a different type.
- `#[tool_router]` goes on the `impl CodeAnalyzer` block. `#[tool_handler]` goes on `impl ServerHandler for CodeAnalyzer`. Flag if either attribute appears on the wrong impl block.
- Every `CallToolResult::success(...)` call must be followed by `.with_meta(Some(no_cache_meta()))`. Flag any success response missing `.with_meta(...)`.
- Transport entry point must follow: `let (stdin, stdout) = stdio(); serve_server(analyzer, (stdin, stdout)).await?`. Flag deviations.

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
