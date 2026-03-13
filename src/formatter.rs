//! Output formatting for analysis results across different modes.
//!
//! Formats semantic analysis, call graphs, and directory structures into human-readable text.
//! Handles multiline wrapping, pagination, and summary generation.

use crate::graph::CallChain;
use crate::graph::CallGraph;
use crate::pagination::PaginationMode;
use crate::test_detection::is_test_file;
use crate::traversal::WalkEntry;
use crate::types::{ClassInfo, FileInfo, FunctionInfo, ImportInfo, SemanticAnalysis};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::Path;
use thiserror::Error;
use tracing::instrument;

const MULTILINE_THRESHOLD: usize = 10;

/// Format a list of function signatures wrapped at 100 characters with bullet annotation.
fn format_function_list_wrapped<'a>(
    functions: impl Iterator<Item = &'a crate::types::FunctionInfo>,
    call_frequency: &std::collections::HashMap<String, usize>,
) -> String {
    let mut output = String::new();
    let mut line = String::from("  ");
    for (i, func) in functions.enumerate() {
        let mut call_marker = func.compact_signature();

        if let Some(&count) = call_frequency.get(&func.name)
            && count > 3
        {
            call_marker.push_str(&format!("\u{2022}{}", count));
        }

        if i == 0 {
            line.push_str(&call_marker);
        } else if line.len() + call_marker.len() + 2 > 100 {
            output.push_str(&line);
            output.push('\n');
            let mut new_line = String::with_capacity(2 + call_marker.len());
            new_line.push_str("  ");
            new_line.push_str(&call_marker);
            line = new_line;
        } else {
            line.push_str(", ");
            line.push_str(&call_marker);
        }
    }
    if !line.trim().is_empty() {
        output.push_str(&line);
        output.push('\n');
    }
    output
}

/// Build a bracket string for file info (line count, function count, class count).
/// Returns None if all counts are zero, otherwise returns "[42L, 7F, 2C]" format.
fn format_file_info_parts(line_count: usize, fn_count: usize, cls_count: usize) -> Option<String> {
    let mut parts = Vec::new();
    if line_count > 0 {
        parts.push(format!("{}L", line_count));
    }
    if fn_count > 0 {
        parts.push(format!("{}F", fn_count));
    }
    if cls_count > 0 {
        parts.push(format!("{}C", cls_count));
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!("[{}]", parts.join(", ")))
    }
}

/// Strip a base path from a Path, returning a relative path or the original on failure.
fn strip_base_path(path: &Path, base_path: Option<&Path>) -> String {
    match base_path {
        Some(base) => {
            if let Ok(rel_path) = path.strip_prefix(base) {
                rel_path.display().to_string()
            } else {
                path.display().to_string()
            }
        }
        None => path.display().to_string(),
    }
}

