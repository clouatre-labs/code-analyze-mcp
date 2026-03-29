// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
//! Main analysis engine for extracting code structure from files and directories.
//!
//! Implements the four MCP tools: `analyze_directory` (Overview), `analyze_file` (`FileDetails`),
//! `analyze_symbol` (call graph), and `analyze_module` (lightweight index). Handles parallel processing and cancellation.

use crate::formatter::{
    format_file_details, format_focused_internal, format_focused_summary_internal, format_structure,
};
use crate::graph::{CallGraph, InternalCallChain};
use crate::lang::language_for_extension;
use crate::parser::{ElementExtractor, SemanticExtractor};
use crate::test_detection::is_test_file;
use crate::traversal::{WalkEntry, walk_directory};
use crate::types::{
    AnalysisMode, FileInfo, ImplTraitInfo, ImportInfo, SemanticAnalysis, SymbolMatchMode,
};
use rayon::prelude::*;
#[cfg(feature = "schemars")]
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
#[non_exhaustive]
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
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[non_exhaustive]
pub struct AnalysisOutput {
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "Formatted text representation of the analysis")
    )]
    pub formatted: String,
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "List of files analyzed in the directory")
    )]
    pub files: Vec<FileInfo>,
    /// Walk entries used internally for summary generation; not serialized.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub entries: Vec<WalkEntry>,
    /// Subtree file counts computed from an unbounded walk; used by `format_summary`; not serialized.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub subtree_counts: Option<Vec<(std::path::PathBuf, usize)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "schemars",
        schemars(
            description = "Opaque cursor token for the next page of results (absent when no more results)"
        )
    )]
    pub next_cursor: Option<String>,
}

/// Result of file-level semantic analysis.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[non_exhaustive]
pub struct FileAnalysisOutput {
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "Formatted text representation of the analysis")
    )]
    pub formatted: String,
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "Semantic analysis data including functions, classes, and imports")
    )]
    pub semantic: SemanticAnalysis,
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "Total line count of the analyzed file")
    )]
    #[cfg_attr(
        feature = "schemars",
        schemars(schema_with = "crate::schema_helpers::integer_schema")
    )]
    pub line_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "schemars",
        schemars(
            description = "Opaque cursor token for the next page of results (absent when no more results)"
        )
    )]
    pub next_cursor: Option<String>,
}

impl FileAnalysisOutput {
    /// Create a new `FileAnalysisOutput`.
    #[must_use]
    pub fn new(
        formatted: String,
        semantic: SemanticAnalysis,
        line_count: usize,
        next_cursor: Option<String>,
    ) -> Self {
        Self {
            formatted,
            semantic,
            line_count,
            next_cursor,
        }
    }
}
#[instrument(skip_all, fields(path = %root.display()))]
// public API; callers expect owned semantics
#[allow(clippy::needless_pass_by_value)]
pub fn analyze_directory_with_progress(
    root: &Path,
    entries: Vec<WalkEntry>,
    progress: Arc<AtomicUsize>,
    ct: CancellationToken,
) -> Result<AnalysisOutput, AnalyzeError> {
    // Check if already cancelled
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

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

            // Try to read file content; skip binary or unreadable files
            let Ok(source) = std::fs::read_to_string(&entry.path) else {
                progress.fetch_add(1, Ordering::Relaxed);
                return None;
            };

            // Count lines
            let line_count = source.lines().count();

            // Detect language and extract counts
            let (language, function_count, class_count) = if let Some(ext_str) = ext {
                if let Some(lang) = language_for_extension(ext_str) {
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
        duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        "analysis complete"
    );

    // Format output
    let formatted = format_structure(&entries, &analysis_results, None);

    Ok(AnalysisOutput {
        formatted,
        files: analysis_results,
        entries,
        next_cursor: None,
        subtree_counts: None,
    })
}

/// Analyze a directory structure and return formatted output and file data.
#[instrument(skip_all, fields(path = %root.display()))]
pub fn analyze_directory(
    root: &Path,
    max_depth: Option<u32>,
) -> Result<AnalysisOutput, AnalyzeError> {
    let entries = walk_directory(root, max_depth)?;
    let counter = Arc::new(AtomicUsize::new(0));
    let ct = CancellationToken::new();
    analyze_directory_with_progress(root, entries, counter, ct)
}

/// Determine analysis mode based on parameters and path.
#[must_use]
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
        .and_then(language_for_extension)
        .map_or_else(|| "unknown".to_string(), std::string::ToString::to_string);

    // Extract semantic information
    let mut semantic = SemanticExtractor::extract(&source, &ext, ast_recursion_limit)?;

    // Populate the file path on references now that the path is known
    for r in &mut semantic.references {
        r.location = path.to_string();
    }

    // Resolve Python wildcard imports
    if ext == "python" {
        resolve_wildcard_imports(Path::new(path), &mut semantic.imports);
    }

    // Detect if this is a test file
    let is_test = is_test_file(Path::new(path));

    // Extract parent directory for relative path display
    let parent_dir = Path::new(path).parent();

    // Format output
    let formatted = format_file_details(path, &semantic, line_count, is_test, parent_dir);

    tracing::debug!(path = %path, language = %ext, functions = semantic.functions.len(), classes = semantic.classes.len(), imports = semantic.imports.len(), duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX), "file analysis complete");

    Ok(FileAnalysisOutput::new(
        formatted, semantic, line_count, None,
    ))
}

