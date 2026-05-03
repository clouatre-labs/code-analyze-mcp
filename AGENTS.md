# AGENTS.md

## Project structure

Rust workspace with two crates:

- `crates/aptu-coder-core` -- parsing, analysis, formatting, graph, pagination, types
- `crates/aptu-coder` -- MCP server, tool handlers, logging, metrics

Nine MCP tools: `analyze_directory`, `analyze_file`, `analyze_module`, `analyze_symbol`, `analyze_raw` (analyze_* family); `edit_overwrite`, `edit_replace`, `edit_rename`, `edit_insert` (edit_* family).
Rust edition 2024, async with tokio, MCP protocol via `rmcp`. Supported languages are listed in `crates/aptu-coder-core/src/lang.rs`.

## Commands

```
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo deny check advisories licenses
cargo bench
```

## API verification (critical)

Do not rely on training data for `rmcp`, `schemars`, or `thiserror` APIs. **Read `crates/aptu-coder/src/lib.rs` before adding or modifying any tool** -- it is the authoritative reference for tool handler patterns.

## rmcp footguns

Patterns contributors consistently get wrong:

- Use `Content`, not `RawContent` (does not exist)
- Every `#[tool(...)]` requires `output_schema = schema_for_type::<T>()` and `title = "..."`
- Tool methods take `_context: RequestContext<RoleServer>` as second parameter
- `#[tool_router]` goes on `impl CodeAnalyzer`; `#[tool_handler]` goes on `impl ServerHandler for CodeAnalyzer` -- they are separate impls
- Apply `.with_meta(Some(no_cache_meta()))` on every `CallToolResult::success(...)` response
- Transport entry point: `let (stdin, stdout) = stdio(); let service = serve_server(analyzer, (stdin, stdout)).await?; service.waiting().await?`

## Adding a language

Follow an existing handler in `crates/aptu-coder-core/src/languages/`. The extension map is in `crates/aptu-coder-core/src/lang.rs`; the `LanguageInfo` registry with queries is in `crates/aptu-coder-core/src/languages/mod.rs`.

## Tool parameters

Canonical parameter lists live in the `types` module (`crates/aptu-coder-core/src/types.rs`). Key non-obvious constraints:

- `summary=true` and `cursor` are mutually exclusive; passing both returns INVALID_PARAMS.
- `impl_only=true` restricts `analyze_symbol` callers to `impl Trait for Type` blocks; returns INVALID_PARAMS for non-Rust directories.
- `analyze_module` supports `path` only -- pagination, summary, force, and verbose are not supported.

## Do not

- Add dependencies without justification in the PR description
- Use `unsafe` code
- Implement features not specified in the assigned issue
- Modify files outside the scope of the assigned issue
- Assume any API exists based on training data; verify against installed crate versions
- Reference host-specific tools or clients in tool descriptions or server instructions (e.g. Claude Code's Grep, Glob, Read)
- Use `gh release create` to tag releases; always create a GPG-signed annotated tag and push it to trigger the release workflow
- Remove `DISABLE_PROMPT_CACHING=1` from server instructions; caching data never read again is detrimental
