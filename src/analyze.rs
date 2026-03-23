//! Main analysis engine for extracting code structure from files and directories.
//!
//! Implements the four MCP tools: `analyze_directory` (Overview), `analyze_file` (FileDetails),
//! `analyze_symbol` (call graph), and `analyze_module` (lightweight index). Handles parallel processing and cancellation.

use crate::formatter::{
    format_file_details, format_focused, format_focused_summary, format_structure,
};
use crate::graph::{CallGraph, InternalCallChain, resolve_symbol};
use crate::lang::language_from_extension;
use crate::parser::{ElementExtractor, SemanticExtractor, extract_impl_traits};
use crate::test_detection::is_test_file;
use crate::traversal::{WalkEntry, walk_directory};
use crate::types::{
    AnalysisMode, FileInfo, ImplTraitInfo, ImportInfo, SemanticAnalysis, SymbolMatchMode,
};
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
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AnalysisOutput {
    #[schemars(description = "Formatted text representation of the analysis")]
    pub formatted: String,
    #[schemars(description = "List of files analyzed in the directory")]
    pub files: Vec<FileInfo>,
    /// Walk entries used internally for summary generation; not serialized.
    #[serde(skip)]
    #[schemars(skip)]
    pub entries: Vec<WalkEntry>,
    /// Subtree file counts computed from an unbounded walk; used by format_summary; not serialized.
    #[serde(skip)]
    #[schemars(skip)]
    pub subtree_counts: Option<Vec<(std::path::PathBuf, usize)>>,
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
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
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
    let formatted = format_structure(&entries, &analysis_results, None, Some(root));

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
    /// Production caller chains (partitioned from incoming chains, excluding test callers).
    /// Not serialized; used for pagination in lib.rs.
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) prod_chains: Vec<InternalCallChain>,
    /// Test caller chains. Not serialized; used for pagination summary in lib.rs.
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) test_chains: Vec<InternalCallChain>,
    /// Outgoing (callee) chains. Not serialized; used for pagination in lib.rs.
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) outgoing_chains: Vec<InternalCallChain>,
    /// Number of definitions for the symbol. Not serialized; used for pagination headers.
    #[serde(skip)]
    #[schemars(skip)]
    pub def_count: usize,
    /// Total unique callers before impl_only filter. Not serialized; used for FILTER header.
    #[serde(skip)]
    #[schemars(skip)]
    pub unfiltered_caller_count: usize,
    /// Unique callers after impl_only filter. Not serialized; used for FILTER header.
    #[serde(skip)]
    #[schemars(skip)]
    pub impl_trait_caller_count: usize,
}

