# AGENTS.md

## Project overview

Rust MCP server providing code structure analysis tools (directory overview, file details, symbol call graphs) using tree-sitter.
Rust edition 2021, async with tokio, MCP protocol via `rmcp` crate.
Single-crate binary, MIT licensed.

## Commands

```
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo deny check advisories licenses
```

## API verification (critical)

Do not rely on training data for `rmcp`, `schemars`, or `thiserror` APIs. These crates evolve rapidly.
Before using any API: check `Cargo.lock` for the installed version, then verify the method exists in the crate source under `~/.cargo/registry/src/` or by running `cargo doc --open`.
If the repo has existing source code, follow the patterns already established there. Existing code is the most reliable reference.

## rmcp patterns (verify against installed version)

```rust
use rmcp::{ErrorData, model::*, tool, tool_router, handler::server::tool::ToolRouter};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MyParams {
    #[schemars(description = "Description of the parameter")]
    path: String,
}

#[derive(Clone)]
struct MyServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MyServer {
    fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }

    #[tool(name = "my_tool", description = "What this tool does", annotations(read_only_hint = true))]
    async fn my_tool(&self, params: Parameters<MyParams>) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![Content::text("result")]))
    }
}
```

- Tool annotations: `read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint` (all `Option<bool>`)
- `ServerHandler` impl with `#[tool_handler]` macro for `call_tool` and `list_tools`
- Transport: `transport-io` feature for stdio

## tree-sitter patterns

- Use `tree-sitter` 0.21 with language-specific grammar crates
- Parse files with `Parser::new()`, set language, then `parser.parse(source, None)`
- Walk the tree with `TreeCursor` for efficient traversal
- Node kinds are language-specific; check grammar definitions
- All analysis is read-only; tree-sitter does not modify source files

## Rust conventions

- Edition 2021 idioms
- `thiserror` for all error types; no `anyhow` (library-style crate)
- `tokio` for async; `#[instrument]` on all async functions via `tracing`
- `schemars` 1.x for JSON Schema generation; use `#[schemars(description = "...")]` on every field
- `serde` + `serde_json` for serialization
- No `unwrap()` or `expect()` in library code; use `?` with typed errors
- No `println!` or `eprintln!`; use `tracing::{info, warn, error, debug}`
- Prefer `&str` over `String` in function parameters where ownership is not needed

## Architecture patterns

- `Parameters<T>` struct pattern for tool parameters (see rmcp example above)
- MCP tool annotations: `read_only_hint = true` for all tools (this server is read-only)
- Three analysis modes: directory overview, file details, symbol focus (call graphs)
- `rayon` for parallel file processing; `ignore` crate for .gitignore-aware directory walking
- `lru` crate for parse tree caching

## Code style

- One happy path test + one edge case test per behavior; no redundant variations
- AAA pattern (Arrange, Act, Assert) in tests
- Keep functions under 50 lines; extract helpers for complex logic
- Group imports: std, external crates, internal modules (separated by blank lines)

## Commit conventions

- Conventional commits: `type(scope): description`
- Types: feat, fix, refactor, test, docs, ci, chore
- GPG sign all commits: `-S` flag
- DCO sign-off all commits: `--signoff` flag
- Do not add co-author trailers for AI agents

## Reference documentation (for agents with web access)

- rmcp: https://docs.rs/rmcp/latest/rmcp/
- MCP specification: https://spec.modelcontextprotocol.io/
- tree-sitter: https://tree-sitter.github.io/tree-sitter/
- schemars 1.x: https://docs.rs/schemars/latest/schemars/
- thiserror: https://docs.rs/thiserror/latest/thiserror/

## Do not

- Add dependencies without justification in the PR description
- Use `unsafe` code
- Use `unwrap()`, `expect()`, `println!`, or `eprintln!`
- Implement features not specified in the assigned issue
- Modify files outside the scope of the assigned issue
- Assume any API exists based on training data; verify against installed crate versions
