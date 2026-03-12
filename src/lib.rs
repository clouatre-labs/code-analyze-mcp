pub mod analyze;
pub mod cache;
pub mod completion;
pub mod dataflow;
pub mod formatter;
pub mod graph;
pub mod lang;
pub mod languages;
pub mod logging;
pub mod pagination;
pub mod parser;
pub mod test_detection;
pub mod traversal;
pub mod types;

use cache::AnalysisCache;
use formatter::{
    format_file_details_paginated, format_file_details_summary, format_focused_paginated,
    format_structure_paginated, format_summary,
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
    LoggingMessageNotificationParam, Notification, NumberOrString, ProgressNotificationParam,
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
use types::{AnalysisMode, AnalyzeDirectoryParams, AnalyzeFileParams, AnalyzeSymbolParams};

const SIZE_LIMIT: usize = 50_000;

/// Helper function for paginating focus chains (callers or callees).
/// Returns (items, re-encoded_cursor_option).
fn paginate_focus_chains(
    chains: &[graph::CallChain],
    mode: PaginationMode,
    offset: usize,
    page_size: usize,
) -> Result<(Vec<graph::CallChain>, Option<String>), ErrorData> {
    let paginated = paginate_slice(chains, offset, page_size, mode)
        .map_err(|e| ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

    if paginated.next_cursor.is_none() && offset == 0 {
        return Ok((paginated.items, None));
    }

    let next = if let Some(raw_cursor) = paginated.next_cursor {
        let decoded = decode_cursor(&raw_cursor).map_err(|e| {
            ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
        })?;
        Some(
            encode_cursor(&CursorData {
                mode,
                offset: decoded.offset,
            })
            .map_err(|e| {
                ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
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
}

#[tool_router]
impl CodeAnalyzer {
    pub fn new(
        peer: Arc<TokioMutex<Option<Peer<RoleServer>>>>,
        log_level_filter: Arc<Mutex<LevelFilter>>,
        event_rx: mpsc::UnboundedReceiver<LogEvent>,
    ) -> Self {
        CodeAnalyzer {
            tool_router: Self::tool_router(),
            cache: AnalysisCache::new(100),
            peer,
            log_level_filter,
            event_rx: Arc::new(TokioMutex::new(Some(event_rx))),
        }
    }

    #[instrument(skip(self))]
    async fn emit_progress(
        &self,
        token: &ProgressToken,
        progress: f64,
        total: f64,
        message: String,
    ) {
        let peer = self.peer.lock().await.clone();
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
                None,
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
                None,
            )),
            Ok(Err(e)) => Err(ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Error analyzing directory: {}", e),
                None,
            )),
            Err(e) => Err(ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Task join error: {}", e),
                None,
            )),
        }
    }

    /// Private helper: Extract analysis logic for file details mode (analyze_file).
    /// Returns the cached or newly analyzed file output.
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
                None,
            )),
        }
    }

    /// Private helper: Extract analysis logic for focused mode (analyze_symbol).
    /// Returns the complete focused analysis output after spawning and monitoring progress.
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
        let ast_recursion_limit = params.ast_recursion_limit;
        let ct_clone = ct.clone();

        // Compute use_summary before spawning: explicit params only
        let use_summary_for_task = params.force != Some(true) && params.summary == Some(true);

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
                    None,
                ));
            }
            Ok(Err(e)) => {
                return Err(ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    format!("Error analyzing symbol: {}", e),
                    None,
                ));
            }
            Err(e) => {
                return Err(ErrorData::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    format!("Task join error: {}", e),
                    None,
                ));
            }
        };

        // Auto-detect: if no explicit summary param and output exceeds limit,
        // re-run analysis with use_summary=true
        if params.summary.is_none()
            && params.force != Some(true)
            && output.formatted.len() > SIZE_LIMIT
        {
            let path_owned2 = Path::new(&params.path).to_path_buf();
            let symbol_owned2 = params.symbol.clone();
            let follow_depth2 = params.follow_depth.unwrap_or(1);
            let max_depth2 = params.max_depth;
            let ast_recursion_limit2 = params.ast_recursion_limit;
            let counter2 = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let ct2 = ct.clone();
            let summary_result = tokio::task::spawn_blocking(move || {
                analyze::analyze_focused_with_progress(
                    &path_owned2,
                    &symbol_owned2,
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
                        rmcp::model::ErrorCode::INVALID_REQUEST,
                        message,
                        None,
                    ));
                }
            }
        } else if output.formatted.len() > SIZE_LIMIT
            && params.force != Some(true)
            && params.summary == Some(false)
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
                rmcp::model::ErrorCode::INVALID_REQUEST,
                message,
                None,
            ));
        }

        Ok(output)
    }

    #[instrument(skip(self, context))]
    #[tool(
        name = "analyze_directory",
        description = "Analyze directory structure and code metrics. Returns a tree with LOC, function count, class count, and test file markers. Respects .gitignore. Use max_depth to limit traversal depth (recommended 2-3 for large monorepos). Output auto-summarizes at 50K chars; use summary=true to force compact output. Paginate large results with cursor and page_size.",
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

        // Call handler for analysis and progress tracking
        let mut output = self.handle_overview_mode(&params, ct).await?;

        // Apply summary/output size limiting logic
        let use_summary = if params.force == Some(true) {
            false
        } else if params.summary == Some(true) {
            true
        } else if params.summary == Some(false) {
            false
        } else {
            output.formatted.len() > SIZE_LIMIT
        };

        if use_summary {
            output.formatted = format_summary(
                &output.entries,
                &output.files,
                params.max_depth,
                Some(Path::new(&params.path)),
            );
        }

        // Decode pagination cursor if provided
        let page_size = params.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        let offset = if let Some(ref cursor_str) = params.cursor {
            let cursor_data = decode_cursor(cursor_str).map_err(|e| {
                ErrorData::new(rmcp::model::ErrorCode::INVALID_PARAMS, e.to_string(), None)
            })?;
            cursor_data.offset
        } else {
            0
        };

        // Apply pagination to files
        let paginated = paginate_slice(&output.files, offset, page_size, PaginationMode::Default)
            .map_err(|e| {
            ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
        })?;

        if paginated.next_cursor.is_some() || offset > 0 {
            output.formatted = format_structure_paginated(
                &paginated.items,
                paginated.total,
                params.max_depth,
                Some(Path::new(&params.path)),
            );
        }

        // Update next_cursor in output after pagination
        output.next_cursor = paginated.next_cursor.clone();

        // Build final text output with pagination cursor if present
        let mut final_text = output.formatted.clone();
        if let Some(cursor) = paginated.next_cursor {
            final_text.push('\n');
            final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
        }

        let mut result = CallToolResult::success(vec![Content::text(final_text)]);
        let structured = serde_json::to_value(&output).unwrap_or(Value::Null);
        result.structured_content = Some(structured);
        Ok(result)
    }

    #[instrument(skip(self, context))]
    #[tool(
        name = "analyze_file",
        description = "Extract semantic structure from a single source file. Returns functions with signatures, types, and line ranges; class and method definitions with inheritance, fields, and imports. Supports pagination for large files via cursor/page_size. Use summary=true for compact output.",
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

        // Call handler for analysis and caching
        let arc_output = self.handle_file_details_mode(&params).await?;

        // Clone only the two fields that may be mutated per-request (formatted and
        // next_cursor). The heavy SemanticAnalysis data is shared via Arc and never
        // modified, so we borrow it directly from the cached pointer.
        let mut formatted = arc_output.formatted.clone();
        let line_count = arc_output.line_count;

        // Apply summary/output size limiting logic
        let use_summary = if params.force == Some(true) {
            false
        } else if params.summary == Some(true) {
            true
        } else if params.summary == Some(false) {
            false
        } else {
            formatted.len() > SIZE_LIMIT
        };

        if use_summary {
            formatted = format_file_details_summary(&arc_output.semantic, &params.path, line_count);
        } else if formatted.len() > SIZE_LIMIT && params.force != Some(true) {
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
            return Err(ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                message,
                None,
            ));
        }

        // Decode pagination cursor if provided
        let page_size = params.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        let offset = if let Some(ref cursor_str) = params.cursor {
            let cursor_data = decode_cursor(cursor_str).map_err(|e| {
                ErrorData::new(rmcp::model::ErrorCode::INVALID_PARAMS, e.to_string(), None)
            })?;
            cursor_data.offset
        } else {
            0
        };

        // Paginate functions
        let paginated = paginate_slice(
            &arc_output.semantic.functions,
            offset,
            page_size,
            PaginationMode::Default,
        )
        .map_err(|e| ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None))?;

        // Regenerate formatted output from the paginated slice when pagination is active
        if paginated.next_cursor.is_some() || offset > 0 {
            formatted = format_file_details_paginated(
                &paginated.items,
                paginated.total,
                &arc_output.semantic,
                &params.path,
                line_count,
                offset,
            );
        }

        // Capture next_cursor from pagination result
        let next_cursor = paginated.next_cursor.clone();

        // Build final text output with pagination cursor if present
        let mut final_text = formatted.clone();
        if let Some(ref cursor) = next_cursor {
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

        let mut result = CallToolResult::success(vec![Content::text(final_text)]);
        let structured = serde_json::to_value(&response_output).unwrap_or(Value::Null);
        result.structured_content = Some(structured);
        Ok(result)
    }

    #[instrument(skip(self, context))]
    #[tool(
        name = "analyze_symbol",
        description = "Build call graph for a named function or method across all files in a directory. Returns direct callers and callees. Symbol lookup is case-sensitive exact-match. Use follow_depth to trace deeper chains. Use cursor/page_size to paginate call chains when results exceed page_size.",
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

        // Call handler for analysis and progress tracking
        let mut output = self.handle_focused_mode(&params, ct).await?;

        // Decode pagination cursor if provided
        let page_size = params.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        let offset = if let Some(ref cursor_str) = params.cursor {
            let cursor_data = decode_cursor(cursor_str).map_err(|e| {
                ErrorData::new(rmcp::model::ErrorCode::INVALID_PARAMS, e.to_string(), None)
            })?;
            cursor_data.offset
        } else {
            0
        };

        // SymbolFocus pagination: decode cursor mode to determine callers vs callees
        let cursor_mode = if let Some(ref cursor_str) = params.cursor {
            decode_cursor(cursor_str)
                .map(|c| c.mode)
                .unwrap_or(PaginationMode::Callers)
        } else {
            PaginationMode::Callers
        };

        let paginated_next_cursor = match cursor_mode {
            PaginationMode::Callers => {
                let (paginated_items, paginated_next) = paginate_focus_chains(
                    &output.prod_chains,
                    PaginationMode::Callers,
                    offset,
                    page_size,
                )?;

                if paginated_next.is_some() || offset > 0 {
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
                    );
                    paginated_next
                } else {
                    None
                }
            }
            PaginationMode::Callees => {
                let (paginated_items, paginated_next) = paginate_focus_chains(
                    &output.outgoing_chains,
                    PaginationMode::Callees,
                    offset,
                    page_size,
                )?;

                if paginated_next.is_some() || offset > 0 {
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

        let mut result = CallToolResult::success(vec![Content::text(final_text)]);
        let structured = serde_json::to_value(&output).unwrap_or(Value::Null);
        result.structured_content = Some(structured);
        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for CodeAnalyzer {
    fn get_info(&self) -> InitializeResult {
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
            .with_instructions("Use analyze_directory to map a codebase (pass a directory). Use analyze_file to extract functions, classes, and imports from a specific file (pass a file path). Use analyze_symbol to trace call graphs for a named function or class (pass a directory and set symbol to the function name, case-sensitive). Prefer summary=true on large directories to reduce output size. When the response includes next_cursor, pass it back as cursor to retrieve the next page.")
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        let mut peer_lock = self.peer.lock().await;
        *peer_lock = Some(context.peer.clone());
        drop(peer_lock);

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