/// Result of focused symbol analysis.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[non_exhaustive]
pub struct FocusedAnalysisOutput {
    #[cfg_attr(
        feature = "schemars",
        schemars(description = "Formatted text representation of the call graph analysis")
    )]
    pub formatted: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "schemars",
        schemars(
            description = "Opaque cursor token for the next page of results (absent when no more results)"
        )
    )]
    pub next_cursor: Option<String>,
    /// Production caller chains (partitioned from incoming chains, excluding test callers).
    /// Not serialized; used for pagination in lib.rs.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub prod_chains: Vec<InternalCallChain>,
    /// Test caller chains. Not serialized; used for pagination summary in lib.rs.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub test_chains: Vec<InternalCallChain>,
    /// Outgoing (callee) chains. Not serialized; used for pagination in lib.rs.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub outgoing_chains: Vec<InternalCallChain>,
    /// Number of definitions for the symbol. Not serialized; used for pagination headers.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub def_count: usize,
    /// Total unique callers before `impl_only` filter. Not serialized; used for FILTER header.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub unfiltered_caller_count: usize,
    /// Unique callers after `impl_only` filter. Not serialized; used for FILTER header.
    #[serde(skip)]
    #[cfg_attr(feature = "schemars", schemars(skip))]
    pub impl_trait_caller_count: usize,
}

/// Parameters for focused symbol analysis. Groups high-arity parameters to keep
/// function signatures under clippy's default 7-argument threshold.
#[derive(Clone)]
pub struct FocusedAnalysisConfig {
    pub focus: String,
    pub match_mode: SymbolMatchMode,
    pub follow_depth: u32,
    pub max_depth: Option<u32>,
    pub ast_recursion_limit: Option<usize>,
    pub use_summary: bool,
    pub impl_only: Option<bool>,
}

/// Internal parameters for focused analysis phases.
#[derive(Clone)]
struct FocusedAnalysisParams {
    focus: String,
    match_mode: SymbolMatchMode,
    follow_depth: u32,
    ast_recursion_limit: Option<usize>,
    use_summary: bool,
    impl_only: Option<bool>,
}

/// Type alias for analysis results: (`file_path`, `semantic_analysis`) pairs and impl-trait info.
type AnalysisResults = (Vec<(PathBuf, SemanticAnalysis)>, Vec<ImplTraitInfo>);

/// Phase 1: Collect semantic analysis for all files in parallel.
fn collect_file_analysis(
    entries: &[WalkEntry],
    progress: &Arc<AtomicUsize>,
    ct: &CancellationToken,
    ast_recursion_limit: Option<usize>,
) -> Result<AnalysisResults, AnalyzeError> {
    // Check if already cancelled
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Use pre-walked entries (passed by caller)
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
            let Ok(source) = std::fs::read_to_string(&entry.path) else {
                progress.fetch_add(1, Ordering::Relaxed);
                return None;
            };

            // Detect language and extract semantic information
            let language = if let Some(ext_str) = ext {
                language_for_extension(ext_str)
                    .map_or_else(|| "unknown".to_string(), std::string::ToString::to_string)
            } else {
                "unknown".to_string()
            };

            if let Ok(mut semantic) =
                SemanticExtractor::extract(&source, &language, ast_recursion_limit)
            {
                // Populate file path on references
                for r in &mut semantic.references {
                    r.location = entry.path.display().to_string();
                }
                // Populate file path on impl_traits (already extracted during SemanticExtractor::extract)
                for trait_info in &mut semantic.impl_traits {
                    trait_info.path.clone_from(&entry.path);
                }
                progress.fetch_add(1, Ordering::Relaxed);
                Some((entry.path.clone(), semantic))
            } else {
                progress.fetch_add(1, Ordering::Relaxed);
                None
            }
        })
        .collect();

    // Check if cancelled after parallel processing
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Collect all impl-trait info from analysis results
    let all_impl_traits: Vec<ImplTraitInfo> = analysis_results
        .iter()
        .flat_map(|(_, sem)| sem.impl_traits.iter().cloned())
        .collect();

    Ok((analysis_results, all_impl_traits))
}

