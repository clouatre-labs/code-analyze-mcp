use crate::formatter::{format_file_details, format_structure};
use crate::lang::language_from_extension;
use crate::parser::{ElementExtractor, SemanticExtractor};
use crate::traversal::{WalkEntry, walk_directory};
use crate::types::{AnalysisMode, FileInfo, SemanticAnalysis};
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
    pub files: Vec<FileInfo>,
}

/// Result of file-level semantic analysis.
pub struct FileAnalysisOutput {
    pub formatted: String,
    pub semantic: SemanticAnalysis,
    pub line_count: usize,
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
    let analysis_results: Vec<FileInfo> = file_entries
        .par_iter()
        .filter_map(|entry| {
            let path_str = entry.path.display().to_string();

            // Detect language from extension
            let ext = entry.path.extension().and_then(|e| e.to_str());

            // Try to read file content
            let source = match std::fs::read_to_string(&entry.path) {
                Ok(content) => content,
                Err(_) => {
                    // Binary file or unreadable - exclude from output
                    return None;
                }
            };

            // Count lines
            let line_count = source.lines().count();

            // Detect language and extract counts
            let (language, function_count, class_count) = if let Some(ext_str) = ext {
                if let Some(lang) = language_from_extension(ext_str) {
                    let lang_str = lang.to_string();
                    match ElementExtractor::extract_with_depth(&source, &lang_str) {
                        Ok((func_count, class_count)) => (lang_str, func_count, class_count),
                        Err(_) => (lang_str, 0, 0),
                    }
                } else {
                    ("unknown".to_string(), 0, 0)
                }
            } else {
                ("unknown".to_string(), 0, 0)
            };

            Some(FileInfo {
                path: path_str,
                line_count,
                function_count,
                class_count,
                language,
            })
        })
        .collect();

    // Format output
    let formatted = format_structure(&entries, &analysis_results, max_depth);

    Ok(AnalysisOutput {
        formatted,
        files: analysis_results,
    })
}

/// Determine analysis mode based on parameters and path.
pub fn determine_mode(path: &str, focus: Option<&str>) -> AnalysisMode {
    if focus.is_some() {
        return AnalysisMode::SymbolFocus;
    }

    let path_obj = Path::new(path);
    if path_obj.is_dir() {
        AnalysisMode::Overview
    } else {
        AnalysisMode::FileDetails
    }
}

/// Analyze a single file and return semantic analysis with formatted output.
#[instrument(skip_all, fields(path))]
pub fn analyze_file(
    path: &str,
    ast_recursion_limit: Option<usize>,
) -> Result<FileAnalysisOutput, AnalyzeError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| AnalyzeError::Parser(crate::parser::ParserError::ParseError(e.to_string())))?;

    let line_count = source.lines().count();

    // Detect language from extension
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .and_then(language_from_extension)
        .map(|l| l.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract semantic information
    let semantic = SemanticExtractor::extract(&source, &ext, ast_recursion_limit)?;

    // Format output
    let formatted = format_file_details(path, &semantic, line_count);

    Ok(FileAnalysisOutput {
        formatted,
        semantic,
        line_count,
    })
}