/// Analyze a symbol's call graph across a directory with progress tracking.
#[instrument(skip_all, fields(path = %root.display(), symbol = %focus))]
#[allow(clippy::too_many_arguments)]
pub fn analyze_focused_with_progress(
    root: &Path,
    focus: &str,
    match_mode: SymbolMatchMode,
    follow_depth: u32,
    max_depth: Option<u32>,
    ast_recursion_limit: Option<usize>,
    progress: Arc<AtomicUsize>,
    ct: CancellationToken,
    use_summary: bool,
    impl_only: Option<bool>,
) -> Result<FocusedAnalysisOutput, AnalyzeError> {
    #[allow(clippy::too_many_arguments)]
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
                    // Extract impl-trait blocks independently (Rust only; empty for other langs)
                    if language == "rust" {
                        semantic.impl_traits = extract_impl_traits(&source, &entry.path);
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

    // Collect all impl-trait info from analysis results
    let all_impl_traits: Vec<ImplTraitInfo> = analysis_results
        .iter()
        .flat_map(|(_, sem)| sem.impl_traits.iter().cloned())
        .collect();

    // Build call graph. Always build without impl_only filter first so we can
    // record the unfiltered caller count before discarding those edges.
    let mut graph = CallGraph::build_from_results(
        analysis_results,
        &all_impl_traits,
        false, // filter applied below after counting
    )?;

    // Resolve symbol name using the requested match mode.
    // Exact mode: check the graph directly without building a sorted set (O(1) lookups).
    // Fuzzy modes: collect a sorted, deduplicated set of all known symbols for deterministic results.
    let resolved_focus = if match_mode == SymbolMatchMode::Exact {
        let exists = graph.definitions.contains_key(focus)
            || graph.callers.contains_key(focus)
            || graph.callees.contains_key(focus);
        if exists {
            focus.to_string()
        } else {
            return Err(crate::graph::GraphError::SymbolNotFound {
                symbol: focus.to_string(),
                hint: "Try match_mode=insensitive for a case-insensitive search.".to_string(),
            }
            .into());
        }
    } else {
        let all_known: Vec<String> = graph
            .definitions
            .keys()
            .chain(graph.callers.keys())
            .chain(graph.callees.keys())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        resolve_symbol(all_known.iter(), focus, &match_mode)?
    };

    // Count unique callers for the focus symbol before applying impl_only filter.
    let unfiltered_caller_count = graph
        .callers
        .get(&resolved_focus)
        .map(|edges| {
            edges
                .iter()
                .map(|e| &e.neighbor_name)
                .collect::<std::collections::HashSet<_>>()
                .len()
        })
        .unwrap_or(0);

    // Apply impl_only filter now if requested, then count filtered callers.
    // Filter all caller adjacency lists so traversal and formatting are consistently
    // restricted to impl-trait edges regardless of follow_depth.
    let impl_trait_caller_count = if impl_only.unwrap_or(false) {
        for edges in graph.callers.values_mut() {
            edges.retain(|e| e.is_impl_trait);
        }
        graph
            .callers
            .get(&resolved_focus)
            .map(|edges| {
                edges
                    .iter()
                    .map(|e| &e.neighbor_name)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
            })
            .unwrap_or(0)
    } else {
        unfiltered_caller_count
    };

    // Compute chain data for pagination (always, regardless of summary mode)
    let def_count = graph
        .definitions
        .get(&resolved_focus)
        .map_or(0, |d| d.len());
    let incoming_chains = graph.find_incoming_chains(&resolved_focus, follow_depth)?;
    let outgoing_chains = graph.find_outgoing_chains(&resolved_focus, follow_depth)?;

    let (prod_chains, test_chains): (Vec<_>, Vec<_>) =
        incoming_chains.into_iter().partition(|chain| {
            chain
                .chain
                .first()
                .is_none_or(|(name, path, _)| !is_test_file(path) && !name.starts_with("test_"))
        });

    // Format output
    let formatted = if use_summary {
        format_focused_summary(&graph, &resolved_focus, follow_depth, Some(root))?
    } else {
        format_focused(&graph, &resolved_focus, follow_depth, Some(root))?
    };

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

/// Analyze a symbol's call graph with use_summary parameter (internal).
#[instrument(skip_all, fields(path = %root.display(), symbol = %focus))]
#[allow(clippy::too_many_arguments)]
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
        SymbolMatchMode::Exact,
        follow_depth,
        max_depth,
        ast_recursion_limit,
        counter,
        ct,
        false,
        None,
    )
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
        .and_then(language_from_extension)
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
/// For each import with items=["*"], this function:
/// 1. Parses the relative dots (if any) and climbs the directory tree
/// 2. Finds the target .py file or __init__.py
/// 3. Extracts symbols (functions and classes) from the target
/// 4. Honors __all__ if defined, otherwise uses function+class names
///
/// All resolution failures are non-fatal: debug-logged and the wildcard is preserved.
fn resolve_wildcard_imports(file_path: &Path, imports: &mut [ImportInfo]) {
    use std::collections::HashMap;

    let mut resolved_cache: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let file_path_canonical = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            tracing::debug!(file = ?file_path, "unable to canonicalize current file path");
            return;
        }
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

    let target_to_read = match locate_target_file(file_path, dot_count, module_path, &module) {
        Some(p) => p,
        None => return,
    };

    let canonical = match target_to_read.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            tracing::debug!(target = ?target_to_read, import = %module, "unable to canonicalize path");
            return;
        }
    };

    if canonical == file_path_canonical {
        tracing::debug!(target = ?canonical, import = %module, "cannot import from self");
        return;
    }

    if let Some(cached) = resolved_cache.get(&canonical) {
        tracing::debug!(import = %module, symbols_count = cached.len(), "using cached symbols");
        import.items = cached.clone();
        return;
    }

    if let Some(symbols) = parse_target_symbols(&target_to_read, &module) {
        tracing::debug!(import = %module, resolved_count = symbols.len(), "wildcard import resolved");
        import.items = symbols.clone();
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
    let source = match std::fs::read_to_string(target_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(target = ?target_path, import = %module, error = %e, "unable to read target file");
            return None;
        }
    };

    // Parse once with tree-sitter
    use tree_sitter::Parser;
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
        match child.kind() {
            "function_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = source[name_node.start_byte()..name_node.end_byte()].to_string();
                    if !name.starts_with('_') {
                        symbols.push(name);
                    }
                }
            }
            "class_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = source[name_node.start_byte()..name_node.end_byte()].to_string();
                    if !name.starts_with('_') {
                        symbols.push(name);
                    }
                }
            }
            _ => {}
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

#[cfg(test)]
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
}