/// Phase 2: Build call graph from analysis results.
fn build_call_graph(
    analysis_results: Vec<(PathBuf, SemanticAnalysis)>,
    all_impl_traits: &[ImplTraitInfo],
) -> Result<CallGraph, AnalyzeError> {
    // Build call graph. Always build without impl_only filter first so we can
    // record the unfiltered caller count before discarding those edges.
    CallGraph::build_from_results(
        analysis_results,
        all_impl_traits,
        false, // filter applied below after counting
    )
    .map_err(std::convert::Into::into)
}

/// Phase 3: Resolve symbol and apply `impl_only` filter.
/// Returns (`resolved_focus`, `unfiltered_caller_count`, `impl_trait_caller_count`).
/// CRITICAL: Must capture `unfiltered_caller_count` BEFORE `retain()`, then apply `retain()`,
/// then compute `impl_trait_caller_count`.
fn resolve_symbol(
    graph: &mut CallGraph,
    params: &FocusedAnalysisParams,
) -> Result<(String, usize, usize), AnalyzeError> {
    // Resolve symbol name using the requested match mode.
    let resolved_focus = if params.match_mode == SymbolMatchMode::Exact {
        let exists = graph.definitions.contains_key(&params.focus)
            || graph.callers.contains_key(&params.focus)
            || graph.callees.contains_key(&params.focus);
        if exists {
            params.focus.clone()
        } else {
            return Err(crate::graph::GraphError::SymbolNotFound {
                symbol: params.focus.clone(),
                hint: "Try match_mode=insensitive for a case-insensitive search, or match_mode=prefix to list symbols starting with this name.".to_string(),
            }
            .into());
        }
    } else {
        graph.resolve_symbol_indexed(&params.focus, &params.match_mode)?
    };

    // Count unique callers for the focus symbol before applying impl_only filter.
    let unfiltered_caller_count = graph.callers.get(&resolved_focus).map_or(0, |edges| {
        edges
            .iter()
            .map(|e| &e.neighbor_name)
            .collect::<std::collections::HashSet<_>>()
            .len()
    });

    // Apply impl_only filter now if requested, then count filtered callers.
    // Filter all caller adjacency lists so traversal and formatting are consistently
    // restricted to impl-trait edges regardless of follow_depth.
    let impl_trait_caller_count = if params.impl_only.unwrap_or(false) {
        for edges in graph.callers.values_mut() {
            edges.retain(|e| e.is_impl_trait);
        }
        graph.callers.get(&resolved_focus).map_or(0, |edges| {
            edges
                .iter()
                .map(|e| &e.neighbor_name)
                .collect::<std::collections::HashSet<_>>()
                .len()
        })
    } else {
        unfiltered_caller_count
    };

    Ok((
        resolved_focus,
        unfiltered_caller_count,
        impl_trait_caller_count,
    ))
}

/// Type alias for `compute_chains` return type: (`formatted_output`, `prod_chains`, `test_chains`, `outgoing_chains`, `def_count`).
type ChainComputeResult = (
    String,
    Vec<InternalCallChain>,
    Vec<InternalCallChain>,
    Vec<InternalCallChain>,
    usize,
);

