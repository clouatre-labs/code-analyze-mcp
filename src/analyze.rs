use crate::dataflow::DataflowGraph;
use crate::formatter::{
    format_file_details, format_focused, format_focused_summary, format_structure,
};
use crate::graph::CallGraph;
use crate::lang::language_from_extension;
use crate::parser::{ElementExtractor, SemanticExtractor};
use crate::test_detection::is_test_file;
use crate::traversal::{WalkEntry, walk_directory};
use crate::types::{AnalysisMode, FileInfo, SemanticAnalysis};
use rayon::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

#[derive(Debug, Error)]
pub enum AnalyzeError {
    #[error("Traversal error: {0}")]
    Traversal(#[from] crate::traversal::TraversalError),
    #[error("Parser error: {0}")]
    Parser(#[from] crate::parser::ParserError),
    #[error("Graph error: {0}")]
    Graph(#[from] crate::graph::GraphError),
    #[error("Formatter error: {0}")]
    Formatter(#[from] crate::formatter::FormatterError),
    #[error("Analysis cancelled")]
    Cancelled,
}

/// Result of directory analysis containing both formatted output and file data.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisOutput {
    #[schemars(description = "Formatted text representation of the analysis")]
    pub formatted: String,
    #[schemars(description = "List of files analyzed in the directory")]
    pub files: Vec<FileInfo>,
    /// Walk entries used internally for summary generation; not serialized.
    #[serde(skip)]
    #[schemars(skip)]
    pub entries: Vec<WalkEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Opaque cursor token for the next page of results (absent when no more results)"
    )]
    pub next_cursor: Option<String>,
}

/// Result of file-level semantic analysis.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FileAnalysisOutput {
    #[schemars(description = "Formatted text representation of the analysis")]
    pub formatted: String,
    #[schemars(description = "Semantic analysis data including functions, classes, and imports")]
    pub semantic: SemanticAnalysis,
    #[schemars(description = "Total line count of the analyzed file")]
    pub line_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Opaque cursor token for the next page of results (absent when no more results)"
    )]
    pub next_cursor: Option<String>,
}

/// Analyze a directory structure with progress tracking.
#[instrument(skip_all, fields(path = %root.display()))]
pub fn analyze_directory_with_progress(
    root: &Path,
    max_depth: Option<u32>,
    progress: Arc<AtomicUsize>,
    ct: CancellationToken,
) -> Result<AnalysisOutput, AnalyzeError> {
    // Check if already cancelled
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Walk the directory
    let entries = walk_directory(root, max_depth)?;

    // Detect language from file extension
    let file_entries: Vec<&WalkEntry> = entries.iter().filter(|e| !e.is_dir).collect();

    let start = Instant::now();
    tracing::debug!(file_count = file_entries.len(), root = %root.display(), "analysis start");

    // Parallel analysis of files
    let analysis_results: Vec<FileInfo> = file_entries
        .par_iter()
        .filter_map(|entry| {
            // Check cancellation per file
            if ct.is_cancelled() {
                return None;
            }

            let path_str = entry.path.display().to_string();

            // Detect language from extension
            let ext = entry.path.extension().and_then(|e| e.to_str());

            // Try to read file content
            let source = match std::fs::read_to_string(&entry.path) {
                Ok(content) => content,
                Err(_) => {
                    // Binary file or unreadable - exclude from output
                    progress.fetch_add(1, Ordering::Relaxed);
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

            progress.fetch_add(1, Ordering::Relaxed);

            let is_test = is_test_file(&entry.path);

            Some(FileInfo {
                path: path_str,
                line_count,
                function_count,
                class_count,
                language,
                is_test,
            })
        })
        .collect();

    // Check if cancelled after parallel processing
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    tracing::debug!(
        file_count = file_entries.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "analysis complete"
    );

    // Format output
    let formatted = format_structure(&entries, &analysis_results, max_depth, Some(root));

    Ok(AnalysisOutput {
        formatted,
        files: analysis_results,
        entries,
        next_cursor: None,
    })
}

/// Analyze a directory structure and return formatted output and file data.
#[instrument(skip_all, fields(path = %root.display()))]
pub fn analyze_directory(
    root: &Path,
    max_depth: Option<u32>,
) -> Result<AnalysisOutput, AnalyzeError> {
    let counter = Arc::new(AtomicUsize::new(0));
    let ct = CancellationToken::new();
    analyze_directory_with_progress(root, max_depth, counter, ct)
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
    let start = Instant::now();
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
    let mut semantic = SemanticExtractor::extract(&source, &ext, ast_recursion_limit)?;

    // Populate the file path on references now that the path is known
    for r in &mut semantic.references {
        r.location = path.to_string();
    }

    // Detect if this is a test file
    let is_test = is_test_file(Path::new(path));

    // Extract parent directory for relative path display
    let parent_dir = Path::new(path).parent();

    // Format output
    let formatted = format_file_details(path, &semantic, line_count, is_test, parent_dir);

    tracing::debug!(path = %path, language = %ext, functions = semantic.functions.len(), classes = semantic.classes.len(), imports = semantic.imports.len(), duration_ms = start.elapsed().as_millis() as u64, "file analysis complete");

    Ok(FileAnalysisOutput {
        formatted,
        semantic,
        line_count,
        next_cursor: None,
    })
}

/// Result of focused symbol analysis.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FocusedAnalysisOutput {
    #[schemars(description = "Formatted text representation of the call graph analysis")]
    pub formatted: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Opaque cursor token for the next page of results (absent when no more results)"
    )]
    pub next_cursor: Option<String>,
}

