//! Rust MCP server for code structure analysis using tree-sitter.
//!
//! This crate provides three analysis modes for multiple programming languages:
//!
//! - **Overview**: Directory tree with file counts and structure
//! - **FileDetails**: Semantic extraction (functions, classes, assignments, references)
//! - **SymbolFocus**: Call graphs and dataflow (planned)
//!
//! Key types:
//! - [`analyze::analyze_directory`]: Analyze entire directory tree
//! - [`analyze::analyze_file`]: Analyze single file
//! - [`parser::ElementExtractor`]: Parse language-specific elements
//!
//! Languages supported: Rust, Go, Java, Python, TypeScript.

pub mod analyze;
pub mod cache;
pub mod completion;
pub mod formatter;
pub mod graph;
pub mod lang;
pub mod languages;
pub mod logging;
pub mod metrics;
pub mod pagination;
pub mod parser;
pub(crate) mod schema_helpers;
pub mod test_detection;
pub mod traversal;
pub mod types;

pub(crate) const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    ".git",
    "__pycache__",
    "target",
    "dist",
    "build",
    ".venv",
];

use cache::AnalysisCache;
use formatter::{
    format_file_details_paginated, format_file_details_summary, format_focused_paginated,
    format_module_info, format_structure_paginated, format_summary,
};
use logging::LogEvent;
use pagination::{
    CursorData, DEFAULT_PAGE_SIZE, PaginationMode, decode_cursor, encode_cursor, paginate_slice,
};
use rmcp::handler::server::tool::{ToolRouter, schema_for_type};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, CancelledNotificationParam, CompleteRequestParams, CompleteResult,
    CompletionInfo, Content, ErrorData, Implementation, InitializeResult, LoggingLevel,
    LoggingMessageNotificationParam, Meta, Notification, NumberOrString, ProgressNotificationParam,
    ProgressToken, ServerCapabilities, ServerNotification, SetLevelRequestParams,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{Peer, RoleServer, ServerHandler, tool, tool_handler, tool_router};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{Mutex as TokioMutex, mpsc};
use tracing::{instrument, warn};
use tracing_subscriber::filter::LevelFilter;
use traversal::walk_directory;
use types::{
    AnalysisMode, AnalyzeDirectoryParams, AnalyzeFileParams, AnalyzeModuleParams,
    AnalyzeSymbolParams,
};

static GLOBAL_SESSION_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const SIZE_LIMIT: usize = 50_000;

pub fn summary_cursor_conflict(summary: Option<bool>, cursor: Option<&str>) -> bool {
    summary == Some(true) && cursor.is_some()
}

fn error_meta(
    category: &'static str,
    is_retryable: bool,
    suggested_action: &'static str,
) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "errorCategory": category,
        "isRetryable": is_retryable,
        "suggestedAction": suggested_action,
    }))
}

fn err_to_tool_result(e: ErrorData) -> CallToolResult {
    CallToolResult::error(vec![Content::text(e.message)])
}

fn no_cache_meta() -> Meta {
    let mut m = serde_json::Map::new();
    m.insert(
        "cache_hint".to_string(),
        serde_json::Value::String("no-cache".to_string()),
    );
    Meta(m)
}

/// Helper function for paginating focus chains (callers or callees).
/// Returns (items, re-encoded_cursor_option).
fn paginate_focus_chains(
    chains: &[graph::CallChain],
    mode: PaginationMode,
    offset: usize,
    page_size: usize,
) -> Result<(Vec<graph::CallChain>, Option<String>), ErrorData> {
    let paginated = paginate_slice(chains, offset, page_size, mode).map_err(|e| {
        ErrorData::new(
            rmcp::model::ErrorCode::INTERNAL_ERROR,
            e.to_string(),
            error_meta("transient", true, "retry the request"),
        )
    })?;

    if paginated.next_cursor.is_none() && offset == 0 {
        return Ok((paginated.items, None));
    }

    let next = if let Some(raw_cursor) = paginated.next_cursor {
        let decoded = decode_cursor(&raw_cursor).map_err(|e| {
            ErrorData::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                e.to_string(),
                error_meta("validation", false, "invalid cursor format"),
            )
        })?;
        Some(
            encode_cursor(&CursorData {
                mode,
                offset: decoded.offset,
            })
            .map_err(|e| {
                ErrorData::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    e.to_string(),
                    error_meta("validation", false, "invalid cursor format"),
                )
            })?,
        )
    } else {
        None
    };

    Ok((paginated.items, next))
}

#[derive(Clone)]
pub struct CodeAnalyzer {
    tool_router: ToolRouter<Self>,
    cache: AnalysisCache,
    peer: Arc<TokioMutex<Option<Peer<RoleServer>>>>,
    log_level_filter: Arc<Mutex<LevelFilter>>,
    event_rx: Arc<TokioMutex<Option<mpsc::UnboundedReceiver<LogEvent>>>>,
    metrics_tx: crate::metrics::MetricsSender,
    session_call_seq: Arc<std::sync::atomic::AtomicU32>,
    session_id: Arc<TokioMutex<Option<String>>>,
}

#[tool_router]
impl CodeAnalyzer {
    pub fn list_tools() -> Vec<rmcp::model::Tool> {
        Self::tool_router().list_all()
    }

    pub fn new(
        peer: Arc<TokioMutex<Option<Peer<RoleServer>>>>,
        log_level_filter: Arc<Mutex<LevelFilter>>,
        event_rx: mpsc::UnboundedReceiver<LogEvent>,
        metrics_tx: crate::metrics::MetricsSender,
    ) -> Self {
        CodeAnalyzer {
            tool_router: Self::tool_router(),
            cache: AnalysisCache::new(100),
            peer,
            log_level_filter,
            event_rx: Arc::new(TokioMutex::new(Some(event_rx))),
            metrics_tx,
            session_call_seq: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            session_id: Arc::new(TokioMutex::new(None)),
        }
    }

