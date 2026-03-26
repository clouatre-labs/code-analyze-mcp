# AGENTS.md

## Project overview

- Rust MCP server for code structure analysis using tree-sitter
- Four MCP tools: `analyze_directory`, `analyze_file`, `analyze_symbol`, `analyze_module`
- Languages: Rust, Go, Java, Python, TypeScript, TSX, Fortran
- Rust edition 2024, async with tokio, MCP protocol via `rmcp`
- Single crate, Apache-2.0 licensed

## Commands

```
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo deny check advisories licenses
cargo bench
cargo install --path . --profile release
```

## API verification (critical)

Do not rely on training data for `rmcp`, `schemars`, or `thiserror` APIs. Verify against `Cargo.lock` and installed crates. **The codebase is the most reliable reference** -- read `src/lib.rs` before adding any tool.

## Tool parameters (quick reference)

- `analyze_directory`: `path`, `max_depth`, `summary`, `cursor`, `page_size`, `force`, `verbose`
- `analyze_file`: `path`, `summary`, `cursor`, `page_size`, `force`, `verbose`, `fields` (functions | classes | imports), `ast_recursion_limit`
- `analyze_symbol`: `path`, `symbol`, `match_mode` (exact | insensitive | prefix | contains), `follow_depth`, `impl_only` (Rust only), `summary`, `cursor`, `page_size`, `force`, `verbose`, `max_depth`, `ast_recursion_limit`
- `analyze_module`: `path` only -- pagination, summary, force, and verbose are not supported

`summary=true` and `cursor` are mutually exclusive; passing both returns INVALID_PARAMS.

`impl_only=true` restricts `analyze_symbol` callers to `impl Trait for Type` blocks; returns INVALID_PARAMS for non-Rust directories.

## rmcp footguns

These are the patterns contributors consistently get wrong:

- Use `Content`, not `RawContent` (does not exist)
- Every `#[tool(...)]` requires `output_schema = schema_for_type::<T>()` and `title = "..."`
- Tool methods take `_context: RequestContext<RoleServer>` as second parameter
- `#[tool_handler]` goes on `impl ServerHandler`, separate from `#[tool_router]` on the tool impl
- Apply `.with_meta(Some(no_cache_meta()))` on every `CallToolResult::success(...)` response
- Transport entry point: `let (stdin, stdout) = stdio(); serve_server(analyzer, (stdin, stdout)).await?`

## Adding a language

Follow an existing handler in `src/languages/`. The extension map is in `src/lang.rs`; the `LanguageInfo` registry with queries is in `src/languages/mod.rs`.

## Non-interactive workflows

Set `DISABLE_PROMPT_CACHING=1` for subagent pipelines.

## Design references

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - module map, data flow, language handler system
- [OBSERVABILITY.md](docs/OBSERVABILITY.md) - metrics schema, rotation, testability
- [anthropic-mcp-agents-orchestration.md](docs/anthropic-mcp-agents-orchestration.md) - MCP tool design principles (sections 3.2-3.3)
- rmcp: https://docs.rs/rmcp/latest/rmcp/
- tree-sitter: https://tree-sitter.github.io/tree-sitter/

## Do not

- Add dependencies without justification in the PR description
- Use `unsafe` code
- Implement features not specified in the assigned issue
- Modify files outside the scope of the assigned issue
- Assume any API exists based on training data; verify against installed crate versions
- Reference host-specific tools or clients in tool descriptions or server instructions (e.g. Claude Code's Grep, Glob, Read)
- Use `gh release create` to tag releases; always create a GPG-signed annotated tag with `git tag -s vX.Y.Z -m "Release vX.Y.Z"` and push it with `git push origin main --tags` to trigger the release workflow