/// Phase 4: Compute chains and format output.
fn compute_chains(
    graph: &CallGraph,
    resolved_focus: &str,
    root: &Path,
    params: &FocusedAnalysisParams,
    unfiltered_caller_count: usize,
    impl_trait_caller_count: usize,
) -> Result<ChainComputeResult, AnalyzeError> {
    // Compute chain data for pagination (always, regardless of summary mode)
    let def_count = graph.definitions.get(resolved_focus).map_or(0, Vec::len);
    let incoming_chains = graph.find_incoming_chains(resolved_focus, params.follow_depth)?;
    let outgoing_chains = graph.find_outgoing_chains(resolved_focus, params.follow_depth)?;

    let (prod_chains, test_chains): (Vec<_>, Vec<_>) =
        incoming_chains.iter().cloned().partition(|chain| {
            chain
                .chain
                .first()
                .is_none_or(|(name, path, _)| !is_test_file(path) && !name.starts_with("test_"))
        });

    // Format output with pre-computed chains
    let mut formatted = if params.use_summary {
        format_focused_summary_internal(
            graph,
            resolved_focus,
            params.follow_depth,
            Some(root),
            Some(&incoming_chains),
            Some(&outgoing_chains),
        )?
    } else {
        format_focused_internal(
            graph,
            resolved_focus,
            params.follow_depth,
            Some(root),
            Some(&incoming_chains),
            Some(&outgoing_chains),
        )?
    };

    // Add FILTER header if impl_only filter was applied
    if params.impl_only.unwrap_or(false) {
        let filter_header = format!(
            "FILTER: impl_only=true ({impl_trait_caller_count} of {unfiltered_caller_count} callers shown)\n",
        );
        formatted = format!("{filter_header}{formatted}");
    }

    Ok((
        formatted,
        prod_chains,
        test_chains,
        outgoing_chains,
        def_count,
    ))
}

/// Analyze a symbol's call graph across a directory with progress tracking.
// public API; callers expect owned semantics
#[allow(clippy::needless_pass_by_value)]
pub fn analyze_focused_with_progress(
    root: &Path,
    params: &FocusedAnalysisConfig,
    progress: Arc<AtomicUsize>,
    ct: CancellationToken,
) -> Result<FocusedAnalysisOutput, AnalyzeError> {
    let entries = walk_directory(root, params.max_depth)?;
    let internal_params = FocusedAnalysisParams {
        focus: params.focus.clone(),
        match_mode: params.match_mode.clone(),
        follow_depth: params.follow_depth,
        ast_recursion_limit: params.ast_recursion_limit,
        use_summary: params.use_summary,
        impl_only: params.impl_only,
    };
    analyze_focused_with_progress_with_entries_internal(
        root,
        params.max_depth,
        &progress,
        &ct,
        &internal_params,
        &entries,
    )
}

/// Internal implementation of focused analysis using pre-walked entries and params struct.
#[instrument(skip_all, fields(path = %root.display(), symbol = %params.focus))]
fn analyze_focused_with_progress_with_entries_internal(
    root: &Path,
    _max_depth: Option<u32>,
    progress: &Arc<AtomicUsize>,
    ct: &CancellationToken,
    params: &FocusedAnalysisParams,
    entries: &[WalkEntry],
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
            prod_chains: vec![],
            test_chains: vec![],
            outgoing_chains: vec![],
            def_count: 0,
            unfiltered_caller_count: 0,
            impl_trait_caller_count: 0,
        });
    }

    // Phase 1: Collect file analysis
    let (analysis_results, all_impl_traits) =
        collect_file_analysis(entries, progress, ct, params.ast_recursion_limit)?;

    // Check for cancellation before building the call graph (phase 2)
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Phase 2: Build call graph
    let mut graph = build_call_graph(analysis_results, &all_impl_traits)?;

    // Check for cancellation before resolving the symbol (phase 3)
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Phase 3: Resolve symbol and apply impl_only filter
    let (resolved_focus, unfiltered_caller_count, impl_trait_caller_count) =
        resolve_symbol(&mut graph, params)?;

    // Check for cancellation before computing chains (phase 4)
    if ct.is_cancelled() {
        return Err(AnalyzeError::Cancelled);
    }

    // Phase 4: Compute chains and format output
    let (formatted, prod_chains, test_chains, outgoing_chains, def_count) = compute_chains(
        &graph,
        &resolved_focus,
        root,
        params,
        unfiltered_caller_count,
        impl_trait_caller_count,
    )?;

    Ok(FocusedAnalysisOutput {
        formatted,
        next_cursor: None,
        prod_chains,
        test_chains,
        outgoing_chains,
        def_count,
        unfiltered_caller_count,
        impl_trait_caller_count,
    })
}

