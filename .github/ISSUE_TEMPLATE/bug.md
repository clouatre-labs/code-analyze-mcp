---
name: Bug
about: Report a defect or unexpected behavior
title: "[BUG] "
labels: bug
assignees: ""
---

## Summary
<!-- 1-2 sentences: what is broken? Be specific about the symptom, not the suspected cause. -->

Example: "FileDetails mode crashes when analyzing Rust files with syntax errors. Expected graceful error handling, got panic."

## Steps to Reproduce
<!-- Numbered list of exact steps to trigger the bug. Include file paths, command-line flags, and input data. -->

1. Create a Rust file with a syntax error (e.g., missing semicolon)
2. Run: `cargo run -- --mode file-details path/to/broken.rs`
3. Observe: panic in stderr

## Expected Behavior
<!-- What should happen? -->

The analyzer should return an error result with a descriptive message, not panic.

## Actual Behavior
<!-- What actually happens? Include error messages, stack traces, or unexpected output. -->

```
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: ...'
stack trace:
  at src/analyzer.rs:42
  at src/main.rs:15
```

## Environment
<!-- Provide context for reproduction. -->

- OS: macOS 14.2 / Linux / Windows
- Rust version: `rustc --version`
- Relevant crate versions: check `Cargo.lock` for tree-sitter, rmcp, etc.
- Command used: exact CLI invocation
- Input file: attach or describe the problematic file

## Logs / Error Output
<!-- Full error message, panic backtrace, or relevant log output. Use code block. -->

```
<paste full error output here>
```

## Root Cause Analysis
<!-- Optional: if you have a hypothesis about what's wrong, include it here. Reference file paths and line ranges. -->

Suspected cause: `src/analyzer.rs` line 42 calls `.unwrap()` on a `Result` that can fail when parsing malformed syntax. Should use `?` operator or explicit error handling instead.

## Fix Direction
<!-- Optional: suggested approach or pattern to follow. -->

Replace `.unwrap()` with `?` operator. Use `thiserror` to define a custom error type for parse failures. Ensure all error paths return `Err` instead of panicking.

Reference: error handling pattern in `src/language/rust.rs` (lines 50-75).

## Acceptance Criteria
<!-- Checkbox list of verifiable outcomes. -->

- [ ] Bug is reproducible with provided steps
- [ ] Root cause identified and documented
- [ ] Fix implemented without introducing new panics
- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Regression test added covering the bug scenario
- [ ] Commit GPG signed and DCO signed-off
