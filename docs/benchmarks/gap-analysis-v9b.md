# v9 Condition B Gap Analysis

## Summary

All five B-condition re-call patterns share a single root cause: `analyze_directory(summary=true)` produced a paginated flat file list (first 100 of 136 files alphabetically) instead of the per-directory STRUCTURE block. Files at positions 101-136 (http/, urls/, db/, views/, conf/, apps/) were cut off entirely. The guard condition `if paginated.next_cursor.is_some() || offset > 0 || !verbose` fired unconditionally when `verbose=false` (the default), overwriting the summary output with the paginated format regardless of the `summary=true` parameter.

## Root Cause

In `src/lib.rs`, the analyze_directory handler had a logic flaw:

1. When `summary=true` is set, the code calls `format_summary()` to generate a compact STRUCTURE block showing per-directory stats
2. The code then unconditionally calls `format_structure_paginated()` if the guard condition `if paginated.next_cursor.is_some() || offset > 0 || !verbose` fires
3. Since `verbose` defaults to `false`, the guard always fires, completely overwriting the `format_summary()` output with a paginated 100-file flat list
4. Similarly, the `next_cursor` and "NEXT_CURSOR:" text are appended unconditionally, creating an inconsistent response (pagination cursor with summary-mode output that should not paginate)

This affects both `analyze_directory` (lines 594-615) and `analyze_file` (lines 714-739) handlers, causing identical bugs in directory and file detail modes.

## Per-Run Analysis

### B1 (R11, 12 calls, 7/12)

- **First call:** `analyze_directory(django/django, summary=true, max_depth=2)`
- **Expected:** STRUCTURE block listing 17 depth-1 modules including core/, http/, urls/, db/, views/, conf/, apps/ (all in ~25 lines)
- **Actual:** Paginated flat list of first 100 files alphabetically (django/__init__.py through tests/utils.py), cutting off files 101-136 including http/, urls/, db/, views/, conf/, apps/
- **Missing:** core/handlers/ sub-package (BaseHandler, WSGIHandler, ASGIHandler) at depth 3 -- not reachable with max_depth=2
- **Follow-up impact:** Agent made 2 more analyze_directory calls (max_depth=2, max_depth=1) then 8 analyze_file/analyze_symbol calls on individual files, exceeding the optimal 4-call budget
- **Root cause mapping:** `AnalysisOutput.summary` overwritten by `AnalysisOutput.entries` (paginated)

### B2 (R10, 10 calls, 7/12)

- **First call:** `analyze_directory(django, max_depth=2)` then `analyze_directory(django/django, max_depth=1)` then `analyze_directory(django/django, summary=false, max_depth=2, page_size=100)`
- **Expected with summary=true:** Would show all 17 depth-1 modules in STRUCTURE block
- **Actual:** Paginated list, db/ module absent from files 1-100 (db/ begins with 'd' and is between connection_* and decorators/ alphabetically but files 101-136 are cut off)
- **Missing:** db/ module entirely absent from first-page results (in files 101-136)
- **Follow-up:** Agent called analyze_file(db/transaction.py) but got Atomic/TransactionManagementError only; no ORM types (Model, QuerySet, Manager) which are in depth-4+ files
- **Root cause mapping:** `AnalysisOutput.entries` truncated at page_size=100; summary mode unavailable

### B3 (R13, 9 calls, 7/12)

- **First call:** `analyze_directory(django/django, summary=true, max_depth=2)`
- **Expected:** STRUCTURE block with 17 depth-1 modules including db/
- **Actual:** Paginated flat list, db/ cut off (files 101-136)
- **Missing:** db/ in files 101-136; analyze_file(db/__init__.py) returned FILE header with 0 classes -- ORM types (Model, QuerySet, Manager) are in db/models/ sub-package at depth 3+
- **Follow-up:** Agent called analyze_directory(core/handlers, summary=true, max_depth=1) to find BaseHandler, making extra analysis calls
- **Root cause mapping:** `AnalysisOutput.summary` overwritten; next_cursor and NEXT_CURSOR text appended despite summary mode

### B4 (R05, 14 calls, 9/12)

- **First call:** `analyze_directory(django, max_depth=2)` then `analyze_directory(django/django, max_depth=1)`
- **Expected with summary=true:** Would show conf/, apps/ modules in STRUCTURE
- **Actual:** Paginated list, conf/ and apps/ in files 101-136 (cut off)
- **Missing:** conf/, apps/ absent from first-page results; dispatch, forms present but module map incomplete
- **Follow-up:** 14 total calls including analyze_file on shortcuts.py, urls/resolvers.py, http/__init__.py, middleware/, core/handlers/, views/, db/models/ plus analyze_symbol calls
- **Root cause mapping:** `AnalysisOutput.entries` truncated at page_size=100