#[derive(Debug, Error)]
pub enum FormatterError {
    #[error("Graph error: {0}")]
    GraphError(#[from] crate::graph::GraphError),
}

/// Format directory structure analysis results.
#[instrument(skip_all)]
pub fn format_structure(
    entries: &[WalkEntry],
    analysis_results: &[FileInfo],
    max_depth: Option<u32>,
    _base_path: Option<&Path>,
) -> String {
    let mut output = String::new();

    // Build a map of path -> analysis for quick lookup
    let analysis_map: HashMap<String, &FileInfo> = analysis_results
        .iter()
        .map(|a| (a.path.clone(), a))
        .collect();

    // Partition files into production and test
    let (prod_files, test_files): (Vec<_>, Vec<_>) =
        analysis_results.iter().partition(|a| !a.is_test);

    // Calculate totals
    let total_loc: usize = analysis_results.iter().map(|a| a.line_count).sum();
    let total_functions: usize = analysis_results.iter().map(|a| a.function_count).sum();
    let total_classes: usize = analysis_results.iter().map(|a| a.class_count).sum();

    // Count files by language and calculate percentages
    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    for analysis in analysis_results {
        *lang_counts.entry(analysis.language.clone()).or_insert(0) += 1;
    }
    let total_files = analysis_results.len();

    // Leading summary line with totals
    let primary_lang = lang_counts
        .iter()
        .max_by_key(|&(_, count)| count)
        .map(|(name, count)| {
            let percentage = if total_files > 0 {
                (*count * 100) / total_files
            } else {
                0
            };
            format!("{} {}%", name, percentage)
        })
        .unwrap_or_else(|| "unknown 0%".to_string());

    output.push_str(&format!(
        "{} files, {}L, {}F, {}C ({})\n",
        total_files, total_loc, total_functions, total_classes, primary_lang
    ));

    // SUMMARY block
    output.push_str("SUMMARY:\n");
    let depth_label = match max_depth {
        Some(n) if n > 0 => format!(" (max_depth={})", n),
        _ => String::new(),
    };
    output.push_str(&format!(
        "Shown: {} files ({} prod, {} test), {}L, {}F, {}C{}\n",
        total_files,
        prod_files.len(),
        test_files.len(),
        total_loc,
        total_functions,
        total_classes,
        depth_label
    ));

    if !lang_counts.is_empty() {
        output.push_str("Languages: ");
        let mut langs: Vec<_> = lang_counts.iter().collect();
        langs.sort_by_key(|&(name, _)| name);
        let lang_strs: Vec<String> = langs
            .iter()
            .map(|(name, count)| {
                let percentage = if total_files > 0 {
                    (**count * 100) / total_files
                } else {
                    0
                };
                format!("{} ({}%)", name, percentage)
            })
            .collect();
        output.push_str(&lang_strs.join(", "));
        output.push('\n');
    }

    output.push('\n');

    // PATH block - tree structure (production files only)
    output.push_str("PATH [LOC, FUNCTIONS, CLASSES]\n");

    for entry in entries {
        // Skip the root directory itself
        if entry.depth == 0 {
            continue;
        }

        // Calculate indentation
        let indent = "  ".repeat(entry.depth - 1);

        // Get just the filename/dirname
        let name = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        // For files, append analysis info
        if !entry.is_dir {
            if let Some(analysis) = analysis_map.get(&entry.path.display().to_string()) {
                // Skip test files in production section
                if analysis.is_test {
                    continue;
                }

                if let Some(info_str) = format_file_info_parts(
                    analysis.line_count,
                    analysis.function_count,
                    analysis.class_count,
                ) {
                    output.push_str(&format!("{}{} {}\n", indent, name, info_str));
                } else {
                    output.push_str(&format!("{}{}\n", indent, name));
                }
            }
            // Skip files not in analysis_map (binary/unreadable files)
        } else {
            output.push_str(&format!("{}{}/\n", indent, name));
        }
    }

    // TEST FILES section (if any test files exist)
    if !test_files.is_empty() {
        output.push_str("\nTEST FILES [LOC, FUNCTIONS, CLASSES]\n");

        for entry in entries {
            // Skip the root directory itself
            if entry.depth == 0 {
                continue;
            }

            // Calculate indentation
            let indent = "  ".repeat(entry.depth - 1);

            // Get just the filename/dirname
            let name = entry
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");

            // For files, append analysis info
            if !entry.is_dir
                && let Some(analysis) = analysis_map.get(&entry.path.display().to_string())
            {
                // Only show test files in test section
                if !analysis.is_test {
                    continue;
                }

                if let Some(info_str) = format_file_info_parts(
                    analysis.line_count,
                    analysis.function_count,
                    analysis.class_count,
                ) {
                    output.push_str(&format!("{}{} {}\n", indent, name, info_str));
                } else {
                    output.push_str(&format!("{}{}\n", indent, name));
                }
            }
        }
    }

    output
}

/// Format file-level semantic analysis results.
#[instrument(skip_all)]
pub fn format_file_details(
    path: &str,
    analysis: &SemanticAnalysis,
    line_count: usize,
    is_test: bool,
    base_path: Option<&Path>,
) -> String {
    let mut output = String::new();

    // FILE: header with counts, prepend [TEST] if applicable
    let display_path = strip_base_path(Path::new(path), base_path);
    if is_test {
        output.push_str(&format!(
            "FILE [TEST] {}({}L, {}F, {}C, {}I)\n",
            display_path,
            line_count,
            analysis.functions.len(),
            analysis.classes.len(),
            analysis.imports.len()
        ));
    } else {
        output.push_str(&format!(
            "FILE: {}({}L, {}F, {}C, {}I)\n",
            display_path,
            line_count,
            analysis.functions.len(),
            analysis.classes.len(),
            analysis.imports.len()
        ));
    }

    // C: section with classes
    output.push_str(&format_classes_section(&analysis.classes));

    // F: section with functions, parameters, return types and call frequency
    if !analysis.functions.is_empty() {
        output.push_str("F:\n");
        output.push_str(&format_function_list_wrapped(
            analysis.functions.iter(),
            &analysis.call_frequency,
        ));
    }

    // I: section with imports grouped by module
    output.push_str(&format_imports_section(&analysis.imports));

    output
}

/// Format chains as a tree-indented output, grouped by depth-1 symbol.
/// Groups chains by their first symbol (depth-1), deduplicates and sorts depth-2 children,
/// then renders with 2-space indentation using the provided arrow.
/// focus_symbol is the name of the depth-0 symbol (focus point) to prepend on depth-1 lines.
///
/// Indentation rules:
/// - Depth-1: `  {focus} {arrow} {parent}` (2-space indent)
/// - Depth-2: `    {arrow} {child}` (4-space indent)
/// - Empty:   `  (none)` (2-space indent)
fn format_chains_as_tree(chains: &[(&str, &str)], arrow: &str, focus_symbol: &str) -> String {
    use std::collections::BTreeMap;

    if chains.is_empty() {
        return "  (none)\n".to_string();
    }

    let mut output = String::new();

    // Group chains by depth-1 symbol, counting duplicate children
    let mut groups: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for (parent, child) in chains {
        // Only count non-empty children
        if !child.is_empty() {
            *groups
                .entry(parent.to_string())
                .or_default()
                .entry(child.to_string())
                .or_insert(0) += 1;
        } else {
            // Ensure parent is in groups even if no children
            groups.entry(parent.to_string()).or_default();
        }
    }

    // Render grouped tree
    for (parent, children) in groups {
        let _ = writeln!(output, "  {} {} {}", focus_symbol, arrow, parent);
        // Sort children by count descending, then alphabetically
        let mut sorted: Vec<_> = children.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        for (child, count) in sorted {
            if count > 1 {
                let _ = writeln!(output, "    {} {} (x{})", arrow, child, count);
            } else {
                let _ = writeln!(output, "    {} {}", arrow, child);
            }
        }
    }

    output
}

/// Format focused symbol analysis with call graph.
#[instrument(skip_all)]
pub fn format_focused(
    graph: &CallGraph,
    symbol: &str,
    follow_depth: u32,
    base_path: Option<&Path>,
) -> Result<String, FormatterError> {
    let mut output = String::new();

    // Compute all counts BEFORE output begins
    let def_count = graph.definitions.get(symbol).map_or(0, |d| d.len());
    let incoming_chains = graph.find_incoming_chains(symbol, follow_depth)?;
    let outgoing_chains = graph.find_outgoing_chains(symbol, follow_depth)?;

    // Partition incoming_chains into production and test callers
    let (prod_chains, test_chains): (Vec<_>, Vec<_>) =
        incoming_chains.clone().into_iter().partition(|chain| {
            chain
                .chain
                .first()
                .is_none_or(|(name, path, _)| !is_test_file(path) && !name.starts_with("test_"))
        });

    // Count unique callers
    let callers_count = prod_chains
        .iter()
        .filter_map(|chain| chain.chain.first().map(|(p, _, _)| p))
        .collect::<std::collections::HashSet<_>>()
        .len();

    // Count unique callees
    let callees_count = outgoing_chains
        .iter()
        .filter_map(|chain| chain.chain.first().map(|(p, _, _)| p))
        .collect::<std::collections::HashSet<_>>()
        .len();

    // FOCUS section - with inline counts
    output.push_str(&format!(
        "FOCUS: {} ({} defs, {} callers, {} callees)\n",
        symbol, def_count, callers_count, callees_count
    ));

    // DEPTH section
    output.push_str(&format!("DEPTH: {}\n", follow_depth));

    // DEFINED section - show where the symbol is defined
    if let Some(definitions) = graph.definitions.get(symbol) {
        output.push_str("DEFINED:\n");
        for (path, line) in definitions {
            output.push_str(&format!(
                "  {}:{}\n",
                strip_base_path(path, base_path),
                line
            ));
        }
    } else {
        output.push_str("DEFINED: (not found)\n");
    }

    // CALLERS section - who calls this symbol
    output.push_str("CALLERS:\n");

    // Render production callers
    let prod_refs: Vec<_> = prod_chains
        .iter()
        .filter_map(|chain| {
            if chain.chain.len() >= 2 {
                Some((chain.chain[0].0.as_str(), chain.chain[1].0.as_str()))
            } else if chain.chain.len() == 1 {
                Some((chain.chain[0].0.as_str(), ""))
            } else {
                None
            }
        })
        .collect();

    if prod_refs.is_empty() {
        output.push_str("  (none)\n");
    } else {
        output.push_str(&format_chains_as_tree(&prod_refs, "<-", symbol));
    }

    // Render test callers summary if any
    if !test_chains.is_empty() {
        let mut test_files: Vec<_> = test_chains
            .iter()
            .filter_map(|chain| {
                chain
                    .chain
                    .first()
                    .map(|(_, path, _)| path.to_string_lossy().into_owned())
            })
            .collect();
        test_files.sort();
        test_files.dedup();

        // Strip base path for display
        let display_files: Vec<_> = test_files
            .iter()
            .map(|f| strip_base_path(Path::new(f), base_path))
            .collect();

        let file_list = display_files.join(", ");
        output.push_str(&format!(
            "CALLERS (test): {} test functions (in {})\n",
            test_chains.len(),
            file_list
        ));
    }

    // CALLEES section - what this symbol calls
    output.push_str("CALLEES:\n");
    let outgoing_refs: Vec<_> = outgoing_chains
        .iter()
        .filter_map(|chain| {
            if chain.chain.len() >= 2 {
                Some((chain.chain[0].0.as_str(), chain.chain[1].0.as_str()))
            } else if chain.chain.len() == 1 {
                Some((chain.chain[0].0.as_str(), ""))
            } else {
                None
            }
        })
        .collect();

    if outgoing_refs.is_empty() {
        output.push_str("  (none)\n");
    } else {
        output.push_str(&format_chains_as_tree(&outgoing_refs, "->", symbol));
    }

    // STATISTICS section
    output.push_str("STATISTICS:\n");
    let incoming_count = prod_refs
        .iter()
        .map(|(p, _)| p)
        .collect::<std::collections::HashSet<_>>()
        .len();
    let outgoing_count = outgoing_refs
        .iter()
        .map(|(p, _)| p)
        .collect::<std::collections::HashSet<_>>()
        .len();
    output.push_str(&format!("  Incoming calls: {}\n", incoming_count));
    output.push_str(&format!("  Outgoing calls: {}\n", outgoing_count));

    // FILES section - collect unique files from production chains
    let mut files = HashSet::new();
    for chain in &prod_chains {
        for (_, path, _) in &chain.chain {
            files.insert(path.clone());
        }
    }
    for chain in &outgoing_chains {
        for (_, path, _) in &chain.chain {
            files.insert(path.clone());
        }
    }
    if let Some(definitions) = graph.definitions.get(symbol) {
        for (path, _) in definitions {
            files.insert(path.clone());
        }
    }

    // Partition files into production and test
    let (prod_files, test_files): (Vec<_>, Vec<_>) =
        files.into_iter().partition(|path| !is_test_file(path));

    output.push_str("FILES:\n");
    if prod_files.is_empty() && test_files.is_empty() {
        output.push_str("  (none)\n");
    } else {
        // Show production files first
        if !prod_files.is_empty() {
            let mut sorted_files = prod_files;
            sorted_files.sort();
            for file in sorted_files {
                output.push_str(&format!("  {}\n", strip_base_path(&file, base_path)));
            }
        }

        // Show test files in separate subsection
        if !test_files.is_empty() {
            output.push_str("  TEST FILES:\n");
            let mut sorted_files = test_files;
            sorted_files.sort();
            for file in sorted_files {
                output.push_str(&format!("    {}\n", strip_base_path(&file, base_path)));
            }
        }
    }

    Ok(output)
}

/// Format a compact summary of focused symbol analysis.
/// Used when output would exceed the size threshold or when explicitly requested.
#[instrument(skip_all)]
pub fn format_focused_summary(
    graph: &CallGraph,
    symbol: &str,
    follow_depth: u32,
    base_path: Option<&Path>,
) -> Result<String, FormatterError> {
    let mut output = String::new();

    // Compute all counts BEFORE output begins
    let def_count = graph.definitions.get(symbol).map_or(0, |d| d.len());
    let incoming_chains = graph.find_incoming_chains(symbol, follow_depth)?;
    let outgoing_chains = graph.find_outgoing_chains(symbol, follow_depth)?;

    // Partition incoming_chains into production and test callers
    let (prod_chains, test_chains): (Vec<_>, Vec<_>) =
        incoming_chains.into_iter().partition(|chain| {
            chain
                .chain
                .first()
                .is_none_or(|(name, path, _)| !is_test_file(path) && !name.starts_with("test_"))
        });

    // Count unique production callers
    let callers_count = prod_chains
        .iter()
        .filter_map(|chain| chain.chain.first().map(|(p, _, _)| p))
        .collect::<std::collections::HashSet<_>>()
        .len();

    // Count unique callees
    let callees_count = outgoing_chains
        .iter()
        .filter_map(|chain| chain.chain.first().map(|(p, _, _)| p))
        .collect::<std::collections::HashSet<_>>()
        .len();

    // FOCUS header
    output.push_str(&format!(
        "FOCUS: {} ({} defs, {} callers, {} callees)\n",
        symbol, def_count, callers_count, callees_count
    ));

    // DEPTH line
    output.push_str(&format!("DEPTH: {}\n", follow_depth));

    // DEFINED section
    if let Some(definitions) = graph.definitions.get(symbol) {
        output.push_str("DEFINED:\n");
        for (path, line) in definitions {
            output.push_str(&format!(
                "  {}:{}\n",
                strip_base_path(path, base_path),
                line
            ));
        }
    } else {
        output.push_str("DEFINED: (not found)\n");
    }

    // CALLERS (production, top 10 by frequency)
    output.push_str("CALLERS (top 10):\n");
    if prod_chains.is_empty() {
        output.push_str("  (none)\n");
    } else {
        // Collect caller names with their file paths (from chain.chain.first())
        let mut caller_freq: std::collections::HashMap<String, (usize, String)> =
            std::collections::HashMap::new();
        for chain in &prod_chains {
            if let Some((name, path, _)) = chain.chain.first() {
                let file_path = strip_base_path(path, base_path);
                caller_freq
                    .entry(name.clone())
                    .and_modify(|(count, _)| *count += 1)
                    .or_insert((1, file_path));
            }
        }

        // Sort by frequency descending, take top 10
        let mut sorted_callers: Vec<_> = caller_freq.into_iter().collect();
        sorted_callers.sort_by(|a, b| b.1.0.cmp(&a.1.0));

        for (name, (_, file_path)) in sorted_callers.into_iter().take(10) {
            output.push_str(&format!("  {} {}\n", name, file_path));
        }
    }

    // CALLERS (test) - summary only
    if !test_chains.is_empty() {
        let mut test_files: Vec<_> = test_chains
            .iter()
            .filter_map(|chain| {
                chain
                    .chain
                    .first()
                    .map(|(_, path, _)| path.to_string_lossy().into_owned())
            })
            .collect();
        test_files.sort();
        test_files.dedup();

        output.push_str(&format!(
            "CALLERS (test): {} test functions (in {} files)\n",
            test_chains.len(),
            test_files.len()
        ));
    }

    // CALLEES (top 10 by frequency)
    output.push_str("CALLEES (top 10):\n");
    if outgoing_chains.is_empty() {
        output.push_str("  (none)\n");
    } else {
        // Collect callee names and count frequency
        let mut callee_freq: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for chain in &outgoing_chains {
            if let Some((name, _, _)) = chain.chain.first() {
                *callee_freq.entry(name.clone()).or_insert(0) += 1;
            }
        }

        // Sort by frequency descending, take top 10
        let mut sorted_callees: Vec<_> = callee_freq.into_iter().collect();
        sorted_callees.sort_by(|a, b| b.1.cmp(&a.1));

        for (name, _) in sorted_callees.into_iter().take(10) {
            output.push_str(&format!("  {}\n", name));
        }
    }

    // SUGGESTION section
    output.push_str("SUGGESTION:\n");
    output.push_str("Use summary=false with force=true for full output\n");

    Ok(output)
}

/// Format a compact summary for large directory analysis results.
/// Used when output would exceed the size threshold or when explicitly requested.
#[instrument(skip_all)]
pub fn format_summary(
    entries: &[WalkEntry],
    analysis_results: &[FileInfo],
    max_depth: Option<u32>,
    _base_path: Option<&Path>,
) -> String {
    let mut output = String::new();

    // Partition files into production and test
    let (prod_files, test_files): (Vec<_>, Vec<_>) =
        analysis_results.iter().partition(|a| !a.is_test);

    // Calculate totals
    let total_loc: usize = analysis_results.iter().map(|a| a.line_count).sum();
    let total_functions: usize = analysis_results.iter().map(|a| a.function_count).sum();
    let total_classes: usize = analysis_results.iter().map(|a| a.class_count).sum();

    // Count files by language
    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    for analysis in analysis_results {
        *lang_counts.entry(analysis.language.clone()).or_insert(0) += 1;
    }
    let total_files = analysis_results.len();

    // Leading summary line with totals
    let primary_lang = lang_counts
        .iter()
        .max_by_key(|&(_, count)| count)
        .map(|(name, count)| {
            let percentage = if total_files > 0 {
                (*count * 100) / total_files
            } else {
                0
            };
            format!("{} {}%", name, percentage)
        })
        .unwrap_or_else(|| "unknown 0%".to_string());

    output.push_str(&format!(
        "{} files, {}L, {}F, {}C ({})\n",
        total_files, total_loc, total_functions, total_classes, primary_lang
    ));

    // SUMMARY block
    output.push_str("SUMMARY:\n");
    let depth_label = match max_depth {
        Some(n) if n > 0 => format!(" (max_depth={})", n),
        _ => String::new(),
    };
    output.push_str(&format!(
        "{} files ({} prod, {} test), {}L, {}F, {}C{}\n",
        total_files,
        prod_files.len(),
        test_files.len(),
        total_loc,
        total_functions,
        total_classes,
        depth_label
    ));

    if !lang_counts.is_empty() {
        output.push_str("Languages: ");
        let mut langs: Vec<_> = lang_counts.iter().collect();
        langs.sort_by_key(|&(name, _)| name);
        let lang_strs: Vec<String> = langs
            .iter()
            .map(|(name, count)| {
                let percentage = if total_files > 0 {
                    (**count * 100) / total_files
                } else {
                    0
                };
                format!("{} ({}%)", name, percentage)
            })
            .collect();
        output.push_str(&lang_strs.join(", "));
        output.push('\n');
    }

    output.push('\n');

    // STRUCTURE (depth 1) block
    output.push_str("STRUCTURE (depth 1):\n");

    // Build a map of path -> analysis for quick lookup
    let analysis_map: HashMap<String, &FileInfo> = analysis_results
        .iter()
        .map(|a| (a.path.clone(), a))
        .collect();

    // Collect depth-1 entries (directories and files at depth 1)
    let mut depth1_entries: Vec<&WalkEntry> = entries.iter().filter(|e| e.depth == 1).collect();
    depth1_entries.sort_by(|a, b| a.path.cmp(&b.path));

    for entry in depth1_entries {
        let name = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        if entry.is_dir {
            // For directories, aggregate stats from all files under this directory
            let dir_path_str = entry.path.display().to_string();
            let files_in_dir: Vec<&FileInfo> = analysis_results
                .iter()
                .filter(|f| f.path.starts_with(&dir_path_str))
                .collect();

            if !files_in_dir.is_empty() {
                let dir_file_count = files_in_dir.len();
                let dir_loc: usize = files_in_dir.iter().map(|f| f.line_count).sum();
                let dir_functions: usize = files_in_dir.iter().map(|f| f.function_count).sum();
                let dir_classes: usize = files_in_dir.iter().map(|f| f.class_count).sum();

                output.push_str(&format!(
                    "  {}/ [{} files, {}L, {}F, {}C]\n",
                    name, dir_file_count, dir_loc, dir_functions, dir_classes
                ));
            } else {
                output.push_str(&format!("  {}/\n", name));
            }
        } else {
            // For files, show individual stats
            if let Some(analysis) = analysis_map.get(&entry.path.display().to_string()) {
                if let Some(info_str) = format_file_info_parts(
                    analysis.line_count,
                    analysis.function_count,
                    analysis.class_count,
                ) {
                    output.push_str(&format!("  {} {}\n", name, info_str));
                } else {
                    output.push_str(&format!("  {}\n", name));
                }
            }
        }
    }

    output.push('\n');

    // SUGGESTION block
    output.push_str("SUGGESTION:\n");
    output.push_str("Use a narrower path for details (e.g., analyze src/core/)\n");

    output
}

/// Format a compact summary of file details for large FileDetails output.
///
/// Returns FILE header with path/LOC/counts, top 10 functions by line span descending,
/// classes inline if <=10, import count, and suggestion block.
#[instrument(skip_all)]
pub fn format_file_details_summary(
    semantic: &SemanticAnalysis,
    path: &str,
    line_count: usize,
) -> String {
    let mut output = String::new();

    // FILE header
    output.push_str("FILE:\n");
    output.push_str(&format!("  path: {}\n", path));
    output.push_str(&format!(
        "  {}L, {}F, {}C\n",
        line_count,
        semantic.functions.len(),
        semantic.classes.len()
    ));
    output.push('\n');

    // Top 10 functions by line span (end_line - start_line) descending
    if !semantic.functions.is_empty() {
        output.push_str("TOP FUNCTIONS BY SIZE:\n");
        let mut funcs: Vec<&crate::types::FunctionInfo> = semantic.functions.iter().collect();
        let k = funcs.len().min(10);
        if k > 0 {
            funcs.select_nth_unstable_by(k.saturating_sub(1), |a, b| {
                let a_span = a.end_line.saturating_sub(a.line);
                let b_span = b.end_line.saturating_sub(b.line);
                b_span.cmp(&a_span)
            });
            funcs[..k].sort_by(|a, b| {
                let a_span = a.end_line.saturating_sub(a.line);
                let b_span = b.end_line.saturating_sub(b.line);
                b_span.cmp(&a_span)
            });
        }

        for func in &funcs[..k] {
            let span = func.end_line.saturating_sub(func.line);
            let params = if func.parameters.is_empty() {
                String::new()
            } else {
                format!("({})", func.parameters.join(", "))
            };
            output.push_str(&format!(
                "  {}:{}: {} {} [{}L]\n",
                func.line, func.end_line, func.name, params, span
            ));
        }
        output.push('\n');
    }

    // Classes inline if <=10, else multiline with method count
    if !semantic.classes.is_empty() {
        output.push_str("CLASSES:\n");
        if semantic.classes.len() <= 10 {
            // Inline format: one class per line with method count
            for class in &semantic.classes {
                let methods_count = class.methods.len();
                output.push_str(&format!("  {}: {}M\n", class.name, methods_count));
            }
        } else {
            // Multiline format with summary
            output.push_str(&format!("  {} classes total\n", semantic.classes.len()));
            for class in semantic.classes.iter().take(5) {
                output.push_str(&format!("    {}\n", class.name));
            }
            if semantic.classes.len() > 5 {
                output.push_str(&format!(
                    "    ... and {} more\n",
                    semantic.classes.len() - 5
                ));
            }
        }
        output.push('\n');
    }

    // Import count only
    output.push_str(&format!("Imports: {}\n", semantic.imports.len()));
    output.push('\n');

    // SUGGESTION block
    output.push_str("SUGGESTION:\n");
    output.push_str("Use force=true for full output, or narrow your scope\n");

    output
}

/// Format a paginated subset of files for Overview mode.
#[instrument(skip_all)]
pub fn format_structure_paginated(
    paginated_files: &[FileInfo],
    total_files: usize,
    max_depth: Option<u32>,
    base_path: Option<&Path>,
    verbose: bool,
) -> String {
    let mut output = String::new();

    let depth_label = match max_depth {
        Some(n) if n > 0 => format!(" (max_depth={})", n),
        _ => String::new(),
    };
    output.push_str(&format!(
        "PAGINATED: showing {} of {} files{}\n\n",
        paginated_files.len(),
        total_files,
        depth_label
    ));

    let prod_files: Vec<&FileInfo> = paginated_files.iter().filter(|f| !f.is_test).collect();
    let test_files: Vec<&FileInfo> = paginated_files.iter().filter(|f| f.is_test).collect();

    if !prod_files.is_empty() {
        if verbose {
            output.push_str("FILES [LOC, FUNCTIONS, CLASSES]\n");
        }
        for file in &prod_files {
            output.push_str(&format_file_entry(file, base_path));
        }
    }

    if !test_files.is_empty() {
        if verbose {
            output.push_str("\nTEST FILES [LOC, FUNCTIONS, CLASSES]\n");
        } else if !prod_files.is_empty() {
            output.push('\n');
        }
        for file in &test_files {
            output.push_str(&format_file_entry(file, base_path));
        }
    }

    output
}

/// Format a paginated subset of functions for FileDetails mode.
/// When `verbose=true`, shows classes and imports on the first page (offset == 0) with F:/C:/I: section headers.
/// When `verbose=false` (default), omits section headers and renders one function per line with line ranges.
/// Header shows position context: `FILE: path (NL, start-end/totalF, CC, II)`.
#[instrument(skip_all)]
pub fn format_file_details_paginated(
    functions_page: &[FunctionInfo],
    total_functions: usize,
    semantic: &SemanticAnalysis,
    path: &str,
    line_count: usize,
    offset: usize,
    verbose: bool,
) -> String {
    let mut output = String::new();

    let start = offset + 1; // 1-indexed for display
    let end = offset + functions_page.len();

    output.push_str(&format!(
        "FILE: {} ({}L, {}-{}/{}F, {}C, {}I)\n",
        path,
        line_count,
        start,
        end,
        total_functions,
        semantic.classes.len(),
        semantic.imports.len(),
    ));

    // Classes section on first page for both verbose and compact modes
    if offset == 0 && !semantic.classes.is_empty() {
        output.push_str(&format_classes_section(&semantic.classes));
    }

    // Imports section only on first page in verbose mode
    if offset == 0 && verbose {
        output.push_str(&format_imports_section(&semantic.imports));
    }

    // F: section with paginated function slice
    if !functions_page.is_empty() {
        output.push_str("F:\n");
        output.push_str(&format_function_list_wrapped(
            functions_page.iter(),
            &semantic.call_frequency,
        ));
    }

    output
}

/// Parameters for `format_focused_paginated`.
pub struct FocusedPaginatedParams<'a> {
    pub paginated_chains: &'a [CallChain],
    pub total: usize,
    pub mode: PaginationMode,
    pub symbol: &'a str,
    pub prod_chains: &'a [CallChain],
    pub test_chains: &'a [CallChain],
    pub outgoing_chains: &'a [CallChain],
    pub def_count: usize,
    pub offset: usize,
    pub base_path: Option<&'a Path>,
}