/// Analyze a symbol's call graph using pre-walked directory entries.
pub fn analyze_focused_with_progress_with_entries(
    root: &Path,
    params: &FocusedAnalysisConfig,
    progress: &Arc<AtomicUsize>,
    ct: &CancellationToken,
    entries: &[WalkEntry],
) -> Result<FocusedAnalysisOutput, AnalyzeError> {
    let internal_params = FocusedAnalysisParams {
        focus: params.focus.clone(),
        match_mode: params.match_mode.clone(),
        follow_depth: params.follow_depth,
        ast_recursion_limit: params.ast_recursion_limit,
        use_summary: params.use_summary,
        impl_only: params.impl_only,
    };
    analyze_focused_with_progress_with_entries_internal(
        root,
        params.max_depth,
        progress,
        ct,
        &internal_params,
        entries,
    )
}

#[instrument(skip_all, fields(path = %root.display(), symbol = %focus))]
pub fn analyze_focused(
    root: &Path,
    focus: &str,
    follow_depth: u32,
    max_depth: Option<u32>,
    ast_recursion_limit: Option<usize>,
) -> Result<FocusedAnalysisOutput, AnalyzeError> {
    let entries = walk_directory(root, max_depth)?;
    let counter = Arc::new(AtomicUsize::new(0));
    let ct = CancellationToken::new();
    let params = FocusedAnalysisConfig {
        focus: focus.to_string(),
        match_mode: SymbolMatchMode::Exact,
        follow_depth,
        max_depth,
        ast_recursion_limit,
        use_summary: false,
        impl_only: None,
    };
    analyze_focused_with_progress_with_entries(root, &params, &counter, &ct, &entries)
}

/// Analyze a single file and return a minimal fixed schema (name, line count, language,
/// functions, imports) for lightweight code understanding.
#[instrument(skip_all, fields(path))]
pub fn analyze_module_file(path: &str) -> Result<crate::types::ModuleInfo, AnalyzeError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| AnalyzeError::Parser(crate::parser::ParserError::ParseError(e.to_string())))?;

    let file_path = Path::new(path);
    let name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let line_count = source.lines().count();

    let language = file_path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(language_for_extension)
        .ok_or_else(|| {
            AnalyzeError::Parser(crate::parser::ParserError::ParseError(
                "unsupported or missing file extension".to_string(),
            ))
        })?;

    let semantic = SemanticExtractor::extract(&source, language, None)?;

    let functions = semantic
        .functions
        .into_iter()
        .map(|f| crate::types::ModuleFunctionInfo {
            name: f.name,
            line: f.line,
        })
        .collect();

    let imports = semantic
        .imports
        .into_iter()
        .map(|i| crate::types::ModuleImportInfo {
            module: i.module,
            items: i.items,
        })
        .collect();

    Ok(crate::types::ModuleInfo {
        name,
        line_count,
        language: language.to_string(),
        functions,
        imports,
    })
}

/// Resolve Python wildcard imports to actual symbol names.
///
/// For each import with items=`["*"]`, this function:
/// 1. Parses the relative dots (if any) and climbs the directory tree
/// 2. Finds the target .py file or __init__.py
/// 3. Extracts symbols (functions and classes) from the target
/// 4. Honors __all__ if defined, otherwise uses function+class names
///
/// All resolution failures are non-fatal: debug-logged and the wildcard is preserved.
fn resolve_wildcard_imports(file_path: &Path, imports: &mut [ImportInfo]) {
    use std::collections::HashMap;

    let mut resolved_cache: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let Ok(file_path_canonical) = file_path.canonicalize() else {
        tracing::debug!(file = ?file_path, "unable to canonicalize current file path");
        return;
    };

    for import in imports.iter_mut() {
        if import.items != ["*"] {
            continue;
        }
        resolve_single_wildcard(import, file_path, &file_path_canonical, &mut resolved_cache);
    }
}

/// Resolve one wildcard import in place. On any failure the import is left unchanged.
fn resolve_single_wildcard(
    import: &mut ImportInfo,
    file_path: &Path,
    file_path_canonical: &Path,
    resolved_cache: &mut std::collections::HashMap<PathBuf, Vec<String>>,
) {
    let module = import.module.clone();
    let dot_count = module.chars().take_while(|c| *c == '.').count();
    if dot_count == 0 {
        return;
    }
    let module_path = module.trim_start_matches('.');

    let Some(target_to_read) = locate_target_file(file_path, dot_count, module_path, &module)
    else {
        return;
    };

    let Ok(canonical) = target_to_read.canonicalize() else {
        tracing::debug!(target = ?target_to_read, import = %module, "unable to canonicalize path");
        return;
    };

    if canonical == file_path_canonical {
        tracing::debug!(target = ?canonical, import = %module, "cannot import from self");
        return;
    }

    if let Some(cached) = resolved_cache.get(&canonical) {
        tracing::debug!(import = %module, symbols_count = cached.len(), "using cached symbols");
        import.items.clone_from(cached);
        return;
    }

    if let Some(symbols) = parse_target_symbols(&target_to_read, &module) {
        tracing::debug!(import = %module, resolved_count = symbols.len(), "wildcard import resolved");
        import.items.clone_from(&symbols);
        resolved_cache.insert(canonical, symbols);
    }
}