    #[instrument(skip(self))]
    async fn emit_progress(
        &self,
        peer: Option<Peer<RoleServer>>,
        token: &ProgressToken,
        progress: f64,
        total: f64,
        message: String,
    ) {
        if let Some(peer) = peer {
            let notification = ServerNotification::ProgressNotification(Notification::new(
                ProgressNotificationParam {
                    progress_token: token.clone(),
                    progress,
                    total: Some(total),
                    message: Some(message),
                },
            ));
            if let Err(e) = peer.send_notification(notification).await {
                warn!("Failed to send progress notification: {}", e);
            }
        }
    }

    /// Private helper: Extract analysis logic for overview mode (analyze_directory).
    /// Returns the complete analysis output after spawning and monitoring progress.
    /// Cancels the blocking task when `ct` is triggered; returns an error on cancellation.
    #[instrument(skip(self, params, ct))]
    async fn handle_overview_mode(
        &self,
        params: &AnalyzeDirectoryParams,
        ct: tokio_util::sync::CancellationToken,
    ) -> Result<analyze::AnalysisOutput, ErrorData> {
        let path = Path::new(&params.path);
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let path_owned = path.to_path_buf();
        let max_depth = params.max_depth;
        let ct_clone = ct.clone();

        // Collect entries once for analysis
        let entries = walk_directory(path, max_depth).map_err(|e| {
            ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Failed to walk directory: {}", e),
                error_meta("resource", false, "check path permissions and availability"),
            )
        })?;

        // Get total file count for progress reporting
        let total_files = entries.iter().filter(|e| !e.is_dir).count();

        // Spawn blocking analysis with progress tracking
        let handle = tokio::task::spawn_blocking(move || {
            analyze::analyze_directory_with_progress(&path_owned, entries, counter_clone, ct_clone)
        });

        // Poll and emit progress every 100ms
        let token = ProgressToken(NumberOrString::String(
            format!(
                "analyze-overview-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            )
            .into(),
        ));
        let peer = self.peer.lock().await.clone();
        let mut last_progress = 0usize;
        let mut cancelled = false;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if ct.is_cancelled() {
                cancelled = true;
                break;
            }
            let current = counter.load(std::sync::atomic::Ordering::Relaxed);
            if current != last_progress && total_files > 0 {
                self.emit_progress(
                    peer.clone(),
                    &token,
                    current as f64,
                    total_files as f64,
                    format!("Analyzing {}/{} files", current, total_files),
                )
                .await;
                last_progress = current;
            }
            if handle.is_finished() {
                break;
            }
        }

        // Emit final 100% progress only if not cancelled
        if !cancelled && total_files > 0 {
            self.emit_progress(
                peer.clone(),
                &token,
                total_files as f64,
                total_files as f64,
                format!("Completed analyzing {} files", total_files),
            )
            .await;
        }