/// Format a paginated subset of callers or callees for SymbolFocus mode.
/// Mode is determined by the `mode` parameter:
/// - `PaginationMode::Callers`: paginate production callers; show test callers summary and callees summary.
/// - `PaginationMode::Callees`: paginate callees; show callers summary and test callers summary.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub fn format_focused_paginated(
    paginated_chains: &[CallChain],
    total: usize,
    mode: PaginationMode,
    symbol: &str,
    prod_chains: &[CallChain],
    test_chains: &[CallChain],
    outgoing_chains: &[CallChain],
    def_count: usize,
    offset: usize,
    base_path: Option<&Path>,
    _verbose: bool,
) -> String {
    let start = offset + 1; // 1-indexed
    let end = offset + paginated_chains.len();

    let callers_count = prod_chains
        .iter()
        .filter_map(|chain| chain.chain.first().map(|(p, _, _)| p))
        .collect::<std::collections::HashSet<_>>()
        .len();

    let callees_count = outgoing_chains
        .iter()
        .filter_map(|chain| chain.chain.first().map(|(p, _, _)| p))
        .collect::<std::collections::HashSet<_>>()
        .len();

    let mut output = String::new();

    output.push_str(&format!(
        "FOCUS: {} ({} defs, {} callers, {} callees)\n",
        symbol, def_count, callers_count, callees_count
    ));

    match mode {
        PaginationMode::Callers => {
            // Paginate production callers
            output.push_str(&format!("CALLERS ({}-{} of {}):\n", start, end, total));

            let page_refs: Vec<_> = paginated_chains
                .iter()
                .filter_map(|chain| {
                    if chain.chain.len() >= 2 {
                        Some((chain.chain[0].0.as_str(), chain.chain[1].0.as_str()))
                    } else if chain.chain.len() == 1 {
                        Some((chain.chain[0].0.as_str(), ""))
                    } else {
                        None
                    }
                })
                .collect();

            if page_refs.is_empty() {
                output.push_str("  (none)\n");
            } else {
                output.push_str(&format_chains_as_tree(&page_refs, "<-", symbol));
            }

            // Test callers summary
            if !test_chains.is_empty() {
                let mut test_files: Vec<_> = test_chains
                    .iter()
                    .filter_map(|chain| {
                        chain
                            .chain
                            .first()
                            .map(|(_, path, _)| path.to_string_lossy().into_owned())
                    })
                    .collect();
                test_files.sort();
                test_files.dedup();

                let display_files: Vec<_> = test_files
                    .iter()
                    .map(|f| strip_base_path(std::path::Path::new(f), base_path))
                    .collect();

                output.push_str(&format!(
                    "CALLERS (test): {} test functions (in {})\n",
                    test_chains.len(),
                    display_files.join(", ")
                ));
            }

            // Callees summary
            let callee_names: Vec<_> = outgoing_chains
                .iter()
                .filter_map(|chain| chain.chain.first().map(|(p, _, _)| p.clone()))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            if callee_names.is_empty() {
                output.push_str("CALLEES: (none)\n");
            } else {
                output.push_str(&format!(
                    "CALLEES: {} (use cursor for callee pagination)\n",
                    callees_count
                ));
            }
        }
        PaginationMode::Callees => {
            // Callers summary
            output.push_str(&format!("CALLERS: {} production callers\n", callers_count));

            // Test callers summary
            if !test_chains.is_empty() {
                output.push_str(&format!(
                    "CALLERS (test): {} test functions\n",
                    test_chains.len()
                ));
            }

            // Paginate callees
            output.push_str(&format!("CALLEES ({}-{} of {}):\n", start, end, total));

            let page_refs: Vec<_> = paginated_chains
                .iter()
                .filter_map(|chain| {
                    if chain.chain.len() >= 2 {
                        Some((chain.chain[0].0.as_str(), chain.chain[1].0.as_str()))
                    } else if chain.chain.len() == 1 {
                        Some((chain.chain[0].0.as_str(), ""))
                    } else {
                        None
                    }
                })
                .collect();

            if page_refs.is_empty() {
                output.push_str("  (none)\n");
            } else {
                output.push_str(&format_chains_as_tree(&page_refs, "->", symbol));
            }
        }
        PaginationMode::Default => {
            unreachable!("format_focused_paginated called with PaginationMode::Default")
        }
    }

    output
}