/// Locate the .py file that a wildcard import refers to. Returns None if not found.
fn locate_target_file(
    file_path: &Path,
    dot_count: usize,
    module_path: &str,
    module: &str,
) -> Option<PathBuf> {
    let mut target_dir = file_path.parent()?.to_path_buf();

    for _ in 1..dot_count {
        if !target_dir.pop() {
            tracing::debug!(import = %module, "unable to climb {} levels", dot_count.saturating_sub(1));
            return None;
        }
    }

    let target_file = if module_path.is_empty() {
        target_dir.join("__init__.py")
    } else {
        let rel_path = module_path.replace('.', "/");
        target_dir.join(format!("{rel_path}.py"))
    };

    if target_file.exists() {
        Some(target_file)
    } else if target_file.with_extension("").is_dir() {
        let init = target_file.with_extension("").join("__init__.py");
        if init.exists() { Some(init) } else { None }
    } else {
        tracing::debug!(target = ?target_file, import = %module, "target file not found");
        None
    }
}

/// Read and parse a target .py file, returning its exported symbols.
fn parse_target_symbols(target_path: &Path, module: &str) -> Option<Vec<String>> {
    use tree_sitter::Parser;

    let source = match std::fs::read_to_string(target_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(target = ?target_path, import = %module, error = %e, "unable to read target file");
            return None;
        }
    };

    // Parse once with tree-sitter
    let lang_info = crate::languages::get_language_info("python")?;
    let mut parser = Parser::new();
    if parser.set_language(&lang_info.language).is_err() {
        return None;
    }
    let tree = parser.parse(&source, None)?;

    // First, try to extract __all__ from the same tree
    let mut symbols = Vec::new();
    extract_all_from_tree(&tree, &source, &mut symbols);
    if !symbols.is_empty() {
        tracing::debug!(import = %module, symbols = ?symbols, "using __all__ symbols");
        return Some(symbols);
    }

    // Fallback: extract functions/classes from the tree
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if matches!(child.kind(), "function_definition" | "class_definition")
            && let Some(name_node) = child.child_by_field_name("name")
        {
            let name = source[name_node.start_byte()..name_node.end_byte()].to_string();
            if !name.starts_with('_') {
                symbols.push(name);
            }
        }
    }
    tracing::debug!(import = %module, fallback_symbols = ?symbols, "using fallback function/class names");
    Some(symbols)
}

/// Extract __all__ from a tree-sitter tree.
fn extract_all_from_tree(tree: &tree_sitter::Tree, source: &str, result: &mut Vec<String>) {
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "simple_statement" {
            // simple_statement contains assignment and other statement types
            let mut simple_cursor = child.walk();
            for simple_child in child.children(&mut simple_cursor) {
                if simple_child.kind() == "assignment"
                    && let Some(left) = simple_child.child_by_field_name("left")
                {
                    let target_text = source[left.start_byte()..left.end_byte()].trim();
                    if target_text == "__all__"
                        && let Some(right) = simple_child.child_by_field_name("right")
                    {
                        extract_string_list_from_list_node(&right, source, result);
                    }
                }
            }
        } else if child.kind() == "expression_statement" {
            // Fallback for older Python AST structures
            let mut stmt_cursor = child.walk();
            for stmt_child in child.children(&mut stmt_cursor) {
                if stmt_child.kind() == "assignment"
                    && let Some(left) = stmt_child.child_by_field_name("left")
                {
                    let target_text = source[left.start_byte()..left.end_byte()].trim();
                    if target_text == "__all__"
                        && let Some(right) = stmt_child.child_by_field_name("right")
                    {
                        extract_string_list_from_list_node(&right, source, result);
                    }
                }
            }
        }
    }
}

/// Extract string literals from a Python list node.
fn extract_string_list_from_list_node(
    list_node: &tree_sitter::Node,
    source: &str,
    result: &mut Vec<String>,
) {
    let mut cursor = list_node.walk();
    for child in list_node.named_children(&mut cursor) {
        if child.kind() == "string" {
            let raw = source[child.start_byte()..child.end_byte()].trim();
            // Strip quotes: "name" -> name
            let unquoted = raw.trim_matches('"').trim_matches('\'').to_string();
            if !unquoted.is_empty() {
                result.push(unquoted);
            }
        }
    }
}

