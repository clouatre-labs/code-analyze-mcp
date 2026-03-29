<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- SPDX-FileCopyrightText: 2025 code-analyze-mcp Contributors -->

# code-analyze-core

Core library for code structure analysis using tree-sitter.

[![docs.rs](https://img.shields.io/badge/docs.rs-code--analyze--core-66c2a5?style=flat-square&labelColor=555555&logo=docs.rs)](https://docs.rs/code-analyze-core)
[![MCP server](https://img.shields.io/badge/MCP-code--analyze--mcp-fc8d62?style=flat-square&labelColor=555555&logo=rust)](https://crates.io/crates/code-analyze-mcp)
[![REUSE](https://api.reuse.software/badge/github.com/clouatre-labs/code-analyze-mcp)](https://api.reuse.software/info/github.com/clouatre-labs/code-analyze-mcp)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12275/badge)](https://www.bestpractices.dev/projects/12275)

## Features

- **Directory analysis** - File tree with LOC, function, and class counts
- **File analysis** - Functions, classes, and imports with signatures and line ranges
- **Symbol call graphs** - Callers and callees across a directory with configurable depth
- **Module index** - Lightweight function and import index (~75% smaller than full file analysis)
- **Multi-language** - Rust, Python, TypeScript, TSX, Go, Java, Fortran
- **Pagination** - Cursor-based pagination for large outputs
- **Caching** - LRU cache for parsed results with mtime-based invalidation
- **Parallel** - Rayon-based parallel file analysis

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
code-analyze-core = "0.2"
```

## Example

```rust,no_run
use code_analyze_core::{analyze_directory, analyze_file, AnalysisConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Analyze a directory (depth 2, compact summary)
    let output = analyze_directory("src/", Some(2), true, None, None, false, false).await?;
    println!("{}", output.formatted);

    // Analyze a single file
    let output = analyze_file("src/lib.rs", false, None, None, false, false, None, None).await?;
    println!("{}", output.formatted);

    Ok(())
}
```

## Supported Languages

| Language | Extensions |
|----------|-----------|
| Rust | `.rs` |
| Python | `.py` |
| TypeScript | `.ts`, `.tsx` |
| Go | `.go` |
| Java | `.java` |
| Fortran | `.f`, `.f77`, `.f90`, `.f95`, `.f03`, `.f08`, `.for`, `.ftn` |

## Configuration

`AnalysisConfig` provides resource limits for library consumers:

```rust
use code_analyze_core::AnalysisConfig;

let config = AnalysisConfig {
    max_file_bytes: Some(1_000_000), // skip files > 1 MB
    parse_timeout_micros: None,      // reserved, no-op in 0.2
    cache_capacity: None,            // use default LRU capacity
};
```

## Support

For questions and support, visit [clouatre.ca](https://clouatre.ca/about/).

## License

Apache-2.0. See [LICENSE](https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE).
