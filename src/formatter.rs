use crate::traversal::WalkEntry;
use std::collections::HashMap;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct FileAnalysis {
    pub path: String,
    pub line_count: usize,
    pub function_count: usize,
    pub class_count: usize,
    pub language: String,
}

/// Format directory structure analysis results.
#[instrument(skip_all)]
pub fn format_structure(entries: &[WalkEntry], analysis_results: &[FileAnalysis]) -> String {
    let mut output = String::new();

    // Build a map of path -> analysis for quick lookup
    let analysis_map: HashMap<String, &FileAnalysis> = analysis_results
        .iter()
        .map(|a| (a.path.clone(), a))
        .collect();

    // Calculate totals
    let total_loc: usize = analysis_results.iter().map(|a| a.line_count).sum();
    let total_functions: usize = analysis_results.iter().map(|a| a.function_count).sum();
    let total_classes: usize = analysis_results.iter().map(|a| a.class_count).sum();

    // Count files by language
    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    for analysis in analysis_results {
        *lang_counts.entry(analysis.language.clone()).or_insert(0) += 1;
    }

    // SUMMARY block
    output.push_str("SUMMARY\n");
    output.push_str(&format!("Total LOC: {}\n", total_loc));
    output.push_str(&format!("Total Functions: {}\n", total_functions));
    output.push_str(&format!("Total Classes: {}\n", total_classes));

    if !lang_counts.is_empty() {
        output.push_str("Languages: ");
        let mut langs: Vec<_> = lang_counts.iter().collect();
        langs.sort_by_key(|&(name, _)| name);
        let lang_strs: Vec<String> = langs
            .iter()
            .map(|(name, count)| format!("{} ({})", name, count))
            .collect();
        output.push_str(&lang_strs.join(", "));
        output.push('\n');
    }

    output.push('\n');

    // PATH block - tree structure
    output.push_str("PATH\n");

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
                    info_parts.push(analysis.line_count.to_string());
                }
                if analysis.function_count > 0 {
                    info_parts.push(format!("FUNCTIONS: {}", analysis.function_count));
                }
                if analysis.class_count > 0 {
                    info_parts.push(format!("CLASSES: {}", analysis.class_count));
                }

                if info_parts.is_empty() {
                    output.push_str(&format!("{}{}\n", indent, name));
                } else {
                    output.push_str(&format!("{}{} [{}]\n", indent, name, info_parts.join(", ")));
                }
            } else {
                output.push_str(&format!("{}{}\n", indent, name));
            }
        } else {
            output.push_str(&format!("{}{}/\n", indent, name));
        }
    }

    output
}