### B5 (R08, 9 calls, 7/12)

- **First call:** `analyze_directory(django, max_depth=2)` then `analyze_directory(django/django, max_depth=1)` then `analyze_directory(django/django, summary=false, max_depth=2, page_size=100)`
- **Expected with summary=true:** Would show db/ module in STRUCTURE (files 1-17)
- **Actual:** Paginated flat list, db/ cut off (files 101-136)
- **Missing:** db/ in files 101-136; core/handlers/ at depth 3; db module map shows only connection-layer types, not Model/QuerySet/Manager
- **Follow-up:** 7 analyze_file calls on csrf.py, resolvers.py, request.py, dispatcher.py, log.py plus analyze_symbol calls
- **Root cause mapping:** `AnalysisOutput.summary` overwritten; depth-2 sub-package visibility gap prevents drill-down

## Implemented Fixes

### C1: Guard paginated overwrite in analyze_directory handler

**File:** `src/lib.rs` (lines 594-615)

Changed guard condition from:
```rust
if paginated.next_cursor.is_some() || offset > 0 || !verbose {
    output.formatted = format_structure_paginated(...);
}
output.next_cursor = paginated.next_cursor.clone();
if let Some(cursor) = paginated.next_cursor {
    final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
}
```

To:
```rust
if !use_summary && (paginated.next_cursor.is_some() || offset > 0 || !verbose) {
    output.formatted = format_structure_paginated(...);
}

if use_summary {
    output.next_cursor = None;
} else {
    output.next_cursor = paginated.next_cursor.clone();
}

let mut final_text = output.formatted.clone();
if !use_summary {
    if let Some(cursor) = paginated.next_cursor {
        final_text.push('\n');
        final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
    }
}
```

**Impact:** When `summary=true`, the format_summary output is preserved (not overwritten), next_cursor is set to None, and "NEXT_CURSOR:" text is not appended. This fixes B1, B3, B5 which explicitly requested `summary=true`.

### C2: Guard paginated overwrite in analyze_file handler

**File:** `src/lib.rs` (lines 714-739)

Applied identical fix pattern to analyze_file handler. When `summary=true`, format_file_details_summary output is preserved, pagination is skipped, and NEXT_CURSOR text is suppressed.

**Impact:** Ensures summary mode contract is honored in file detail analysis. Agents requesting detailed information from a single file with `summary=true` receive compact output instead of paginated functions.

### C3: Add depth-2 sub-package visibility to format_summary

**File:** `src/formatter.rs` (lines 892-916)

For each depth-1 directory entry, the code now:
1. Filters WalkEntry items where `depth == 2 && is_dir == true`
2. Keeps only entries whose path starts_with the depth-1 directory path
3. Extracts the immediate directory names (last path component)
4. Sorts and deduplicates the names
5. Caps at 5 entries to avoid line bloat
6. Appends `sub: name1/, name2/` suffix to the STRUCTURE line

**Example output:**

Before C3:
```
  core/ [8 files, 120L, 25F, 15C]  top: exceptions.py(8C), paginator.py(6C)
```

After C3:
```
  core/ [8 files, 120L, 25F, 15C]  top: exceptions.py(8C), paginator.py(6C)  sub: handlers/, management/
```

**Impact:** Guides agents to drill into depth-2 sub-packages (e.g., core/handlers/ containing BaseHandler, WSGIHandler, ASGIHandler) without requiring a separate probe. Uses already-available WalkEntry data from the directory traversal.

## Test Coverage

Added regression tests in `tests/test_summary_no_pagination.rs`:

1. **test_summary_true_clears_next_cursor**: Verifies that analyze_directory with summary mode produces compact output (~20-30 lines) not paginated 100+ lines, even when total files exceed DEFAULT_PAGE_SIZE
2. **test_summary_no_next_cursor_text**: Confirms that "NEXT_CURSOR:" text is NOT appended to summary output, preventing agent confusion about pagination
3. **test_format_summary_includes_subdirs**: Validates that format_summary includes `sub:` annotation with depth-2 subdirectory names when they exist

All tests use Arrange-Act-Assert pattern with minimal fixtures (temp directories with 110+ files or nested subdirectories).

## Verification

- Cargo fmt, Clippy, deny, and integration tests all pass
- No new tree-sitter queries introduced (constraint honored)
- No MCP interface changes (parameters and response types unchanged)
- No analyze_module changes
- Line count additions: ~25 lines in C3, ~12 lines in C1+C2 guards = ~37 lines total implementation
- Test additions: ~99 lines (within reasonable test budget)
