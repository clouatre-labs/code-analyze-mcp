# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-04-20

### Breaking Changes

- **Crate rename**: The primary crate has been renamed from `code-analyze-mcp` to `aptu-coder`. Update your Cargo.toml dependencies from `code-analyze-mcp` to `aptu-coder`.
- **Binary rename**: The MCP server binary is now `aptu-coder` instead of `code-analyze-mcp`. Update any MCP configuration files to reference the new binary name.
- **XDG metrics directory**: Metrics are now stored in `~/.local/share/aptu-coder/` instead of `~/.local/share/code-analyze-mcp/`. Migration happens automatically on first run.
- **Homebrew formula rename**: The Homebrew formula has been renamed from `code-analyze-mcp` to `aptu-coder`. Uninstall the old formula and reinstall with the new name.

### Migration Steps

1. **For library consumers**: Update `Cargo.toml` to depend on `aptu-coder` instead of `code-analyze-mcp`:
   ```toml
   [dependencies]
   aptu-coder-core = "0.7.0"
   ```

2. **For binary users**: Update any MCP configuration files to reference `aptu-coder` instead of `code-analyze-mcp` in the binary path and tool name prefixes (e.g., `mcp__aptu-coder__analyze_directory`).

3. **For Homebrew users**:
   ```bash
   brew uninstall code-analyze-mcp
   brew install aptu-coder
   ```

4. **Metrics migration**: The first run of `aptu-coder 0.7.0` will automatically migrate your metrics from `~/.local/share/code-analyze-mcp/` to `~/.local/share/aptu-coder/`. If both directories exist, a warning will be logged and no migration will occur; manually verify and clean up old metrics if needed.

### Added

- Automatic migration of legacy metrics directory on startup.

[0.7.0]: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.7.0
