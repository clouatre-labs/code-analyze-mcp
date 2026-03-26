---
applyTo: "src/**/*.rs"
excludeAgent: "coding-agent"
description: "rmcp API patterns and Rust conventions for code-analyze-mcp source files"
---

## rmcp API correctness

Do not rely on training data for rmcp APIs -- verify against `Cargo.lock` and `src/lib.rs`.

- Every `#[tool(...)]` attribute requires `output_schema = schema_for_type::<T>()`. Titles go in `annotations(title = "...")`, not as a top-level `title = "..."` field. Flag tools missing `output_schema`.
- Tool methods must take a `RequestContext<RoleServer>` as the second parameter after `&self`. Flag methods that omit it or use a different type.
- `#[tool_router]` and `#[tool_handler]` go on separate `impl` blocks. Flag if either appears on the wrong block.
- Every `CallToolResult::success(...)` must be chained with `.with_meta(...)`. Flag success responses missing it.

## Error handling

- Use `thiserror` for error types (library crate). Flag use of `anyhow` in `src/`.
- Prefer `?` over `.unwrap()` or `.expect()` in non-test code. Flag `.unwrap()` in `src/` unless a comment explains why it cannot fail.

## Observability

- Flag `async fn` that perform I/O or tree-sitter parsing without `#[instrument]`.
- Flag direct `eprintln!` or `println!` in `src/`.

## Scope

- Flag any new dependency in `Cargo.toml` not justified in the PR description.
- Flag `unsafe` blocks unconditionally.
- Flag changes to files outside the scope described in the PR description.

## Tool parameter invariants

Behavioral contracts enforced at runtime -- must be preserved across refactors:

- `summary=true` and `cursor` together must return `INVALID_PARAMS`. Flag any change that weakens this check.
- `impl_only=true` on a non-Rust directory must return `INVALID_PARAMS`. Flag changes that allow it to silently no-op instead of erroring.
- `analyze_module` accepts `path` only. Flag any PR that adds pagination, summary, force, or verbose to its parameter type.