        match handle.await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(analyze::AnalyzeError::Cancelled)) => Err(ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                "Analysis cancelled".to_string(),
                error_meta("transient", true, "analysis was cancelled"),
            )),
            Ok(Err(e)) => Err(ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Error analyzing directory: {}", e),
                error_meta("resource", false, "check path and file permissions"),
            )),
            Err(e) => Err(ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Task join error: {}", e),
                error_meta("transient", true, "retry the request"),
            )),
        }
    }

    /// Private helper: Extract analysis logic for file details mode (analyze_file).
    /// Returns the cached or newly analyzed file output.
    #[instrument(skip(self, params))]
    async fn handle_file_details_mode(
        &self,
        params: &AnalyzeFileParams,
    ) -> Result<std::sync::Arc<analyze::FileAnalysisOutput>, ErrorData> {
        // Build cache key from file metadata
        let cache_key = std::fs::metadata(&params.path).ok().and_then(|meta| {
            meta.modified().ok().map(|mtime| cache::CacheKey {
                path: std::path::PathBuf::from(&params.path),
                modified: mtime,
                mode: AnalysisMode::FileDetails,
            })
        });

        // Check cache first
        if let Some(ref key) = cache_key
            && let Some(cached) = self.cache.get(key)
        {
            return Ok(cached);
        }

        // Cache miss or no cache key, analyze and optionally store
        match analyze::analyze_file(&params.path, params.ast_recursion_limit) {
            Ok(output) => {
                let arc_output = std::sync::Arc::new(output);
                if let Some(ref key) = cache_key {
                    self.cache.put(key.clone(), arc_output.clone());
                }
                Ok(arc_output)
            }
            Err(e) => Err(ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Error analyzing file: {}", e),
                error_meta("resource", false, "check file path and permissions"),
            )),
        }
    }

    /// Private helper: Extract analysis logic for focused mode (analyze_symbol).
    /// Returns the complete focused analysis output after spawning and monitoring progress.
    /// Cancels the blocking task when `ct` is triggered; returns an error on cancellation.
    #[instrument(skip(self, params, ct))]
    async fn handle_focused_mode(
        &self,
        params: &AnalyzeSymbolParams,
        ct: tokio_util::sync::CancellationToken,
    ) -> Result<analyze::FocusedAnalysisOutput, ErrorData> {
        let follow_depth = params.follow_depth.unwrap_or(1);
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let path = Path::new(&params.path);
        let path_owned = path.to_path_buf();
        let max_depth = params.max_depth;
        let symbol_owned = params.symbol.clone();
        let match_mode = params.match_mode.clone().unwrap_or_default();
        let ast_recursion_limit = params.ast_recursion_limit;
        let ct_clone = ct.clone();

        // Compute use_summary before spawning: explicit params only
        let use_summary_for_task = params.output_control.force != Some(true)
            && params.output_control.summary == Some(true);

        // Get total file count for progress reporting
        let total_files = match walk_directory(path, max_depth) {
            Ok(entries) => entries.iter().filter(|e| !e.is_dir).count(),
            Err(_) => 0,
        };

        // Spawn blocking analysis with progress tracking
        let handle = tokio::task::spawn_blocking(move || {
            analyze::analyze_focused_with_progress(
                &path_owned,
                &symbol_owned,
                match_mode,
                follow_depth,
                max_depth,
                ast_recursion_limit,
                counter_clone,
                ct_clone,
                use_summary_for_task,
            )
        });

        // Poll and emit progress every 100ms
        let token = ProgressToken(NumberOrString::String(
            format!(
                "analyze-symbol-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            )
            .into(),
        ));
        let peer = self.peer.lock().await.clone();
        let mut last_progress = 0usize;
        let mut cancelled = false;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if ct.is_cancelled() {
                cancelled = true;
                break;
            }
            let current = counter.load(std::sync::atomic::Ordering::Relaxed);
            if current != last_progress && total_files > 0 {
                self.emit_progress(
                    peer.clone(),
                    &token,
                    current as f64,
                    total_files as f64,
                    format!(
                        "Analyzing {}/{} files for symbol '{}'",
                        current, total_files, params.symbol
                    ),
                )
                .await;
                last_progress = current;
            }
            if handle.is_finished() {
                break;
            }
        }

        // Emit final 100% progress only if not cancelled
        if !cancelled && total_files > 0 {
            self.emit_progress(
                peer.clone(),
                &token,
                total_files as f64,
                total_files as f64,
                format!(
                    "Completed analyzing {} files for symbol '{}'",
                    total_files, params.symbol
                ),
            )
            .await;
        }

        let mut output = match handle.await {
            Ok(Ok(output)) => output,
            Ok(Err(analyze::AnalyzeError::Cancelled)) => {
                return Err(ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    "Analysis cancelled".to_string(),
                    error_meta("transient", true, "analysis was cancelled"),
                ));
            }
            Ok(Err(e)) => {
                return Err(ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    format!("Error analyzing symbol: {}", e),
                    error_meta("resource", false, "check symbol name and file"),
                ));
            }
            Err(e) => {
                return Err(ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    format!("Task join error: {}", e),
                    error_meta("transient", true, "retry the request"),
                ));
            }
        };

        // Auto-detect: if no explicit summary param and output exceeds limit,
        // re-run analysis with use_summary=true
        if params.output_control.summary.is_none()
            && params.output_control.force != Some(true)
            && output.formatted.len() > SIZE_LIMIT
        {
            let path_owned2 = Path::new(&params.path).to_path_buf();
            let symbol_owned2 = params.symbol.clone();
            let match_mode2 = params.match_mode.clone().unwrap_or_default();
            let follow_depth2 = params.follow_depth.unwrap_or(1);
            let max_depth2 = params.max_depth;
            let ast_recursion_limit2 = params.ast_recursion_limit;
            let counter2 = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let ct2 = ct.clone();
            let summary_result = tokio::task::spawn_blocking(move || {
                analyze::analyze_focused_with_progress(
                    &path_owned2,
                    &symbol_owned2,
                    match_mode2,
                    follow_depth2,
                    max_depth2,
                    ast_recursion_limit2,
                    counter2,
                    ct2,
                    true, // use_summary=true
                )
            })
            .await;
            match summary_result {
                Ok(Ok(summary_output)) => {
                    output.formatted = summary_output.formatted;
                }
                _ => {
                    // Fallback: return error (summary generation failed)
                    let estimated_tokens = output.formatted.len() / 4;
                    let message = format!(
                        "Output exceeds 50K chars ({} chars, ~{} tokens). Use summary=true or force=true.",
                        output.formatted.len(),
                        estimated_tokens
                    );
                    return Err(ErrorData::new(
                        rmcp::model::ErrorCode::INVALID_PARAMS,
                        message,
                        error_meta("validation", false, "use summary=true or force=true"),
                    ));
                }
            }
        } else if output.formatted.len() > SIZE_LIMIT
            && params.output_control.force != Some(true)
            && params.output_control.summary == Some(false)
        {
            // Explicit summary=false with large output: return error
            let estimated_tokens = output.formatted.len() / 4;
            let message = format!(
                "Output exceeds 50K chars ({} chars, ~{} tokens). Use one of:\n\
                 - force=true to return full output\n\
                 - summary=true to get compact summary\n\
                 - Narrow your scope (smaller directory, specific file)",
                output.formatted.len(),
                estimated_tokens
            );
            return Err(ErrorData::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                message,
                error_meta(
                    "validation",
                    false,
                    "use force=true, summary=true, or narrow scope",
                ),
            ));
        }

        Ok(output)
    }

    #[instrument(skip(self, context))]
    #[tool(
        name = "analyze_directory",
        description = "Analyze directory structure and code metrics for multi-file overview. Use this tool for directories; use analyze_file for a single file. Returns a tree with LOC, function count, class count, and test file markers. Respects .gitignore (results may differ from raw filesystem listing because .gitignore rules are applied). For repos with 1000+ files, use max_depth=2-3 and summary=true to stay within token budgets. Note: max_depth controls what is analyzed (traversal depth), while page_size controls how results are returned (chunking); these are independent. Strategy comparison: prefer pagination (page_size=50) over force=true to reduce per-call token overhead; use summary=true when counts and structure are sufficient and no pagination is needed; force=true is an escape hatch for exceptional cases. Empty directories return an empty tree with zero counts. Output auto-summarizes at 50K chars; use summary=true to force compact output. Paginate large results with cursor and page_size. Example queries: Analyze the src/ directory to understand module structure; What files are in the tests/ directory and how large are they? summary=true and cursor are mutually exclusive; passing both returns an error.",
        output_schema = schema_for_type::<analyze::AnalysisOutput>(),
        annotations(
            title = "Analyze Directory",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn analyze_directory(
        &self,
        params: Parameters<AnalyzeDirectoryParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let ct = context.ct.clone();
        let _t_start = std::time::Instant::now();
        let _param_path = params.path.clone();
        let _max_depth_val = params.max_depth;
        let _seq = self
            .session_call_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _sid = self.session_id.lock().await.clone();

        // Call handler for analysis and progress tracking
        let mut output = match self.handle_overview_mode(&params, ct).await {
            Ok(v) => v,
            Err(e) => return Ok(err_to_tool_result(e)),
        };

        // summary=true (explicit) and cursor are mutually exclusive.
        // Auto-summarization (summary=None + large output) must NOT block cursor pagination.
        if summary_cursor_conflict(
            params.output_control.summary,
            params.pagination.cursor.as_deref(),
        ) {
            return Ok(err_to_tool_result(ErrorData::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                "summary=true is incompatible with a pagination cursor; use one or the other"
                    .to_string(),
                error_meta("validation", false, "remove cursor or set summary=false"),
            )));
        }

        // Apply summary/output size limiting logic
        let use_summary = if params.output_control.force == Some(true) {
            false
        } else if params.output_control.summary == Some(true) {
            true
        } else if params.output_control.summary == Some(false) {
            false
        } else {
            output.formatted.len() > SIZE_LIMIT
        };

        if use_summary {
            let subtree_counts = if params.max_depth.is_some_and(|d| d > 0) {
                traversal::count_files_by_dir(std::path::Path::new(&params.path)).ok()
            } else {
                None
            };
            output.formatted = format_summary(
                &output.entries,
                &output.files,
                params.max_depth,
                Some(Path::new(&params.path)),
                subtree_counts.as_ref(),
            );
        }

        // Decode pagination cursor if provided
        let page_size = params.pagination.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        let offset = if let Some(ref cursor_str) = params.pagination.cursor {
            let cursor_data = match decode_cursor(cursor_str).map_err(|e| {
                ErrorData::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    e.to_string(),
                    error_meta("validation", false, "invalid cursor format"),
                )
            }) {
                Ok(v) => v,
                Err(e) => return Ok(err_to_tool_result(e)),
            };
            cursor_data.offset
        } else {
            0
        };

        // Apply pagination to files
        let paginated =
            match paginate_slice(&output.files, offset, page_size, PaginationMode::Default) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(err_to_tool_result(ErrorData::new(
                        rmcp::model::ErrorCode::INTERNAL_ERROR,
                        e.to_string(),
                        error_meta("transient", true, "retry the request"),
                    )));
                }
            };

        let verbose = params.output_control.verbose.unwrap_or(false);
        if !use_summary {
            output.formatted = format_structure_paginated(
                &paginated.items,
                paginated.total,
                params.max_depth,
                Some(Path::new(&params.path)),
                verbose,
            );
        }

        // Update next_cursor in output after pagination (unless using summary mode)
        if use_summary {
            output.next_cursor = None;
        } else {
            output.next_cursor = paginated.next_cursor.clone();
        }

        // Build final text output with pagination cursor if present (unless using summary mode)
        let mut final_text = output.formatted.clone();
        if !use_summary && let Some(cursor) = paginated.next_cursor {
            final_text.push('\n');
            final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
        }

        let mut result = CallToolResult::success(vec![Content::text(final_text.clone())])
            .with_meta(Some(no_cache_meta()));
        let structured = serde_json::to_value(&output).unwrap_or(Value::Null);
        result.structured_content = Some(structured);
        let _dur = _t_start.elapsed().as_millis() as u64;
        self.metrics_tx.send(crate::metrics::MetricEvent {
            ts: crate::metrics::unix_ms(),
            tool: "analyze_directory",
            duration_ms: _dur,
            output_chars: final_text.chars().count(),
            param_path_depth: crate::metrics::path_component_count(&_param_path),
            max_depth: _max_depth_val,
            result: "ok",
            error_type: None,
            session_id: _sid,
            seq: Some(_seq),
        });
        Ok(result)
    }

    #[instrument(skip(self, context))]
    #[tool(
        name = "analyze_file",
        description = "Extract semantic structure from a single source file only; pass a directory to analyze_directory instead. Returns functions with signatures, types, and line ranges; class and method definitions with inheritance, fields, and imports. Supported languages: Rust, Go, Java, Python, TypeScript, TSX; unsupported file extensions return an error. Common mistake: passing a directory path returns an error; use analyze_directory for directories. Generated code with deeply nested ASTs may exceed 50K chars; use summary=true to get counts only. Supports pagination for large files via cursor/page_size. Use summary=true for compact output. Example queries: What functions are defined in src/lib.rs?; Show me the classes and their methods in src/analyzer.py",
        output_schema = schema_for_type::<analyze::FileAnalysisOutput>(),
        annotations(
            title = "Analyze File",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn analyze_file(
        &self,
        params: Parameters<AnalyzeFileParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let _ct = context.ct.clone();
        let _t_start = std::time::Instant::now();
        let _param_path = params.path.clone();
        let _seq = self
            .session_call_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _sid = self.session_id.lock().await.clone();

        // Call handler for analysis and caching
        let arc_output = match self.handle_file_details_mode(&params).await {
            Ok(v) => v,
            Err(e) => return Ok(err_to_tool_result(e)),
        };

        // Clone only the two fields that may be mutated per-request (formatted and
        // next_cursor). The heavy SemanticAnalysis data is shared via Arc and never
        // modified, so we borrow it directly from the cached pointer.
        let mut formatted = arc_output.formatted.clone();
        let line_count = arc_output.line_count;

        // Apply summary/output size limiting logic
        let use_summary = if params.output_control.force == Some(true) {
            false
        } else if params.output_control.summary == Some(true) {
            true
        } else if params.output_control.summary == Some(false) {
            false
        } else {
            formatted.len() > SIZE_LIMIT
        };

        if use_summary {
            formatted = format_file_details_summary(&arc_output.semantic, &params.path, line_count);
        } else if formatted.len() > SIZE_LIMIT && params.output_control.force != Some(true) {
            let estimated_tokens = formatted.len() / 4;
            let message = format!(
                "Output exceeds 50K chars ({} chars, ~{} tokens). Use one of:\n\
                 - force=true to return full output\n\
                 - Narrow your scope (smaller directory, specific file)\n\
                 - Use analyze_symbol mode for targeted analysis\n\
                 - Reduce max_depth parameter",
                formatted.len(),
                estimated_tokens
            );
            return Ok(err_to_tool_result(ErrorData::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                message,
                error_meta("validation", false, "use force=true or narrow scope"),
            )));
        }

        // Decode pagination cursor if provided (analyze_file)
        let page_size = params.pagination.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        let offset = if let Some(ref cursor_str) = params.pagination.cursor {
            let cursor_data = match decode_cursor(cursor_str).map_err(|e| {
                ErrorData::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    e.to_string(),
                    error_meta("validation", false, "invalid cursor format"),
                )
            }) {
                Ok(v) => v,
                Err(e) => return Ok(err_to_tool_result(e)),
            };
            cursor_data.offset
        } else {
            0
        };

        // Filter to top-level functions only (exclude methods) before pagination
        let top_level_fns: Vec<crate::types::FunctionInfo> = arc_output
            .semantic
            .functions
            .iter()
            .filter(|func| {
                !arc_output
                    .semantic
                    .classes
                    .iter()
                    .any(|class| func.line >= class.line && func.end_line <= class.end_line)
            })
            .cloned()
            .collect();

        // Paginate top-level functions only
        let paginated =
            match paginate_slice(&top_level_fns, offset, page_size, PaginationMode::Default) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(err_to_tool_result(ErrorData::new(
                        rmcp::model::ErrorCode::INTERNAL_ERROR,
                        e.to_string(),
                        error_meta("transient", true, "retry the request"),
                    )));
                }
            };

        // Regenerate formatted output using the paginated formatter (handles verbose and pagination correctly)
        let verbose = params.output_control.verbose.unwrap_or(false);
        if !use_summary {
            formatted = format_file_details_paginated(
                &paginated.items,
                paginated.total,
                &arc_output.semantic,
                &params.path,
                line_count,
                offset,
                verbose,
            );
            // Append RELATED: section at handler layer (first page only)
            if offset == 0 {
                formatted.push_str(&crate::formatter::format_related_section(
                    std::path::Path::new(&params.path),
                    None,
                ));
            }
        }

        // Capture next_cursor from pagination result (unless using summary mode)
        let next_cursor = if use_summary {
            None
        } else {
            paginated.next_cursor.clone()
        };

        // Build final text output with pagination cursor if present (unless using summary mode)
        let mut final_text = formatted.clone();
        if !use_summary && let Some(ref cursor) = next_cursor {
            final_text.push('\n');
            final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
        }

        // Build the response output, sharing SemanticAnalysis from the Arc to avoid cloning it.
        let response_output = analyze::FileAnalysisOutput {
            formatted,
            semantic: arc_output.semantic.clone(),
            line_count,
            next_cursor,
        };

        let mut result = CallToolResult::success(vec![Content::text(final_text.clone())])
            .with_meta(Some(no_cache_meta()));
        let structured = serde_json::to_value(&response_output).unwrap_or(Value::Null);
        result.structured_content = Some(structured);
        let _dur = _t_start.elapsed().as_millis() as u64;
        self.metrics_tx.send(crate::metrics::MetricEvent {
            ts: crate::metrics::unix_ms(),
            tool: "analyze_file",
            duration_ms: _dur,
            output_chars: final_text.chars().count(),
            param_path_depth: crate::metrics::path_component_count(&_param_path),
            max_depth: None,
            result: "ok",
            error_type: None,
            session_id: _sid,
            seq: Some(_seq),
        });
        Ok(result)
    }

    #[instrument(skip(self, context))]
    #[tool(
        name = "analyze_symbol",
        description = "Build call graph for a named function or method across all files in a directory to trace a specific function's usage. Returns direct callers and callees. Default symbol lookup is case-sensitive exact-match (match_mode=exact); myFunc and myfunc are different symbols. If exact match fails, retry with match_mode=insensitive for a case-insensitive search. To list candidates matching a prefix, use match_mode=prefix. To find symbols containing a substring, use match_mode=contains. When prefix or contains matches multiple symbols, an error is returned listing all candidates so you can refine to a single match. A symbol unknown to the graph (not defined and not referenced) returns an error; a symbol that is defined but has no callers or callees returns empty chains without error. follow_depth warning: each increment can multiply output size exponentially; use follow_depth=1 for production use; follow_depth=2+ only for targeted deep dives. Use cursor/page_size to paginate call chains when results exceed page_size. Example queries: Find all callers of the parse_config function; Trace the call chain for MyClass.process_request up to 2 levels deep",
        output_schema = schema_for_type::<analyze::FocusedAnalysisOutput>(),
        annotations(
            title = "Analyze Symbol",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn analyze_symbol(
        &self,
        params: Parameters<AnalyzeSymbolParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let ct = context.ct.clone();
        let _t_start = std::time::Instant::now();
        let _param_path = params.path.clone();
        let _max_depth_val = params.follow_depth;
        let _seq = self
            .session_call_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _sid = self.session_id.lock().await.clone();

        // Call handler for analysis and progress tracking
        let mut output = match self.handle_focused_mode(&params, ct).await {
            Ok(v) => v,
            Err(e) => return Ok(err_to_tool_result(e)),
        };

        // Decode pagination cursor if provided (analyze_symbol)
        let page_size = params.pagination.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        let offset = if let Some(ref cursor_str) = params.pagination.cursor {
            let cursor_data = match decode_cursor(cursor_str).map_err(|e| {
                ErrorData::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    e.to_string(),
                    error_meta("validation", false, "invalid cursor format"),
                )
            }) {
                Ok(v) => v,
                Err(e) => return Ok(err_to_tool_result(e)),
            };
            cursor_data.offset
        } else {
            0
        };

        // SymbolFocus pagination: decode cursor mode to determine callers vs callees
        let cursor_mode = if let Some(ref cursor_str) = params.pagination.cursor {
            decode_cursor(cursor_str)
                .map(|c| c.mode)
                .unwrap_or(PaginationMode::Callers)
        } else {
            PaginationMode::Callers
        };

        let paginated_next_cursor = match cursor_mode {
            PaginationMode::Callers => {
                let (paginated_items, paginated_next) = match paginate_focus_chains(
                    &output.prod_chains,
                    PaginationMode::Callers,
                    offset,
                    page_size,
                ) {
                    Ok(v) => v,
                    Err(e) => return Ok(err_to_tool_result(e)),
                };

                let verbose = params.output_control.verbose.unwrap_or(false);
                if paginated_next.is_some() || offset > 0 || !verbose {
                    let base_path = Path::new(&params.path);
                    output.formatted = format_focused_paginated(
                        &paginated_items,
                        output.prod_chains.len(),
                        PaginationMode::Callers,
                        &params.symbol,
                        &output.prod_chains,
                        &output.test_chains,
                        &output.outgoing_chains,
                        output.def_count,
                        offset,
                        Some(base_path),
                        verbose,
                    );
                    paginated_next
                } else {
                    None
                }
            }
            PaginationMode::Callees => {
                let (paginated_items, paginated_next) = match paginate_focus_chains(
                    &output.outgoing_chains,
                    PaginationMode::Callees,
                    offset,
                    page_size,
                ) {
                    Ok(v) => v,
                    Err(e) => return Ok(err_to_tool_result(e)),
                };

                let verbose = params.output_control.verbose.unwrap_or(false);
                if paginated_next.is_some() || offset > 0 || !verbose {
                    let base_path = Path::new(&params.path);
                    output.formatted = format_focused_paginated(
                        &paginated_items,
                        output.outgoing_chains.len(),
                        PaginationMode::Callees,
                        &params.symbol,
                        &output.prod_chains,
                        &output.test_chains,
                        &output.outgoing_chains,
                        output.def_count,
                        offset,
                        Some(base_path),
                        verbose,
                    );
                    paginated_next
                } else {
                    None
                }
            }
            PaginationMode::Default => {
                unreachable!("SymbolFocus should only use Callers or Callees modes")
            }
        };

        // Build final text output with pagination cursor if present
        let mut final_text = output.formatted.clone();
        if let Some(cursor) = paginated_next_cursor {
            final_text.push('\n');
            final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
        }

        let mut result = CallToolResult::success(vec![Content::text(final_text.clone())])
            .with_meta(Some(no_cache_meta()));
        let structured = serde_json::to_value(&output).unwrap_or(Value::Null);
        result.structured_content = Some(structured);
        let _dur = _t_start.elapsed().as_millis() as u64;
        self.metrics_tx.send(crate::metrics::MetricEvent {
            ts: crate::metrics::unix_ms(),
            tool: "analyze_symbol",
            duration_ms: _dur,
            output_chars: final_text.chars().count(),
            param_path_depth: crate::metrics::path_component_count(&_param_path),
            max_depth: _max_depth_val,
            result: "ok",
            error_type: None,
            session_id: _sid,
            seq: Some(_seq),
        });
        Ok(result)
    }

    #[instrument(skip(self))]
    #[tool(
        name = "analyze_module",
        description = "Index functions and imports in a single source file with minimal token cost. Returns name, line_count, language, function names with line numbers, and import list only -- no signatures, no types, no call graphs, no references. ~75% smaller output than analyze_file. Use analyze_file when you need function signatures, types, or class details; use analyze_module when you only need a function/import index to orient in a file or survey many files in sequence. Use analyze_directory for multi-file overviews; use analyze_symbol to trace call graphs for a specific function. Supported languages: Rust, Go, Java, Python, TypeScript, TSX; unsupported extensions return an error. Example queries: What functions are defined in src/analyze.rs?; List all imports in src/lib.rs. Pagination, summary, force, and verbose parameters are not supported by this tool.",
        output_schema = schema_for_type::<types::ModuleInfo>(),
        annotations(
            title = "Analyze Module",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn analyze_module(
        &self,
        params: Parameters<AnalyzeModuleParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let _t_start = std::time::Instant::now();
        let _param_path = params.path.clone();
        let _seq = self
            .session_call_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _sid = self.session_id.lock().await.clone();

        // Issue 340: Guard against directory paths
        if std::fs::metadata(&params.path)
            .map(|m| m.is_dir())
            .unwrap_or(false)
        {
            let _dur = _t_start.elapsed().as_millis() as u64;
            self.metrics_tx.send(crate::metrics::MetricEvent {
                ts: crate::metrics::unix_ms(),
                tool: "analyze_module",
                duration_ms: _dur,
                output_chars: 0,
                param_path_depth: crate::metrics::path_component_count(&_param_path),
                max_depth: None,
                result: "error",
                error_type: Some("invalid_params".to_string()),
                session_id: _sid.clone(),
                seq: Some(_seq),
            });
            return Ok(err_to_tool_result(ErrorData::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                format!(
                    "'{}' is a directory. Use analyze_directory to analyze a directory, or pass a specific file path to analyze_module.",
                    params.path
                ),
                error_meta("validation", false, "use analyze_directory for directories"),
            )));
        }

        let module_info = match analyze::analyze_module_file(&params.path).map_err(|e| {
            ErrorData::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                format!("Failed to analyze module: {}", e),
                error_meta(
                    "validation",
                    false,
                    "ensure file exists, is readable, and has a supported extension",
                ),
            )
        }) {
            Ok(v) => v,
            Err(e) => return Ok(err_to_tool_result(e)),
        };

        let text = format_module_info(&module_info);
        let mut result = CallToolResult::success(vec![Content::text(text.clone())])
            .with_meta(Some(no_cache_meta()));
        let structured = match serde_json::to_value(&module_info).map_err(|e| {
            ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("serialization failed: {}", e),
                error_meta("internal", false, "report this as a bug"),
            )
        }) {
            Ok(v) => v,
            Err(e) => return Ok(err_to_tool_result(e)),
        };
        result.structured_content = Some(structured);
        let _dur = _t_start.elapsed().as_millis() as u64;
        self.metrics_tx.send(crate::metrics::MetricEvent {
            ts: crate::metrics::unix_ms(),
            tool: "analyze_module",
            duration_ms: _dur,
            output_chars: text.chars().count(),
            param_path_depth: crate::metrics::path_component_count(&_param_path),
            max_depth: None,
            result: "ok",
            error_type: None,
            session_id: _sid,
            seq: Some(_seq),
        });
        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for CodeAnalyzer {
    fn get_info(&self) -> InitializeResult {
        let excluded = crate::EXCLUDED_DIRS.join(", ");
        let instructions = format!(
            "Recommended workflow for unknown repositories:\n\
            1. Start with analyze_directory(path=<repo_root>, max_depth=2, summary=true) to identify the source package directory \
            (typically the largest directory by file count; exclude {excluded}).\n\
            2. Re-run analyze_directory(path=<source_package>, max_depth=2, summary=true) for a module map with per-package class and function counts.\n\
            3. Use analyze_file on key files identified in step 2 (prefer files with high class counts for framework entry points).\n\
            4. Use analyze_symbol to trace call graphs for specific functions found in step 3.\n\
            Use analyze_module for a minimal schema (name, line count, functions, imports) when token budget is critical. \
            Prefer summary=true on large directories (1000+ files). Set max_depth=2 for the first call; increase only if packages are too large to differentiate. \
            Paginate with cursor/page_size. For subagents: DISABLE_PROMPT_CACHING=1."
        );
        let capabilities = ServerCapabilities::builder()
            .enable_logging()
            .enable_tools()
            .enable_tool_list_changed()
            .enable_completions()
            .build();
        let server_info = Implementation::new("code-analyze-mcp", env!("CARGO_PKG_VERSION"))
            .with_title("Code Analyze MCP")
            .with_description("MCP server for code structure analysis using tree-sitter");
        InitializeResult::new(capabilities)
            .with_server_info(server_info)
            .with_instructions(&instructions)
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        let mut peer_lock = self.peer.lock().await;
        *peer_lock = Some(context.peer.clone());
        drop(peer_lock);

        // Generate session_id in MILLIS-N format
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let counter = GLOBAL_SESSION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let sid = format!("{}-{}", millis, counter);
        {
            let mut session_id_lock = self.session_id.lock().await;
            *session_id_lock = Some(sid);
        }
        self.session_call_seq
            .store(0, std::sync::atomic::Ordering::Relaxed);

        // Spawn consumer task to drain log events from channel with batching.
        let peer = self.peer.clone();
        let event_rx = self.event_rx.clone();

        tokio::spawn(async move {
            let rx = {
                let mut rx_lock = event_rx.lock().await;
                rx_lock.take()
            };

            if let Some(mut receiver) = rx {
                let mut buffer = Vec::with_capacity(64);
                loop {
                    // Drain up to 64 events from channel
                    receiver.recv_many(&mut buffer, 64).await;

                    if buffer.is_empty() {
                        // Channel closed, exit consumer task
                        break;
                    }

                    // Acquire peer lock once per batch
                    let peer_lock = peer.lock().await;
                    if let Some(peer) = peer_lock.as_ref() {
                        for log_event in buffer.drain(..) {
                            let notification = ServerNotification::LoggingMessageNotification(
                                Notification::new(LoggingMessageNotificationParam {
                                    level: log_event.level,
                                    logger: Some(log_event.logger),
                                    data: log_event.data,
                                }),
                            );
                            if let Err(e) = peer.send_notification(notification).await {
                                warn!("Failed to send logging notification: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }

    #[instrument(skip(self, _context))]
    async fn on_cancelled(
        &self,
        notification: CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) {
        tracing::info!(
            request_id = ?notification.request_id,
            reason = ?notification.reason,
            "Received cancellation notification"
        );
    }

    #[instrument(skip(self, _context))]
    async fn complete(
        &self,
        request: CompleteRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CompleteResult, ErrorData> {
        // Dispatch on argument name: "path" or "symbol"
        let argument_name = &request.argument.name;
        let argument_value = &request.argument.value;

        let completions = match argument_name.as_str() {
            "path" => {
                // Path completions: use current directory as root
                let root = Path::new(".");
                completion::path_completions(root, argument_value)
            }
            "symbol" => {
                // Symbol completions: need the path argument from context
                let path_arg = request
                    .context
                    .as_ref()
                    .and_then(|ctx| ctx.get_argument("path"));

                match path_arg {
                    Some(path_str) => {
                        let path = Path::new(path_str);
                        completion::symbol_completions(&self.cache, path, argument_value)
                    }
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        };

        // Create CompletionInfo with has_more flag if >100 results
        let total_count = completions.len() as u32;
        let (values, has_more) = if completions.len() > 100 {
            (completions.into_iter().take(100).collect(), true)
        } else {
            (completions, false)
        };

        let completion_info =
            match CompletionInfo::with_pagination(values, Some(total_count), has_more) {
                Ok(info) => info,
                Err(_) => {
                    // Graceful degradation: return empty on error
                    CompletionInfo::with_all_values(Vec::new())
                        .unwrap_or_else(|_| CompletionInfo::new(Vec::new()).unwrap())
                }
            };

        Ok(CompleteResult::new(completion_info))
    }

    async fn set_level(
        &self,
        params: SetLevelRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        let level_filter = match params.level {
            LoggingLevel::Debug => LevelFilter::DEBUG,
            LoggingLevel::Info => LevelFilter::INFO,
            LoggingLevel::Notice => LevelFilter::INFO,
            LoggingLevel::Warning => LevelFilter::WARN,
            LoggingLevel::Error => LevelFilter::ERROR,
            LoggingLevel::Critical => LevelFilter::ERROR,
            LoggingLevel::Alert => LevelFilter::ERROR,
            LoggingLevel::Emergency => LevelFilter::ERROR,
        };

        let mut filter_lock = self.log_level_filter.lock().unwrap();
        *filter_lock = level_filter;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emit_progress_none_peer_is_noop() {
        let peer = Arc::new(TokioMutex::new(None));
        let log_level_filter = Arc::new(Mutex::new(LevelFilter::INFO));
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (metrics_tx, _metrics_rx) = tokio::sync::mpsc::unbounded_channel();
        let analyzer = CodeAnalyzer::new(
            peer,
            log_level_filter,
            rx,
            crate::metrics::MetricsSender(metrics_tx),
        );
        let token = ProgressToken(NumberOrString::String("test".into()));
        // Should complete without panic
        analyzer
            .emit_progress(None, &token, 0.0, 10.0, "test".to_string())
            .await;
    }

    #[tokio::test]
    async fn test_handle_overview_mode_verbose_no_summary_block() {
        use crate::pagination::{PaginationMode, paginate_slice};
        use crate::types::{AnalyzeDirectoryParams, OutputControlParams, PaginationParams};
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();

        let peer = Arc::new(TokioMutex::new(None));
        let log_level_filter = Arc::new(Mutex::new(LevelFilter::INFO));
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (metrics_tx, _metrics_rx) = tokio::sync::mpsc::unbounded_channel();
        let analyzer = CodeAnalyzer::new(
            peer,
            log_level_filter,
            rx,
            crate::metrics::MetricsSender(metrics_tx),
        );

        let params = AnalyzeDirectoryParams {
            path: tmp.path().to_str().unwrap().to_string(),
            max_depth: None,
            pagination: PaginationParams {
                cursor: None,
                page_size: None,
            },
            output_control: OutputControlParams {
                summary: None,
                force: None,
                verbose: Some(true),
            },
        };

        let ct = tokio_util::sync::CancellationToken::new();
        let output = analyzer.handle_overview_mode(&params, ct).await.unwrap();

        // Replicate the handler's formatting path (the fix site)
        let use_summary = output.formatted.len() > SIZE_LIMIT; // summary=None, force=None, small output
        let paginated =
            paginate_slice(&output.files, 0, DEFAULT_PAGE_SIZE, PaginationMode::Default).unwrap();
        let verbose = true;
        let formatted = if !use_summary {
            format_structure_paginated(
                &paginated.items,
                paginated.total,
                params.max_depth,
                Some(std::path::Path::new(&params.path)),
                verbose,
            )
        } else {
            output.formatted.clone()
        };

        // After the fix: verbose=true must not emit the SUMMARY: block
        assert!(
            !formatted.contains("SUMMARY:"),
            "verbose=true must not emit SUMMARY: block; got: {}",
            &formatted[..formatted.len().min(300)]
        );
        assert!(
            formatted.contains("PAGINATED:"),
            "verbose=true must emit PAGINATED: header"
        );
        assert!(
            formatted.contains("FILES [LOC, FUNCTIONS, CLASSES]"),
            "verbose=true must emit FILES section header"
        );
    }
}
