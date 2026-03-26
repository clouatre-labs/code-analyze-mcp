---
applyTo: "tests/**/*.rs"
excludeAgent: "coding-agent"
description: "Test conventions for code-analyze-mcp"
---

## Structure

- One happy path and one edge case per distinct behavior. Flag test files where multiple tests exercise the same code path with only input variation (e.g., testing the same parse logic for five different languages when one representative suffices).
- Use AAA layout (Arrange, Act, Assert) with a blank line between each section. Flag tests that interleave setup and assertions.
- Use `TempDir` from the `tempfile` crate for filesystem fixtures. Flag tests that write to hardcoded paths, and avoid introducing new uses of `std::env::temp_dir()` directly.

## Assertions

- Prefer specific assertions over boolean ones: `assert_eq!(output.functions.len(), 2)` over `assert!(output.functions.len() > 0)`. Flag `assert!(x > 0)` when an exact count is knowable from the fixture.
- Assert on the property under test, not on implementation details. Flag assertions that check internal field names or intermediate struct layouts that are not part of the public API.

## Async tests

- Use `#[tokio::test]` for async test functions. Flag `#[test]` on functions that call `.await`.

## Fixtures

- Inline fixture source as a raw string literal (`r#"..."#`) for small cases. Flag test files that read fixture data from external files under `tests/` unless the fixture is reused across three or more test functions.
- Shared fixtures belong in `tests/fixtures.rs`. Flag duplicate fixture definitions across test files.

## Scope

- Test files must not contain `#[allow(dead_code)]` suppressing unused fixture helpers. Flag it: the helper should be removed or moved to `fixtures.rs`.
- Do not introduce new `use code_analyze_mcp::*` glob imports in test files; if you touch an existing glob import, replace it with explicit imports so it is clear what is under test.