#[cfg(all(test, feature = "lang-rust"))]
mod tests {
    use super::*;
    use crate::formatter::format_focused_paginated;
    use crate::pagination::{PaginationMode, decode_cursor, paginate_slice};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_symbol_focus_callers_pagination_first_page() {
        let temp_dir = TempDir::new().unwrap();

        // Create a file with many callers of `target`
        let mut code = String::from("fn target() {}\n");
        for i in 0..15 {
            code.push_str(&format!("fn caller_{:02}() {{ target(); }}\n", i));
        }
        fs::write(temp_dir.path().join("lib.rs"), &code).unwrap();

        // Act
        let output = analyze_focused(temp_dir.path(), "target", 1, None, None).unwrap();

        // Paginate prod callers with page_size=5
        let paginated = paginate_slice(&output.prod_chains, 0, 5, PaginationMode::Callers)
            .expect("paginate failed");
        assert!(
            paginated.total >= 5,
            "should have enough callers to paginate"
        );
        assert!(
            paginated.next_cursor.is_some(),
            "should have next_cursor for page 1"
        );

        // Verify cursor encodes callers mode
        assert_eq!(paginated.items.len(), 5);
    }

    #[test]
    fn test_symbol_focus_callers_pagination_second_page() {
        let temp_dir = TempDir::new().unwrap();

        let mut code = String::from("fn target() {}\n");
        for i in 0..12 {
            code.push_str(&format!("fn caller_{:02}() {{ target(); }}\n", i));
        }
        fs::write(temp_dir.path().join("lib.rs"), &code).unwrap();

        let output = analyze_focused(temp_dir.path(), "target", 1, None, None).unwrap();
        let total_prod = output.prod_chains.len();

        if total_prod > 5 {
            // Get page 1 cursor
            let p1 = paginate_slice(&output.prod_chains, 0, 5, PaginationMode::Callers)
                .expect("paginate failed");
            assert!(p1.next_cursor.is_some());

            let cursor_str = p1.next_cursor.unwrap();
            let cursor_data = decode_cursor(&cursor_str).expect("decode failed");

            // Get page 2
            let p2 = paginate_slice(
                &output.prod_chains,
                cursor_data.offset,
                5,
                PaginationMode::Callers,
            )
            .expect("paginate failed");

            // Format paginated output
            let formatted = format_focused_paginated(
                &p2.items,
                total_prod,
                PaginationMode::Callers,
                "target",
                &output.prod_chains,
                &output.test_chains,
                &output.outgoing_chains,
                output.def_count,
                cursor_data.offset,
                Some(temp_dir.path()),
                true,
            );

            // Assert: header shows correct range for page 2
            let expected_start = cursor_data.offset + 1;
            assert!(
                formatted.contains(&format!("CALLERS ({}", expected_start)),
                "header should show page 2 range, got: {}",
                formatted
            );
        }
    }

    #[test]
    fn test_symbol_focus_callees_pagination() {
        let temp_dir = TempDir::new().unwrap();

        // target calls many functions
        let mut code = String::from("fn target() {\n");
        for i in 0..10 {
            code.push_str(&format!("    callee_{:02}();\n", i));
        }
        code.push_str("}\n");
        for i in 0..10 {
            code.push_str(&format!("fn callee_{:02}() {{}}\n", i));
        }
        fs::write(temp_dir.path().join("lib.rs"), &code).unwrap();

        let output = analyze_focused(temp_dir.path(), "target", 1, None, None).unwrap();
        let total_callees = output.outgoing_chains.len();

        if total_callees > 3 {
            let paginated = paginate_slice(&output.outgoing_chains, 0, 3, PaginationMode::Callees)
                .expect("paginate failed");

            let formatted = format_focused_paginated(
                &paginated.items,
                total_callees,
                PaginationMode::Callees,
                "target",
                &output.prod_chains,
                &output.test_chains,
                &output.outgoing_chains,
                output.def_count,
                0,
                Some(temp_dir.path()),
                true,
            );

            assert!(
                formatted.contains(&format!(
                    "CALLEES (1-{} of {})",
                    paginated.items.len(),
                    total_callees
                )),
                "header should show callees range, got: {}",
                formatted
            );
        }
    }

