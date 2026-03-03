# Ground Truth: bat Data Flow

## Module Map (Structural Accuracy)

The `bat` codebase has these key modules in `src/`:

| Module | Role |
|--------|------|
| `lib.rs` | Re-exports public API types |
| `controller.rs` | Orchestrates file processing pipeline |
| `printer.rs` | Renders lines to output (trait + 2 impls) |
| `decorations.rs` | Line decorations (numbers, git changes, grid) |
| `output.rs` | Output destination (stdout, pager) |
| `input.rs` | Input abstraction (files, stdin, readers) |
| `config.rs` | Configuration struct aggregating all options |
| `assets.rs` | Syntax/theme loading (syntect integration) |
| `diff.rs` | Git diff computation |
| `style.rs` | Style component flags |
| `theme.rs` | Theme selection and color scheme detection |
| `syntax_mapping.rs` | File-to-syntax mapping rules |
| `line_range.rs` | Line range filtering |
| `preprocessor.rs` | Tab expansion, ANSI stripping, nonprintable replacement |
| `terminal.rs` | Terminal escape code generation |
| `vscreen.rs` | Virtual screen / ANSI escape sequence parsing |
| `wrapping.rs` | Line wrapping modes |
| `paging.rs` | Paging mode enum |
| `less.rs` | Less version detection |
| `lessopen.rs` | LESSOPEN preprocessor support |
| `pretty_printer.rs` | High-level builder API (library consumers) |
| `bin/bat/main.rs` | CLI entry point |
| `bin/bat/app.rs` | Argument parsing |
| `bin/bat/config.rs` | Config file loading |

**Scoring:** A complete answer identifies at least the core pipeline modules (controller, printer, decorations, output, input, config, assets, preprocessor). Partial credit for listing files without understanding roles.

## Data Flow (Cross-Module Tracing)

### Entry Point
`bin/bat/main.rs::run_controller()` -> creates `Controller::new(&config, &assets)` -> calls `controller.run(inputs, None)`

### Pipeline
1. **main.rs** parses CLI args via `App` (clap), builds `Config` struct
2. **Controller::run()** creates `OutputType` (stdout or pager) from `output.rs`, gets `OutputHandle`
3. For each input, **Controller::run()** calls `Controller::print_input()`:
   - Opens input via `InputReader` from `input.rs`
   - Computes git diff via `diff::get_git_diff()` (if enabled)
   - Creates printer: `InteractivePrinter::new()` or `SimplePrinter::new()`
   - Calls `Controller::print_file()` with the printer
4. **Controller::print_file()** calls:
   - `printer.print_header()` -> writes file name, language info
   - `Controller::print_file_ranges()` -> iterates lines, calls `printer.print_line()` for each
   - `printer.print_footer()` -> writes closing grid
5. **InteractivePrinter::print_line()** (the core rendering):
   - Runs decorations (line numbers, git changes, grid border) via `Decoration` trait
   - Applies syntax highlighting via `syntect` (from `assets.rs`)
   - Preprocesses content: `expand_tabs()`, `replace_nonprintable()`, `strip_ansi()` from `preprocessor.rs`
   - Converts to terminal escapes via `terminal::as_terminal_escaped()`
   - Handles line wrapping via `wrapping.rs` logic
   - Writes to `OutputHandle`

### Key Types Crossing Module Boundaries
- `Config` (config.rs -> controller, printer, pretty_printer)
- `HighlightingAssets` (assets.rs -> controller, printer)
- `Input` / `OpenedInput` / `InputReader` (input.rs -> controller, printer, assets)
- `OutputType` / `OutputHandle` (output.rs -> controller, printer)
- `LineChanges` (diff.rs -> controller, printer, decorations)
- `Decoration` trait (decorations.rs -> printer)
- `StyleComponents` (style.rs -> config, printer)
- `AnsiStyle` / `EscapeSequence` (vscreen.rs -> printer)
- `Printer` trait (printer.rs -> controller)

**Scoring:** A complete answer traces the full pipeline from main -> Controller -> Printer -> Output with the key types. Must identify the Printer trait and its two implementations. Must identify the Decoration system.

## HTML Output Approach (Approach Quality)

### Recommended Approach: New Printer Implementation

The cleanest approach is to implement a new `HtmlPrinter` that implements the `Printer` trait:

1. **Add `HtmlPrinter` struct** in a new `html_printer.rs` (or in `printer.rs`)
   - Implements `Printer` trait: `print_header`, `print_footer`, `print_snip`, `print_line`
   - Generates HTML tags instead of terminal escape codes
   - Skips `Decoration` system (or adapts it for HTML classes)

2. **Add output format to Config**
   - New enum variant in config or a new field: `output_format: OutputFormat`
   - `OutputFormat::Terminal` (default) | `OutputFormat::Html`

3. **Controller selects printer based on format**
   - In `Controller::print_input()`, choose `HtmlPrinter` when format is HTML
   - The `Printer` trait already abstracts this; no changes to `Controller::print_file()`

4. **Minimal changes:**
   - `printer.rs` or new `html_printer.rs`: new struct + trait impl
   - `config.rs`: add output format field
   - `controller.rs`: add printer selection branch
   - `bin/bat/main.rs` or `clap_app.rs`: add CLI flag

**Why this is elegant:** The `Printer` trait already exists as the abstraction point. `Controller::print_file()` takes `&mut dyn Printer`, so a new implementation slots in without changing the orchestration layer.

**Scoring:** An answer that identifies the Printer trait as the extension point and proposes a new implementation scores 3. An answer that proposes modifying InteractivePrinter or adding HTML logic inline scores 1-2.
