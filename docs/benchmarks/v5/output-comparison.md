# V5 Output Comparison: code-analyze-mcp vs Native Analyze

## Purpose

The v5 [analysis.md](analysis.md) answers "did tool isolation close the efficiency gap?" (yes,
on calls and wall time; no, on tokens). This document answers the follow-up: "why does the 22%
token overhead persist, and what can we do about it?"

The findings here directly informed the output compaction epic (#128) and its sub-issues
(#129, #130, #132, #133, #134, #136).

## Method

Compared tool responses from matched v5 benchmark runs:
- **B5** (code-analyze-mcp, session `20260309_36`): 13 analyze calls, 1 shell call, 33,046 tokens
- **A1** (native analyze, session `20260309_43`): 8 analyze calls, 7 shell calls, 27,203 tokens

Both analyzed the same codebase (lsd-rs/lsd, ~13K LOC, 52 Rust files) with the same task
(module map, data flow, dependency hubs, change proposal). Tool responses were extracted from
the goose sessions database and compared by output mode.

## Findings by Output Mode

### Overview Mode

Both tools produce similar output for directory overview. Not a significant contributor to the
token gap.

| Property | Native | code-analyze-mcp |
|----------|:------:|:----------------:|
| Output size | 1,533 chars | 1,652 chars |
| Format | `flags/ [LOC]` per file | `flags/ [LOC, F, C]` per file |
| Paths | Relative | Relative |

code-analyze-mcp includes function and class counts per file (`[604L, 52F, 2C]` vs `[604L]`),
adding ~7% more characters. This is useful information worth the cost.

### File Details Mode

Per-file output is comparable. Minor format differences, no major verbosity gap.

**Native analyze on core.rs (613 chars):**
```
core.rs [201L, 5F, 2C]

C: Core:23{flags,icons,colors,git_theme,sorters} Core:31(impl)
F: Core.new(mut flags: Flags)->Self:32•4 Core.run(self, paths: Vec<PathBuf>)->ExitCode:97 Core.fetch(&self, paths: Vec<PathBuf>)->(Vec<Meta>, ExitCode):105 Core.sort(&self, metas: &mut Vec<Meta>):170 Core.display(&self, metas: &[Meta]):180
I: crate::color::Colors; crate::display; crate::flags; crate::git::GitCache; crate::icon::Icons; crate::meta::Meta; crate; std::path::PathBuf; std::io; std::os::unix::io::AsRawFd; terminal_size::terminal_size
```

**code-analyze-mcp on core.rs (565 chars):**
```
FILE: /Users/hugues.clouatre/git/lsd-rs/lsd/src/core.rs(201L, 5F, 1C, 24I)
C:
  Core:23
F:
  new((mut flags: Flags)) -> Self :32-95•4, run((self, paths: Vec<PathBuf>)) -> ExitCode :97-103
  fetch((&self, paths: Vec<PathBuf>)) -> (Vec<Meta>, ExitCode) :105-168
  sort((&self, metas: &mut Vec<Meta>)) :170-178, display((&self, metas: &[Meta])) :180-200
I:
  crate(6)
  crate::color(1)
  crate::flags(8)
  ...
```

Key differences:

| Property | Native | code-analyze-mcp |
|----------|--------|-----------------|
| Path | Relative (`core.rs`) | Absolute (`/Users/.../src/core.rs`) |
| Class detail | Fields + impl blocks (`Core:23{flags,...} Core:31(impl)`) | Line number only (`Core:23`) |
| Function lines | Start only (`:32`) | Range (`:32-95`) |
| Imports | Flat with full paths | Grouped by module with counts |

Native includes struct fields in the class line, which is denser but informative. Our tool
uses absolute paths (+40 chars per header). Overall, file details are similarly compact; the
absolute path is the main waste.

### Symbol Focus Mode: The Primary Verbosity Driver

This is where the gap explodes. Comparing `from_path` focus queries:

| Property | Native | code-analyze-mcp | Delta |
|----------|:------:|:----------------:|:-----:|
| Total chars | 14,680 | 17,028 | +16% |
| Total lines | 400 | 604 | +51% |
| Caller lines | 84 | 68 | -19% |
| Callee lines | 306 | 528 | **+73%** |
| Paths | Relative | Absolute | +40 chars/line |

#### Callee format comparison

**Native (tree-indented, 306 lines):**
```
FOCUS: from_path (2 defs, 76 refs)

DEF theme.rs:from_path:42
DEF meta/mod.rs:from_path:261

OUT:
  meta/mod.rs:from_path:261 -> app.rs:from:272
  meta/mod.rs:from_path:261 -> color.rs:new:184
    -> config_file.rs:default:198
    -> flags/blocks.rs:default:179
    -> flags/icons.rs:default:158
```

Depth-2 callees are indented under their parent. One `new` line, children below.

**code-analyze-mcp (flat per-line, 528 lines):**
```
FOCUS: from_path
DEPTH: 2
DEFINED:
  /Users/hugues.clouatre/git/lsd-rs/lsd/src/theme.rs:42
  /Users/hugues.clouatre/git/lsd-rs/lsd/src/meta/mod.rs:261
CALLEES:
  from_path -> new
  from_path -> as_path
  from_path -> metadata
  ...70 direct callee lines...
  new -> Self
  new -> from
  new -> String
  ...458 depth-2 callee lines...
```

Every depth-2 edge is a separate flat line. The parent name (`new`) is repeated on every child
line. No grouping, no indentation.

#### Callee section breakdown

| Category | Native lines | code-analyze-mcp lines |
|----------|:------------:|:----------------------:|
| Direct (depth-1) callees | grouped under parent | 70 flat lines |
| Depth-2 callees | 306 indented under parents | 458 flat lines |
| Total callee section | 306 | 528 |

The flat format causes 73% more lines for the same semantic content because:
1. **Parent repetition.** Each flat line repeats the parent (`new -> Self`, `new -> from`)
   instead of indenting children under one parent line.
2. **No deduplication.** `default -> default` appears 12+ times from different call sites.
3. **No grouping.** Children of the same parent are interleaved with children of other parents
   rather than clustered.

#### Caller format comparison

**Native (labeled sections):**
```
IN:
  meta/mod.rs:from_path:261 -> core.rs:fetch:105 -> core.rs:run:97
  meta/mod.rs:from_path:261 -> meta/mod.rs:recurse_into:63

IN (tests):
  meta/mod.rs:from_path:261 -> display.rs:test_display_tree_with_all:...
  ...
```

Test callers are in a separate labeled section.

**code-analyze-mcp (mixed):**
```
CALLERS:
  fetch <- run <- fetch <- from_path
  test_display_tree_with_all <- from_path
  test_tree_align_subfolder <- from_path
  ...
```

Test and production callers are interleaved. 50 of 68 caller lines (74%) are test functions.

#### Header comparison

| Property | Native | code-analyze-mcp |
|----------|--------|-----------------|
| Header | `FOCUS: from_path (2 defs, 76 refs)` | `FOCUS: from_path` |
| Depth | (implicit from output) | `DEPTH: 2` |

Native provides an instant structural overview (how many definitions, how widely referenced)
before the detail. Ours provides no summary counts.

## Aggregate Response Size

Total tool response characters across the full session:

| Category | A1 (native) | B5 (code-analyze-mcp) |
|----------|:-----------:|:---------------------:|
| Analyze responses | 22,795 chars (8 calls) | 42,228 chars (13 calls) |
| Shell responses | 13,237 chars (7 calls) | 0 chars (0 structural) |
| Other (write/mkdir) | 144 chars | 2,078 chars |
| **Total** | **36,176 chars** | **44,306 chars** |
| Per-analyze-call avg | 2,849 chars | 3,248 chars |

code-analyze-mcp's per-call average is 14% higher. But the real gap is that B5 made 13 analyze
calls (producing 42K chars of analyze output) while A1 made 8 analyze calls (23K chars) plus 7
shell calls (13K chars of targeted file reads). The rg-blocking constraint eliminated shell
calls but pushed the agent into more analyze calls to compensate.

### Focus query cost

The largest single responses:

| Query | Native | code-analyze-mcp |
|-------|:------:|:----------------:|
| `from_path` focus | 14,680 chars | 17,028 chars |
| `grid` focus | 1,072 chars | 10,762 chars |
| Total focus output | 15,752 chars | 27,790 chars |

The B5 agent made two focus queries (from_path + grid) totaling 27.8K chars. The A1 agent made
one focus query (from_path only, 14.7K chars) and supplemented with 7 targeted shell reads
(13.2K chars of specific function bodies). Same total information consumed, different
distribution.

## What the LLM Actually Uses

Verified across all 10 v5 benchmark runs (both conditions):

- **Zero reports reference depth-2 callee names.** No run cites `new -> Self`, `metadata -> len`,
  `default -> default`, or any other depth-2 edge. The LLM derives data flow and hub analysis
  from overview (import counts) and file_details (function signatures).

- **All reports use relative paths.** Every report writes `meta/mod.rs`, `core.rs`, `display.rs`.
  The LLM mentally strips the absolute prefix.

- **Zero reports cite test caller function names.** Reports reference production chains
  (`fetch <- run <- from_path`) for data flow, never test callers.

- **Change proposals reference file_details patterns.** All 10 runs cite "follow meta/size.rs
  pattern" or similar. This comes from file_details mode, not from focus callee chains.

- **Hub dependency counts come from imports.** The `inbound_deps` and `outbound_deps` in
  cross_module_hubs are derived from the I: (imports) section of file_details, not from focus
  caller/callee counts.

## Compaction Opportunities

Ranked by estimated savings, all lossless (no information the LLM uses is removed):

| # | Change | Est. savings | Origin |
|---|--------|-------------|--------|
| #129 | Relative paths in all modes | ~15% of focus output | Goose comparison: native uses relative paths throughout |
| #130 | Tree-indent callees | ~40% of callee section | Goose comparison: native groups depth-2 under parents |
| #132 | Separate test callers | ~10% of caller section | Goose comparison: native has `IN (tests):` section; rtk noise-filtering principle |
| #133 | Summary header with counts | Signal improvement | Goose comparison: native has `(2 defs, 76 refs)` header |
| #134 | Deduplicate callee chains | ~15% of callee section | rtk deduplication pattern: count occurrences instead of listing each |
| #136 | Cap depth-2 to top N by frequency | Contingency | If lossless changes insufficient; bounds the query, not the display |

See #131 (closed) for why conditional collapsing via a `summary` parameter was rejected: the
MCP protocol has no progressive disclosure mechanism to let the LLM recover hidden information.

## External Reference: rtk (rtk-ai/rtk)

[rtk](https://github.com/rtk-ai/rtk) is a CLI proxy that compresses command output for LLM
consumption (60-90% token savings). Two of its patterns directly apply:

1. **Noise filtering** (`tree.rs`, `ls.rs`): Auto-exclude directories like `node_modules`,
   `.git`, `target` from listings. Analogous to filtering test callers from the CALLERS
   section: test functions are structural noise for cross-module research tasks.

2. **Deduplication with counts** (`summary.rs`, `log_cmd.rs`): Count errors/warnings instead
   of listing each occurrence. Analogous to collapsing `default -> default (x12)` instead of
   12 separate lines.

Two other rtk principles (aggressive filtering by default, smart truncation with "N lines
omitted") initially informed #131, but were not applicable because MCP tools cannot hint "call
again with different parameters" in a protocol-guaranteed way.
