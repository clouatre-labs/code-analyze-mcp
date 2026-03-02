# AGENTS.md

## Project overview

- Rust MCP server for code structure analysis using tree-sitter
- 9 languages planned; only Rust is implemented
- Three analysis modes: Overview, FileDetails (implemented); SymbolFocus (planned Wave 3)
- Rust edition 2024, async with tokio, MCP protocol via `rmcp`
- Single crate, Apache-2.0 licensed
- See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for design and module map

## Commands

```
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo deny check advisories licenses
```

## API verification (critical)

- Do not rely on training data for `rmcp`, `schemars`, or `thiserror` APIs
- These crates evolve rapidly
- Check `Cargo.lock` for the installed version before using any API
- Verify the method exists in crate source under `~/.cargo/registry/src/` or via `cargo doc --open`
- Follow patterns already established in existing code; it is the most reliable reference

## rmcp patterns (verify against installed version)

```rust
use rmcp::{
    model::{CallToolResult, ErrorData, RawContent},
    tool,
    tool_router,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
};

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
        Ok(CallToolResult::success(vec![RawContent::text("result")]))
    }
}
```

- Tool annotations (all `Option<bool>`):
  `read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint`
- `ServerHandler` impl with `#[tool_handler]` macro for `call_tool` and `list_tools`
- Transport: `transport-io` feature for stdio

## tree-sitter patterns

- Use `tree-sitter` 0.26+ with language-specific grammar crates
- Grammar crates export `LANGUAGE` const; use `tree_sitter_rust::LANGUAGE.into()` pattern
- Parse files with `Parser::new()`, set language, then `parser.parse(source, None)`
- Query results use `StreamingIterator`; iterate with `while let Some(mat) = matches.next()`
- Walk the tree with `TreeCursor` for efficient traversal
- Node kinds are language-specific; check grammar definitions
- All analysis is read-only; tree-sitter does not modify source files

## Rust conventions

- Edition 2024 idioms
- `thiserror` 2.x for all error types; no `anyhow` (library-style crate)
- `tokio` for async; `#[instrument]` on key functions via `tracing`
- `schemars` 1.x for JSON Schema generation; use `#[schemars(description = "...")]` on every field
- `serde` + `serde_json` for serialization
- No `unwrap()` or `expect()` in library code; use `?` with typed errors
- No `println!` or `eprintln!`; use `tracing::{info, warn, error, debug}`
- Prefer `&str` over `String` in function parameters where ownership is not needed

## Architecture patterns

- `Parameters<T>` struct pattern for tool parameters (see rmcp example above)
- MCP tool annotations: `read_only_hint = true` for all tools (this server is read-only)
- Analysis modes: Overview (directory tree), FileDetails (semantic extraction) are implemented
- SymbolFocus (call graphs) is planned for Wave 3
- Language handler system: `LanguageInfo` registry with tree-sitter queries and semantic handlers
- Adding a language: see [ARCHITECTURE.md](docs/ARCHITECTURE.md#language-handler-system)
- `rayon` for parallel file processing; `ignore` crate for .gitignore-aware directory walking

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

## Project status

- Only Rust language support is implemented
- README.md shows 9 languages; that is aspirational
- Roadmap: [issue #1](https://github.com/clouatre-labs/code-analyze-mcp/issues/1)
- Wave 0 (Foundation): complete
- Wave 1 (Tooling): complete
- Wave 2 (Overview, FileDetails modes; Rust): in progress
- Wave 3 (SymbolFocus / call graphs): planned
- Wave 4 (Caching, performance): planned

## Design references

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - design, module map, data flow, language handlers
- [README.md](README.md) - Quick start, tool interface, analysis modes, supported languages
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