    #[test]
    fn test_symbol_focus_empty_prod_callers() {
        let temp_dir = TempDir::new().unwrap();

        // target is only called from test functions
        let code = r#"
fn target() {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_something() { target(); }
}
"#;
        fs::write(temp_dir.path().join("lib.rs"), code).unwrap();

        let output = analyze_focused(temp_dir.path(), "target", 1, None, None).unwrap();

        // prod_chains may be empty; pagination should handle it gracefully
        let paginated = paginate_slice(&output.prod_chains, 0, 100, PaginationMode::Callers)
            .expect("paginate failed");
        assert_eq!(paginated.items.len(), output.prod_chains.len());
        assert!(
            paginated.next_cursor.is_none(),
            "no next_cursor for empty or single-page prod_chains"
        );
    }

    #[test]
    fn test_impl_only_filter_header_correct_counts() {
        let temp_dir = TempDir::new().unwrap();

        // Create a Rust fixture with:
        // - A trait definition
        // - An impl Trait for SomeType block that calls the focus symbol
        // - A regular (non-trait-impl) function that also calls the focus symbol
        let code = r#"
trait MyTrait {
    fn focus_symbol();
}

struct SomeType;

impl MyTrait for SomeType {
    fn focus_symbol() {}
}

fn impl_caller() {
    SomeType::focus_symbol();
}

fn regular_caller() {
    SomeType::focus_symbol();
}
"#;
        fs::write(temp_dir.path().join("lib.rs"), code).unwrap();

        // Call analyze_focused with impl_only=Some(true)
        let params = FocusedAnalysisConfig {
            focus: "focus_symbol".to_string(),
            match_mode: SymbolMatchMode::Insensitive,
            follow_depth: 1,
            max_depth: None,
            ast_recursion_limit: None,
            use_summary: false,
            impl_only: Some(true),
        };
        let output = analyze_focused_with_progress(
            temp_dir.path(),
            &params,
            Arc::new(AtomicUsize::new(0)),
            CancellationToken::new(),
        )
        .unwrap();

        // Assert the result contains "FILTER: impl_only=true"
        assert!(
            output.formatted.contains("FILTER: impl_only=true"),
            "formatted output should contain FILTER header for impl_only=true, got: {}",
            output.formatted
        );

        // Assert the retained count N < total count M
        assert!(
            output.impl_trait_caller_count < output.unfiltered_caller_count,
            "impl_trait_caller_count ({}) should be less than unfiltered_caller_count ({})",
            output.impl_trait_caller_count,
            output.unfiltered_caller_count
        );

        // Assert format is "FILTER: impl_only=true (N of M callers shown)"
        let filter_line = output
            .formatted
            .lines()
            .find(|line| line.contains("FILTER: impl_only=true"))
            .expect("should find FILTER line");
        assert!(
            filter_line.contains(&format!(
                "({} of {} callers shown)",
                output.impl_trait_caller_count, output.unfiltered_caller_count
            )),
            "FILTER line should show correct N of M counts, got: {}",
            filter_line
        );
    }

    #[test]
    fn test_callers_count_matches_formatted_output() {
        let temp_dir = TempDir::new().unwrap();

        // Create a file with multiple callers of `target`
        let code = r#"
fn target() {}
fn caller_a() { target(); }
fn caller_b() { target(); }
fn caller_c() { target(); }
"#;
        fs::write(temp_dir.path().join("lib.rs"), code).unwrap();

        // Analyze the symbol
        let output = analyze_focused(temp_dir.path(), "target", 1, None, None).unwrap();

        // Extract CALLERS count from formatted output
        let formatted = &output.formatted;
        let callers_count_from_output = formatted
            .lines()
            .find(|line| line.contains("FOCUS:"))
            .and_then(|line| {
                line.split(',')
                    .find(|part| part.contains("callers"))
                    .and_then(|part| {
                        part.trim()
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.parse::<usize>().ok())
                    })
            })
            .expect("should find CALLERS count in formatted output");

        // Compute expected count from prod_chains (unique first-caller names)
        let expected_callers_count = output
            .prod_chains
            .iter()
            .filter_map(|chain| chain.chain.first().map(|(name, _, _)| name))
            .collect::<std::collections::HashSet<_>>()
            .len();

        assert_eq!(
            callers_count_from_output, expected_callers_count,
            "CALLERS count in formatted output should match unique-first-caller count in prod_chains"
        );
    }
}
