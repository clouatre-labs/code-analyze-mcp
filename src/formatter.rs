use crate::graph::CallGraph;
use crate::traversal::WalkEntry;
use crate::types::{FileInfo, SemanticAnalysis};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use tracing::instrument;

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

    // SUMMARY block
    output.push_str("SUMMARY:\n");
    let depth_label = match max_depth {
        Some(n) if n > 0 => format!(" (max_depth={})", n),
        _ => String::new(),
    };
    output.push_str(&format!(
        "Shown: {} files, {}L, {}F, {}C{}\n",
        total_files, total_loc, total_functions, total_classes, depth_label
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

    // PATH block - tree structure
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

    output
}

/// Format file-level semantic analysis results.
#[instrument(skip_all)]
pub fn format_file_details(path: &str, analysis: &SemanticAnalysis, line_count: usize) -> String {
    let mut output = String::new();

    // FILE: header with counts
    output.push_str(&format!(
        "FILE: {} ({}L, {}F, {}C, {}I)\n",
        path,
        line_count,
        analysis.functions.len(),
        analysis.classes.len(),
        analysis.imports.len()
    ));

    // C: section with classes
    if !analysis.classes.is_empty() {
        output.push_str("C:\n");
        for class in &analysis.classes {
            output.push_str(&format!("  {}:{}\n", class.name, class.line));
        }
    }

    // F: section with functions, parameters, return types and call frequency
    if !analysis.functions.is_empty() {
        output.push_str("F:\n");
        let mut line = String::from("  ");
        for (i, func) in analysis.functions.iter().enumerate() {
            let params_display = func.parameters.first().map(|p| p.as_str()).unwrap_or("()");
            let ret_display = func
                .return_type
                .as_deref()
                .map(|r| format!(" {}", r))
                .unwrap_or_default();
            let call_suffix = if let Some(&count) = analysis.call_frequency.get(&func.name) {
                if count > 3 {
                    format!("•{}", count)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let call_marker = format!(
                "{}{}{}:{}{}",
                func.name, params_display, ret_display, func.line, call_suffix
            );

            if i == 0 {
                line.push_str(&call_marker);
            } else if line.len() + call_marker.len() + 2 > 100 {
                output.push_str(&line);
                output.push('\n');
                line = format!("  {}", call_marker);
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

        let mut modules: Vec<_> = module_map.keys().collect();
        modules.sort();
        for module in modules {
            let count = module_map[module];
            output.push_str(&format!("  {} ({})\n", module, count));
        }
    }

    // R: section with references and line numbers
    if !analysis.references.is_empty() {
        output.push_str("R:\n");
        for reference in &analysis.references {
            output.push_str(&format!(
                "  {} (line {}, Usage)\n",
                reference.symbol, reference.line
            ));
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

    output.push_str("FILES:\n");
    if files.is_empty() {
        output.push_str("  (none)\n");
    } else {
        let mut sorted_files: Vec<_> = files.into_iter().collect();
        sorted_files.sort();
        for file in sorted_files {
            output.push_str(&format!("  {}\n", file.display()));
        }
    }

    Ok(output)
}