/// Analyze a symbol's call graph across a directory with progress tracking.
#[instrument(skip_all, fields(path = %root.display(), symbol = %focus))]
pub fn analyze_focused_with_progress(
    root: &Path,
    focus: &str,
    follow_depth: u32,
    max_depth: Option<u32>,
    ast_recursion_limit: Option<usize>,
    progress: Arc<AtomicUsize>,
    ct: CancellationToken,
) -> Result<FocusedAnalysisOutput, AnalyzeError> {
    // Check if already cancelled
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Check if path is a file (hint to use directory)
    if root.is_file() {
        let formatted =
            "Single-file focus not supported. Please provide a directory path for cross-file call graph analysis.\n"
                .to_string();
        return Ok(FocusedAnalysisOutput {
            formatted,
            next_cursor: None,
        });
    }

    // Walk the directory
    let entries = walk_directory(root, max_depth)?;

    // Collect semantic analysis for all files in parallel
    let file_entries: Vec<&WalkEntry> = entries.iter().filter(|e| !e.is_dir).collect();

    let analysis_results: Vec<(PathBuf, SemanticAnalysis)> = file_entries
        .par_iter()
        .filter_map(|entry| {
            // Check cancellation per file
            if ct.is_cancelled() {
                return None;
            }

            let ext = entry.path.extension().and_then(|e| e.to_str());

            // Try to read file content
            let source = match std::fs::read_to_string(&entry.path) {
                Ok(content) => content,
                Err(_) => {
                    progress.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
            };

            // Detect language and extract semantic information
            let language = if let Some(ext_str) = ext {
                language_from_extension(ext_str)
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            };

            match SemanticExtractor::extract(&source, &language, ast_recursion_limit) {
                Ok(mut semantic) => {
                    // Populate file path on references
                    for r in &mut semantic.references {
                        r.location = entry.path.display().to_string();
                    }
                    progress.fetch_add(1, Ordering::Relaxed);
                    Some((entry.path.clone(), semantic))
                }
                Err(_) => {
                    progress.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        })
        .collect();

    // Check if cancelled after parallel processing
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Build call graph
    let dataflow = DataflowGraph::build_from_results(&analysis_results);
    let graph = CallGraph::build_from_results(analysis_results)?;

    // Format output
    let formatted = format_focused(&graph, &dataflow, focus, follow_depth, Some(root))?;

    Ok(FocusedAnalysisOutput {
        formatted,
        next_cursor: None,
    })
}

/// Analyze a symbol's call graph with use_summary parameter (internal).
#[instrument(skip_all, fields(path = %root.display(), symbol = %focus))]
#[allow(clippy::too_many_arguments)]
pub fn analyze_focused_with_progress_internal(
    root: &Path,
    focus: &str,
    follow_depth: u32,
    max_depth: Option<u32>,
    ast_recursion_limit: Option<usize>,
    progress: Arc<AtomicUsize>,
    ct: CancellationToken,
    use_summary: bool,
) -> Result<FocusedAnalysisOutput, AnalyzeError> {
    // Check if already cancelled
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Check if path is a file (hint to use directory)
    if root.is_file() {
        let formatted =
            "Single-file focus not supported. Please provide a directory path for cross-file call graph analysis.\n"
                .to_string();
        return Ok(FocusedAnalysisOutput {
            formatted,
            next_cursor: None,
        });
    }

    // Walk the directory
    let entries = walk_directory(root, max_depth)?;

    // Collect semantic analysis for all files in parallel
    let file_entries: Vec<&WalkEntry> = entries.iter().filter(|e| !e.is_dir).collect();

    let analysis_results: Vec<(PathBuf, SemanticAnalysis)> = file_entries
        .par_iter()
        .filter_map(|entry| {
            // Check cancellation per file
            if ct.is_cancelled() {
                return None;
            }

            let ext = entry.path.extension().and_then(|e| e.to_str());

            // Try to read file content
            let source = match std::fs::read_to_string(&entry.path) {
                Ok(content) => content,
                Err(_) => {
                    progress.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
            };

            // Detect language and extract semantic information
            let language = if let Some(ext_str) = ext {
                language_from_extension(ext_str)
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            };

            match SemanticExtractor::extract(&source, &language, ast_recursion_limit) {
                Ok(mut semantic) => {
                    // Populate file path on references
                    for r in &mut semantic.references {
                        r.location = entry.path.display().to_string();
                    }
                    progress.fetch_add(1, Ordering::Relaxed);
                    Some((entry.path.clone(), semantic))
                }
                Err(_) => {
                    progress.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        })
        .collect();

    // Check if cancelled after parallel processing
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Build call graph
    let dataflow = DataflowGraph::build_from_results(&analysis_results);
    let graph = CallGraph::build_from_results(analysis_results)?;

    // Format output
    let formatted = if use_summary {
        format_focused_summary(&graph, &dataflow, focus, follow_depth, Some(root))?
    } else {
        format_focused(&graph, &dataflow, focus, follow_depth, Some(root))?
    };

    Ok(FocusedAnalysisOutput {
        formatted,
        next_cursor: None,
    })
}

/// Analyze a symbol's call graph across a directory.
#[instrument(skip_all, fields(path = %root.display(), symbol = %focus))]
pub fn analyze_focused(
    root: &Path,
    focus: &str,
    follow_depth: u32,
    max_depth: Option<u32>,
    ast_recursion_limit: Option<usize>,
) -> Result<FocusedAnalysisOutput, AnalyzeError> {
    let counter = Arc::new(AtomicUsize::new(0));
    let ct = CancellationToken::new();
    analyze_focused_with_progress(
        root,
        focus,
        follow_depth,
        max_depth,
        ast_recursion_limit,
        counter,
        ct,
    )
}