fn format_file_entry(file: &FileInfo, base_path: Option<&Path>) -> String {
    let mut parts = Vec::new();
    if file.line_count > 0 {
        parts.push(format!("{}L", file.line_count));
    }
    if file.function_count > 0 {
        parts.push(format!("{}F", file.function_count));
    }
    if file.class_count > 0 {
        parts.push(format!("{}C", file.class_count));
    }
    let display_path = strip_base_path(Path::new(&file.path), base_path);
    if parts.is_empty() {
        format!("{}\n", display_path)
    } else {
        format!("{} [{}]\n", display_path, parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_base_path_relative() {
        let path = Path::new("/home/user/project/src/main.rs");
        let base = Path::new("/home/user/project");
        let result = strip_base_path(path, Some(base));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn test_strip_base_path_fallback_absolute() {
        let path = Path::new("/other/project/src/main.rs");
        let base = Path::new("/home/user/project");
        let result = strip_base_path(path, Some(base));
        assert_eq!(result, "/other/project/src/main.rs");
    }

    #[test]
    fn test_strip_base_path_none() {
        let path = Path::new("/home/user/project/src/main.rs");
        let result = strip_base_path(path, None);
        assert_eq!(result, "/home/user/project/src/main.rs");
    }

    #[test]
    fn test_format_file_details_summary_empty() {
        use crate::types::SemanticAnalysis;
        use std::collections::HashMap;

        let semantic = SemanticAnalysis {
            functions: vec![],
            classes: vec![],
            imports: vec![],
            references: vec![],
            call_frequency: HashMap::new(),
            calls: vec![],
            assignments: vec![],
            field_accesses: vec![],
        };

        let result = format_file_details_summary(&semantic, "src/main.rs", 100);

        // Should contain FILE header, Imports count, and SUGGESTION
        assert!(result.contains("FILE:"));
        assert!(result.contains("100L, 0F, 0C"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("Imports: 0"));
        assert!(result.contains("SUGGESTION:"));
    }

    #[test]
    fn test_format_file_details_summary_with_functions() {
        use crate::types::{ClassInfo, FunctionInfo, SemanticAnalysis};
        use std::collections::HashMap;

        let semantic = SemanticAnalysis {
            functions: vec![
                FunctionInfo {
                    name: "short".to_string(),
                    line: 10,
                    end_line: 12,
                    parameters: vec![],
                    return_type: None,
                },
                FunctionInfo {
                    name: "long_function".to_string(),
                    line: 20,
                    end_line: 50,
                    parameters: vec!["x".to_string(), "y".to_string()],
                    return_type: Some("i32".to_string()),
                },
            ],
            classes: vec![ClassInfo {
                name: "MyClass".to_string(),
                line: 60,
                end_line: 80,
                methods: vec![],
                fields: vec![],
                inherits: vec![],
            }],
            imports: vec![],
            references: vec![],
            call_frequency: HashMap::new(),
            calls: vec![],
            assignments: vec![],
            field_accesses: vec![],
        };

        let result = format_file_details_summary(&semantic, "src/lib.rs", 250);

        // Should contain FILE header with counts
        assert!(result.contains("FILE:"));
        assert!(result.contains("src/lib.rs"));
        assert!(result.contains("250L, 2F, 1C"));

        // Should contain TOP FUNCTIONS BY SIZE with longest first
        assert!(result.contains("TOP FUNCTIONS BY SIZE:"));
        let long_idx = result.find("long_function").unwrap_or(0);
        let short_idx = result.find("short").unwrap_or(0);
        assert!(
            long_idx > 0 && short_idx > 0 && long_idx < short_idx,
            "long_function should appear before short"
        );

        // Should contain classes inline
        assert!(result.contains("CLASSES:"));
        assert!(result.contains("MyClass:"));

        // Should contain import count
        assert!(result.contains("Imports: 0"));
    }
    #[test]
    fn test_format_file_info_parts_all_zero() {
        assert_eq!(format_file_info_parts(0, 0, 0), None);
    }

    #[test]
    fn test_format_file_info_parts_partial() {
        assert_eq!(
            format_file_info_parts(42, 0, 3),
            Some("[42L, 3C]".to_string())
        );
    }

    #[test]
    fn test_format_file_info_parts_all_nonzero() {
        assert_eq!(
            format_file_info_parts(100, 5, 2),
            Some("[100L, 5F, 2C]".to_string())
        );
    }

    #[test]
    fn test_format_function_list_wrapped_empty() {
        let freq = std::collections::HashMap::new();
        let result = format_function_list_wrapped(std::iter::empty(), &freq);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_function_list_wrapped_bullet_annotation() {
        use crate::types::FunctionInfo;
        use std::collections::HashMap;

        let mut freq = HashMap::new();
        freq.insert("frequent".to_string(), 5); // count > 3 should get bullet

        let funcs = vec![FunctionInfo {
            name: "frequent".to_string(),
            line: 1,
            end_line: 10,
            parameters: vec![],
            return_type: Some("void".to_string()),
        }];

        let result = format_function_list_wrapped(funcs.iter(), &freq);
        // Should contain bullet (U+2022) followed by count
        assert!(result.contains("\u{2022}5"));
    }

    #[test]
    fn test_compact_format_omits_sections() {
        use crate::types::{ClassInfo, FunctionInfo, ImportInfo, SemanticAnalysis};
        use std::collections::HashMap;

        let funcs: Vec<FunctionInfo> = (0..10)
            .map(|i| FunctionInfo {
                name: format!("fn_{}", i),
                line: i * 5 + 1,
                end_line: i * 5 + 4,
                parameters: vec![format!("x: u32")],
                return_type: Some("bool".to_string()),
            })
            .collect();
        let imports: Vec<ImportInfo> = vec![ImportInfo {
            module: "std::collections".to_string(),
            items: vec!["HashMap".to_string()],
            line: 1,
        }];
        let classes: Vec<ClassInfo> = vec![ClassInfo {
            name: "MyStruct".to_string(),
            line: 5,
            end_line: 50,
            methods: vec![],
            fields: vec![],
            inherits: vec![],
        }];
        let semantic = SemanticAnalysis {
            functions: funcs,
            classes,
            imports,
            references: vec![],
            call_frequency: HashMap::new(),
            calls: vec![],
            assignments: vec![],
            field_accesses: vec![],
        };

        let verbose_out = format_file_details_paginated(
            &semantic.functions,
            semantic.functions.len(),
            &semantic,
            "src/lib.rs",
            100,
            0,
            true,
        );
        let compact_out = format_file_details_paginated(
            &semantic.functions,
            semantic.functions.len(),
            &semantic,
            "src/lib.rs",
            100,
            0,
            false,
        );

        // Verbose includes C:, I:, F: section headers
        assert!(verbose_out.contains("C:\n"), "verbose must have C: section");
        assert!(verbose_out.contains("I:\n"), "verbose must have I: section");
        assert!(verbose_out.contains("F:\n"), "verbose must have F: section");

        // Compact includes C: and F: but omits I: (imports)
        assert!(
            compact_out.contains("C:\n"),
            "compact must have C: section (restored)"
        );
        assert!(
            !compact_out.contains("I:\n"),
            "compact must not have I: section (imports omitted)"
        );
        assert!(
            compact_out.contains("F:\n"),
            "compact must have F: section with wrapped formatting"
        );

        // Compact functions are wrapped at 100 chars, not one-per-line
        assert!(compact_out.contains("fn_0"), "compact must list functions");
        // Wrapped format should have multiple functions on same line with spaces
        // not newline after each function
    }

    /// Regression test: Compact mode output should be <= verbose mode for consistent token reduction.
    /// Tests with 10, 50, and 200 functions to ensure wrapped rendering saves space across file sizes.
    #[test]
    fn test_compact_mode_consistent_token_reduction() {
        use crate::types::{ClassInfo, FunctionInfo, ImportInfo, SemanticAnalysis};
        use std::collections::HashMap;

        for function_count in [10, 50, 200] {
            let funcs: Vec<FunctionInfo> = (0..function_count)
                .map(|i| FunctionInfo {
                    name: format!("function_name_{}", i),
                    line: i * 10 + 1,
                    end_line: i * 10 + 8,
                    parameters: vec![
                        "arg1: u32".to_string(),
                        "arg2: String".to_string(),
                        "arg3: Option<bool>".to_string(),
                    ],
                    return_type: Some("Result<Vec<String>, Error>".to_string()),
                })
                .collect();

            let imports: Vec<ImportInfo> = (0..5)
                .map(|i| ImportInfo {
                    module: format!("module::submodule_{}", i),
                    items: vec![
                        format!("Type{}", i),
                        format!("Trait{}", i),
                    ],
                    line: i + 1,
                })
                .collect();

            let classes: Vec<ClassInfo> = (0..3)
                .map(|i| ClassInfo {
                    name: format!("ClassName{}", i),
                    line: i * 50 + 100,
                    end_line: i * 50 + 140,
                    methods: vec![],
                    fields: vec![],
                    inherits: vec![],
                })
                .collect();

            let semantic = SemanticAnalysis {
                functions: funcs,
                classes,
                imports,
                references: vec![],
                call_frequency: HashMap::new(),
                calls: vec![],
                assignments: vec![],
                field_accesses: vec![],
            };

            let verbose_out = format_file_details_paginated(
                &semantic.functions,
                semantic.functions.len(),
                &semantic,
                "src/large_file.rs",
                10000,
                0,
                true,
            );

            let compact_out = format_file_details_paginated(
                &semantic.functions,
                semantic.functions.len(),
                &semantic,
                "src/large_file.rs",
                10000,
                0,
                false,
            );

            assert!(
                compact_out.len() <= verbose_out.len(),
                "Compact output ({} chars) must be <= verbose output ({} chars) for {} functions",
                compact_out.len(),
                verbose_out.len(),
                function_count
            );
        }
    }

    /// Edge case test: Compact mode with empty classes should not emit C: header.
    #[test]
    fn test_compact_mode_empty_classes_no_header() {
        use crate::types::{FunctionInfo, SemanticAnalysis};
        use std::collections::HashMap;

        let funcs: Vec<FunctionInfo> = (0..5)
            .map(|i| FunctionInfo {
                name: format!("fn_{}", i),
                line: i * 5 + 1,
                end_line: i * 5 + 4,
                parameters: vec![],
                return_type: None,
            })
            .collect();

        let semantic = SemanticAnalysis {
            functions: funcs,
            classes: vec![], // Empty classes
            imports: vec![],
            references: vec![],
            call_frequency: HashMap::new(),
            calls: vec![],
            assignments: vec![],
            field_accesses: vec![],
        };

        let compact_out = format_file_details_paginated(
            &semantic.functions,
            semantic.functions.len(),
            &semantic,
            "src/simple.rs",
            100,
            0,
            false,
        );

        // Should not have stray C: header when classes are empty
        assert!(
            !compact_out.contains("C:\n"),
            "compact mode must not emit C: header when classes are empty"
        );
    }
}

fn format_classes_section(classes: &[ClassInfo]) -> String {
    let mut output = String::new();
    if classes.is_empty() {
        return output;
    }
    output.push_str("C:\n");
    if classes.len() <= MULTILINE_THRESHOLD {
        let class_strs: Vec<String> = classes
            .iter()
            .map(|class| {
                if class.inherits.is_empty() {
                    format!("{}:{}", class.name, class.line)
                } else {
                    format!(
                        "{}:{} ({})",
                        class.name,
                        class.line,
                        class.inherits.join(", ")
                    )
                }
            })
            .collect();
        output.push_str("  ");
        output.push_str(&class_strs.join("; "));
        output.push('\n');
    } else {
        for class in classes {
            if class.inherits.is_empty() {
                output.push_str(&format!("  {}:{}\n", class.name, class.line));
            } else {
                output.push_str(&format!(
                    "  {}:{} ({})\n",
                    class.name,
                    class.line,
                    class.inherits.join(", ")
                ));
            }
        }
    }
    output
}

fn format_imports_section(imports: &[ImportInfo]) -> String {
    let mut output = String::new();
    if imports.is_empty() {
        return output;
    }
    output.push_str("I:\n");
    let mut module_map: HashMap<String, usize> = HashMap::new();
    for import in imports {
        module_map
            .entry(import.module.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }
    let mut modules: Vec<_> = module_map.keys().cloned().collect();
    modules.sort();
    let formatted_modules: Vec<String> = modules
        .iter()
        .map(|module| format!("{}({})", module, module_map[module]))
        .collect();
    if formatted_modules.len() <= MULTILINE_THRESHOLD {
        output.push_str("  ");
        output.push_str(&formatted_modules.join("; "));
        output.push('\n');
    } else {
        for module_str in formatted_modules {
            output.push_str("  ");
            output.push_str(&module_str);
            output.push('\n');
        }
    }
    output
}
