<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors -->

# code-analyze-core

Core library for code structure analysis using tree-sitter.

<p align="center">
  <a href="https://docs.rs/code-analyze-core"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-code--analyze--core-66c2a5?style=flat-square&labelColor=555555&logo=docs.rs" height="20"></a>
  <a href="https://crates.io/crates/code-analyze-mcp"><img alt="MCP server" src="https://img.shields.io/badge/MCP-code--analyze--mcp-fc8d62?style=flat-square&labelColor=555555&logo=rust" height="20"></a>
  <a href="https://api.reuse.software/info/github.com/clouatre-labs/code-analyze-mcp"><img alt="REUSE" src="https://img.shields.io/reuse/compliance/github.com/clouatre-labs/code-analyze-mcp?style=for-the-badge" height="20"></a>
  <a href="https://www.bestpractices.dev/projects/12275"><img alt="OpenSSF Best Practices" src="https://img.shields.io/cii/level/12275?style=for-the-badge" height="20"></a>
</p>

## Features

- **Directory analysis** - File tree with LOC, function, and class counts
- **File analysis** - Functions, classes, and imports with signatures and line ranges
- **Symbol call graphs** - Callers and callees across a directory with configurable depth
- **Module index** - Lightweight function and import index (~75% smaller than full file analysis)
- **In-memory analysis** - `analyze_str` parses source text directly without a file path; returns the same `FileAnalysisOutput` as `analyze_file`
- **Multi-language** - Rust, Python, TypeScript, TSX, Go, Java, Fortran, JavaScript, C/C++, C#
- **Pagination** - Cursor-based pagination for large outputs
- **Caching** - LRU cache for parsed results with mtime-based invalidation
- **Parallel** - Rayon-based parallel file analysis

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
code-analyze-core = "0.3"
```

## Example

```rust,no_run
use code_analyze_core::{analyze_directory, analyze_file, analyze_str, AnalysisConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Analyze a directory (depth 2, compact summary)
    let output = analyze_directory("src/", Some(2), true, None, None, false, false).await?;
    println!("{}", output.formatted);

    // Analyze a single file
    let output = analyze_file("src/lib.rs", false, None, None, false, false, None, None).await?;
    println!("{}", output.formatted);

    // Analyze source text in memory (no file path required)
    let source = std::fs::read_to_string("src/lib.rs")?;
    let output = analyze_str(&source, "rs", None).await?;
    println!("{}", output.formatted);

    Ok(())
}
```

## Supported Languages

| Language | Extensions | Feature flag |
|----------|------------|--------------|
| Rust | `.rs` | `lang-rust` |
| Python | `.py` | `lang-python` |
| TypeScript | `.ts` | `lang-typescript` |
| TSX | `.tsx` | `lang-tsx` |
| Go | `.go` | `lang-go` |
| Java | `.java` | `lang-java` |
| Fortran | `.f`, `.f77`, `.f90`, `.f95`, `.f03`, `.f08`, `.for`, `.ftn` | `lang-fortran` |
| JavaScript | `.js`, `.mjs`, `.cjs` | `lang-javascript` |
| C/C++ | `.c`, `.cc`, `.cpp`, `.cxx`, `.h`, `.hpp`, `.hxx` | `lang-cpp` |
| C# | `.cs` | `lang-csharp` |

## Configuration

`AnalysisConfig` provides resource limits for library consumers:

```rust
use code_analyze_core::AnalysisConfig;

let config = AnalysisConfig {
    max_file_bytes: Some(1_000_000), // skip files > 1 MB
    parse_timeout_micros: None,      // reserved, no-op in 0.3
    cache_capacity: None,            // use default LRU capacity
};
```

## Support

For questions and support, visit [clouatre.ca](https://clouatre.ca/about/).

## License

Apache-2.0. See [LICENSE](https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE).
