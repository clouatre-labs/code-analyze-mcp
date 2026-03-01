use crate::traversal::WalkEntry;
use crate::types::{FileInfo, SemanticAnalysis};
use std::collections::HashMap;
use tracing::instrument;

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

    // F: section with functions and call frequency
    if !analysis.functions.is_empty() {
        output.push_str("F:\n");
        let mut line = String::from("  ");
        for (i, func) in analysis.functions.iter().enumerate() {
            let call_marker = if let Some(&count) = analysis.call_frequency.get(&func.name) {
                if count > 3 {
                    format!("{}:{}•{}", func.name, func.line, count)
                } else {
                    format!("{}:{}", func.name, func.line)
                }
            } else {
                format!("{}:{}", func.name, func.line)
            };

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
        let mut module_map: HashMap<String, Vec<String>> = HashMap::new();
        for import in &analysis.imports {
            module_map
                .entry(import.module.clone())
                .or_default()
                .extend(import.items.clone());
        }

        let mut modules: Vec<_> = module_map.keys().collect();
        modules.sort();
        for module in modules {
            let items = &module_map[module];
            output.push_str(&format!("  {} ({})\n", module, items.len()));
        }
    }

    // R: section with references (conditional)
    if !analysis.references.is_empty() {
        output.push_str("R:\n");
        for reference in &analysis.references {
            output.push_str(&format!("  {}\n", reference));
        }
    }

    output
}
