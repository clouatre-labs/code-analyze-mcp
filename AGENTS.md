# AGENTS.md

## Project structure

Rust workspace with two crates:

- `crates/code-analyze-core` -- parsing, analysis, formatting, graph, pagination, types
- `crates/code-analyze-mcp` -- MCP server, tool handlers, logging, metrics

Four MCP tools: `analyze_directory`, `analyze_file`, `analyze_symbol`, `analyze_module`.
Languages: Rust, Go, Java, Python, TypeScript, TSX, Fortran.
Rust edition 2024, async with tokio, MCP protocol via `rmcp`.

## Commands

```
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo deny check advisories licenses
cargo bench
cargo install --path crates/code-analyze-mcp --profile release
```

## API verification (critical)

Do not rely on training data for `rmcp`, `schemars`, or `thiserror` APIs. **Read `crates/code-analyze-mcp/src/lib.rs` before adding or modifying any tool** -- it is the authoritative reference for tool handler patterns.

## rmcp footguns

Patterns contributors consistently get wrong:

- Use `Content`, not `RawContent` (does not exist)
- Every `#[tool(...)]` requires `output_schema = schema_for_type::<T>()` and `title = "..."`
- Tool methods take `_context: RequestContext<RoleServer>` as second parameter
- `#[tool_router]` goes on `impl CodeAnalyzer`; `#[tool_handler]` goes on `impl ServerHandler for CodeAnalyzer` -- they are separate impls
- Apply `.with_meta(Some(no_cache_meta()))` on every `CallToolResult::success(...)` response
- Transport entry point: `let (stdin, stdout) = stdio(); let service = serve_server(analyzer, (stdin, stdout)).await?; service.waiting().await?`

## Adding a language

Follow an existing handler in `crates/code-analyze-core/src/languages/`. The extension map is in `crates/code-analyze-core/src/lang.rs`; the `LanguageInfo` registry with queries is in `crates/code-analyze-core/src/languages/mod.rs`.

## Tool parameters (quick reference)

- `analyze_directory`: `path`, `max_depth`, `summary`, `cursor`, `page_size`, `force`, `verbose`
- `analyze_file`: `path`, `summary`, `cursor`, `page_size`, `force`, `verbose`, `fields` (functions | classes | imports), `ast_recursion_limit`
- `analyze_symbol`: `path`, `symbol`, `match_mode` (exact | insensitive | prefix | contains), `follow_depth`, `impl_only` (Rust only), `summary`, `cursor`, `page_size`, `force`, `verbose`, `max_depth`, `ast_recursion_limit`
- `analyze_module`: `path` only -- pagination, summary, force, and verbose are not supported

`summary=true` and `cursor` are mutually exclusive; passing both returns INVALID_PARAMS.

`impl_only=true` restricts `analyze_symbol` callers to `impl Trait for Type` blocks; returns INVALID_PARAMS for non-Rust directories.

## Do not

- Add dependencies without justification in the PR description
- Use `unsafe` code
- Implement features not specified in the assigned issue
- Modify files outside the scope of the assigned issue
- Assume any API exists based on training data; verify against installed crate versions
- Reference host-specific tools or clients in tool descriptions or server instructions (e.g. Claude Code's Grep, Glob, Read)
- Use `gh release create` to tag releases; always create a GPG-signed annotated tag with `git tag -s vX.Y.Z -m "Release vX.Y.Z"` and push it with `git push origin main --tags` to trigger the release workflow
