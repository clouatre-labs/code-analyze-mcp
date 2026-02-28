use crate::formatter::{format_structure, FileAnalysis};
use crate::lang::language_from_extension;
use crate::parser::ParserManager;
use crate::traversal::{walk_directory, WalkEntry};
use rayon::prelude::*;
use std::path::Path;
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Error)]
pub enum AnalyzeError {
    #[error("Traversal error: {0}")]
    Traversal(#[from] crate::traversal::TraversalError),
    #[error("Parser error: {0}")]
    Parser(#[from] crate::parser::ParserError),
}

/// Result of directory analysis containing both formatted output and file data.
pub struct AnalysisOutput {
    pub formatted: String,
    pub files: Vec<FileAnalysis>,
}

/// Analyze a directory structure and return formatted output and file data.
#[instrument(skip_all, fields(path = %root.display()))]
pub fn analyze_directory(
    root: &Path,
    max_depth: Option<u32>,
) -> Result<AnalysisOutput, AnalyzeError> {
    // Walk the directory
    let entries = walk_directory(root, max_depth)?;

    // Detect language from file extension
    let file_entries: Vec<&WalkEntry> = entries.iter().filter(|e| !e.is_dir).collect();

    // Parallel analysis of files
    let analysis_results: Vec<FileAnalysis> = file_entries
        .par_iter()
        .filter_map(|entry| {
            let path_str = entry.path.display().to_string();

            // Detect language from extension
            let ext = entry.path.extension().and_then(|e| e.to_str())?;
            let language = language_from_extension(ext)?.to_string();

            // Read file content
            let source = std::fs::read_to_string(&entry.path).ok()?;

            // Count lines
            let line_count = source.lines().count();

            // Extract counts
            let (function_count, class_count) =
                ParserManager::extract_counts(&source, &language).ok()?;

            Some(FileAnalysis {
                path: path_str,
                line_count,
                function_count,
                class_count,
                language,
            })
        })
        .collect();

    // Format output
    let formatted = format_structure(&entries, &analysis_results);

    Ok(AnalysisOutput {
        formatted,
        files: analysis_results,
    })
}
