use crate::graph::CallGraph;
use crate::test_detection::is_test_file;
use crate::traversal::WalkEntry;
use crate::types::{FileInfo, SemanticAnalysis};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use thiserror::Error;
use tracing::instrument;

const MULTILINE_THRESHOLD: usize = 10;

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

                let mut info_parts = Vec::new();

                if analysis.line_count > 0 {
                    info_parts.push(format!("{}L", analysis.line_count));
                }
                if analysis.function_count > 0 {
                    info_parts.push(format!("{}F", analysis.function_count));
                }
                if analysis.class_count > 0 {
                    info_parts.push(format!("{}C", analysis.class_count));
                }

                if info_parts.is_empty() {
                    output.push_str(&format!("{}{}\n", indent, name));
                } else {
                    output.push_str(&format!("{}{} [{}]\n", indent, name, info_parts.join(", ")));
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

                let mut info_parts = Vec::new();

                if analysis.line_count > 0 {
                    info_parts.push(format!("{}L", analysis.line_count));
                }
                if analysis.function_count > 0 {
                    info_parts.push(format!("{}F", analysis.function_count));
                }
                if analysis.class_count > 0 {
                    info_parts.push(format!("{}C", analysis.class_count));
                }

                if info_parts.is_empty() {
                    output.push_str(&format!("{}{}\n", indent, name));
                } else {
                    output.push_str(&format!("{}{} [{}]\n", indent, name, info_parts.join(", ")));
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
) -> String {
    let mut output = String::new();

    // FILE: header with counts, prepend [TEST] if applicable
    if is_test {
        output.push_str(&format!(
            "FILE [TEST] {}({}L, {}F, {}C, {}I)\n",
            path,
            line_count,
            analysis.functions.len(),
            analysis.classes.len(),
            analysis.imports.len()
        ));
    } else {
        output.push_str(&format!(
            "FILE: {}({}L, {}F, {}C, {}I)\n",
            path,
            line_count,
            analysis.functions.len(),
            analysis.classes.len(),
            analysis.imports.len()
        ));
    }

    // C: section with classes
    if !analysis.classes.is_empty() {
        output.push_str("C:\n");
        if analysis.classes.len() <= MULTILINE_THRESHOLD {
            // Inline format for <= 10 classes
            let class_strs: Vec<String> = analysis
                .classes
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
            // Multiline format for > 10 classes
            for class in &analysis.classes {
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
    }

    // F: section with functions, parameters, return types and call frequency
    if !analysis.functions.is_empty() {
        output.push_str("F:\n");
        let mut line = String::from("  ");
        for (i, func) in analysis.functions.iter().enumerate() {
            let mut call_marker = func.compact_signature();

            if let Some(&count) = analysis.call_frequency.get(&func.name)
                && count > 3
            {
                write!(call_marker, "\u{2022}{}", count).ok();
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
    }

    // I: section with imports grouped by module
    if !analysis.imports.is_empty() {
        output.push_str("I:\n");
        let mut module_map: HashMap<String, usize> = HashMap::new();
        for import in &analysis.imports {
            module_map
                .entry(import.module.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        let mut modules: Vec<_> = module_map.keys().cloned().collect();
        modules.sort();

        // Format modules with count notation
        let formatted_modules: Vec<String> = modules
            .iter()
            .map(|module| format!("{}({})", module, module_map[module]))
            .collect();

        if formatted_modules.len() <= MULTILINE_THRESHOLD {
            // Inline format for <= 10 modules
            output.push_str("  ");
            output.push_str(&formatted_modules.join("; "));
            output.push('\n');
        } else {
            // Multiline format for > 10 modules
            for module_str in formatted_modules {
                output.push_str("  ");
                output.push_str(&module_str);
                output.push('\n');
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
) -> Result<String, FormatterError> {
    let mut output = String::new();

    // FOCUS section
    output.push_str(&format!("FOCUS: {}\n", symbol));

    // DEPTH section
    output.push_str(&format!("DEPTH: {}\n", follow_depth));

    // DEFINED section - show where the symbol is defined
    if let Some(definitions) = graph.definitions.get(symbol) {
        output.push_str("DEFINED:\n");
        for (path, line) in definitions {
            output.push_str(&format!("  {}:{}\n", path.display(), line));
        }
    } else {
        output.push_str("DEFINED: (not found)\n");
    }

    // CALLERS section - who calls this symbol
    let incoming_chains = graph.find_incoming_chains(symbol, follow_depth)?;
    output.push_str("CALLERS:\n");
    if incoming_chains.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for chain in &incoming_chains {
            let chain_str = chain
                .chain
                .iter()
                .map(|(name, _, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(" <- ");
            output.push_str(&format!("  {}\n", chain_str));
        }
    }

    // CALLEES section - what this symbol calls
    let outgoing_chains = graph.find_outgoing_chains(symbol, follow_depth)?;
    output.push_str("CALLEES:\n");
    if outgoing_chains.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for chain in &outgoing_chains {
            let chain_str = chain
                .chain
                .iter()
                .map(|(name, _, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(" -> ");
            output.push_str(&format!("  {}\n", chain_str));
        }
    }

    // STATISTICS section
    output.push_str("STATISTICS:\n");
    let incoming_count = incoming_chains.len();
    let outgoing_count = outgoing_chains.len();
    output.push_str(&format!("  Incoming calls: {}\n", incoming_count));
    output.push_str(&format!("  Outgoing calls: {}\n", outgoing_count));

    // FILES section - collect unique files from chains
    let mut files = HashSet::new();
    for chain in &incoming_chains {
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
                output.push_str(&format!("  {}\n", file.display()));
            }
        }

        // Show test files in separate subsection
        if !test_files.is_empty() {
            output.push_str("  TEST FILES:\n");
            let mut sorted_files = test_files;
            sorted_files.sort();
            for file in sorted_files {
                output.push_str(&format!("    {}\n", file.display()));
            }
        }
    }

    Ok(output)
}

/// Format a compact summary for large directory analysis results.
/// Used when output would exceed the size threshold or when explicitly requested.
#[instrument(skip_all)]
pub fn format_summary(
    entries: &[WalkEntry],
    analysis_results: &[FileInfo],
    max_depth: Option<u32>,
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
                let mut info_parts = Vec::new();

                if analysis.line_count > 0 {
                    info_parts.push(format!("{}L", analysis.line_count));
                }
                if analysis.function_count > 0 {
                    info_parts.push(format!("{}F", analysis.function_count));
                }
                if analysis.class_count > 0 {
                    info_parts.push(format!("{}C", analysis.class_count));
                }

                if info_parts.is_empty() {
                    output.push_str(&format!("  {}\n", name));
                } else {
                    output.push_str(&format!("  {} [{}]\n", name, info_parts.join(", ")));
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

/// Format a paginated subset of files for Overview mode.
#[instrument(skip_all)]
pub fn format_structure_paginated(
    paginated_files: &[FileInfo],
    total_files: usize,
    max_depth: Option<u32>,
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
        output.push_str("FILES [LOC, FUNCTIONS, CLASSES]\n");
        for file in &prod_files {
            output.push_str(&format_file_entry(file));
        }
    }

    if !test_files.is_empty() {
        output.push_str("\nTEST FILES [LOC, FUNCTIONS, CLASSES]\n");
        for file in &test_files {
            output.push_str(&format_file_entry(file));
        }
    }

    output
}

fn format_file_entry(file: &FileInfo) -> String {
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
    if parts.is_empty() {
        format!("{}\n", file.path)
    } else {
        format!("{} [{}]\n", file.path, parts.join(", "))
    }
}
