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
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, CancelledNotificationParam, CompleteRequestParams, CompleteResult,
    CompletionInfo, Content, ErrorData, Implementation, InitializeResult, LoggingLevel,
    LoggingMessageNotificationParam, Notification, NumberOrString, ProgressNotificationParam,
    ProgressToken, ProtocolVersion, ServerCapabilities, ServerNotification, SetLevelRequestParams,
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
use types::{AnalysisMode, AnalyzeParams};

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

/// Helper function to create the output schema for the analyze tool
fn create_analyze_output_schema() -> std::sync::Arc<serde_json::Map<String, Value>> {
    use serde_json::json;
    let schema = json!({
        "type": "object",
        "properties": {
            "mode": {
                "type": "string",
                "enum": ["overview", "file_details", "symbol_focus"],
                "description": "The analysis mode used"
            },
            "formatted": {
                "type": "string",
                "description": "Formatted text output of the analysis"
            },
            "files": {
                "type": "array",
                "description": "List of files analyzed (overview mode only)",
                "items": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "language": { "type": "string" },
                        "loc": { "type": "integer" },
                        "functions": { "type": "integer" },
                        "classes": { "type": "integer" }
                    }
                }
            },
            "semantic": {
                "type": "object",
                "description": "Semantic analysis data (file_details mode only)",
                "properties": {
                    "functions": { "type": "array" },
                    "classes": { "type": "array" },
                    "imports": { "type": "array" }
                }
            },
            "line_count": {
                "type": "integer",
                "description": "Total line count (file_details mode only)"
            },
            "next_cursor": {
                "type": ["string", "null"],
                "description": "Pagination cursor for next page of results"
            }
        },
        "required": ["formatted"]
    });

    if let Value::Object(map) = schema {
        std::sync::Arc::new(map)
    } else {
        std::sync::Arc::new(serde_json::Map::new())
    }
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

    #[instrument(skip(self, context))]
    #[tool(
        title = "Code Structure Analyzer",
        description = "Analyze code structure in 3 modes: 1) Overview - directory tree with LOC/function/class counts (use max_depth to limit). 2) FileDetails - functions, classes, imports for one file. 3) SymbolFocus - call graph for a named symbol across a directory (requires focus, case-sensitive). Typical flow: directory overview -> file details -> symbol focus. For large overview output (>50K chars), use summary=true to get totals and top-level structure without per-file detail; output auto-summarizes at the 50K threshold. Use cursor/page_size to paginate files (overview) or functions (file_details) when next_cursor appears in the response. Functions called >3x show N.",
        output_schema = create_analyze_output_schema(),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn analyze(
        &self,
        params: Parameters<AnalyzeParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;
        let ct = context.ct.clone();

        // Determine mode if not provided
        let mode = params
            .mode
            .unwrap_or_else(|| analyze::determine_mode(&params.path, params.focus.as_deref()));

        // Dispatch based on mode and construct ModeResult
        let mode_result = match mode {
            AnalysisMode::Overview => {
                let path = Path::new(&params.path);
                let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let counter_clone = counter.clone();
                let path_owned = path.to_path_buf();
                let max_depth = params.max_depth;
                let ct_clone = ct.clone();

                // Get total file count for progress reporting
                let total_files = match walk_directory(path, max_depth) {
                    Ok(entries) => entries.iter().filter(|e| !e.is_dir).count(),
                    Err(_) => 0,
                };

                // Spawn blocking analysis with progress tracking
                let handle = tokio::task::spawn_blocking(move || {
                    analyze::analyze_directory_with_progress(
                        &path_owned,
                        max_depth,
                        counter_clone,
                        ct_clone,
                    )
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
                    Ok(Ok(output)) => types::ModeResult::Overview(output),
                    Ok(Err(analyze::AnalyzeError::Cancelled)) => {
                        let output = analyze::AnalysisOutput {
                            formatted: "Analysis cancelled".to_string(),
                            files: vec![],
                            entries: vec![],
                            next_cursor: None,
                        };
                        types::ModeResult::Overview(output)
                    }
                    Ok(Err(e)) => {
                        let output = analyze::AnalysisOutput {
                            formatted: format!("Error analyzing directory: {}", e),
                            files: vec![],
                            entries: vec![],
                            next_cursor: None,
                        };
                        types::ModeResult::Overview(output)
                    }
                    Err(e) => {
                        let output = analyze::AnalysisOutput {
                            formatted: format!("Task join error: {}", e),
                            files: vec![],
                            entries: vec![],
                            next_cursor: None,
                        };
                        types::ModeResult::Overview(output)
                    }
                }
            }
            AnalysisMode::FileDetails => {
                // Build cache key from file metadata
                let cache_key = std::fs::metadata(&params.path).ok().and_then(|meta| {
                    meta.modified().ok().map(|mtime| cache::CacheKey {
                        path: std::path::PathBuf::from(&params.path),
                        modified: mtime,
                        mode: AnalysisMode::FileDetails,
                    })
                });

                // Check cache first
                let output_result = if let Some(ref key) = cache_key {
                    if let Some(cached) = self.cache.get(key) {
                        Ok(cached)
                    } else {
                        // Cache miss, analyze and store
                        match analyze::analyze_file(&params.path, params.ast_recursion_limit) {
                            Ok(output) => {
                                let arc_output = std::sync::Arc::new(output);
                                self.cache.put(key.clone(), arc_output.clone());
                                Ok(arc_output)
                            }
                            Err(e) => Err(format!("Error analyzing file: {}", e)),
                        }
                    }
                } else {
                    // No cache key available, analyze directly
                    match analyze::analyze_file(&params.path, params.ast_recursion_limit) {
                        Ok(output) => Ok(std::sync::Arc::new(output)),
                        Err(e) => Err(format!("Error analyzing file: {}", e)),
                    }
                };

                match output_result {
                    Ok(output) => types::ModeResult::FileDetails((*output).clone()),
                    Err(e) => {
                        let output = analyze::FileAnalysisOutput {
                            formatted: e,
                            semantic: types::SemanticAnalysis {
                                functions: vec![],
                                classes: vec![],
                                imports: vec![],
                                references: vec![],
                                call_frequency: std::collections::HashMap::new(),
                                calls: vec![],
                                assignments: vec![],
                                field_accesses: vec![],
                            },
                            line_count: 0,
                            next_cursor: None,
                        };
                        types::ModeResult::FileDetails(output)
                    }
                }
            }
            AnalysisMode::SymbolFocus => {
                let focus = params.focus.as_deref().unwrap_or("");
                let follow_depth = params.follow_depth.unwrap_or(1);
                let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let counter_clone = counter.clone();
                let path = Path::new(&params.path);
                let path_owned = path.to_path_buf();
                let max_depth = params.max_depth;
                let focus_owned = focus.to_string();
                let ast_recursion_limit = params.ast_recursion_limit;
                let ct_clone = ct.clone();

                // Compute use_summary before spawning: explicit params only
                // (auto-detect requires full output first, handled after task)
                let use_summary_for_task =
                    params.force != Some(true) && params.summary == Some(true);

                // Get total file count for progress reporting
                let total_files = match walk_directory(path, max_depth) {
                    Ok(entries) => entries.iter().filter(|e| !e.is_dir).count(),
                    Err(_) => 0,
                };

                // Spawn blocking analysis with progress tracking
                let handle = tokio::task::spawn_blocking(move || {
                    analyze::analyze_focused_with_progress(
                        &path_owned,
                        &focus_owned,
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
                                current, total_files, focus
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
                            total_files, focus
                        ),
                    )
                    .await;
                }

                match handle.await {
                    Ok(Ok(output)) => types::ModeResult::SymbolFocus(output),
                    Ok(Err(analyze::AnalyzeError::Cancelled)) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: "Analysis cancelled".to_string(),
                            next_cursor: None,
                            prod_chains: vec![],
                            test_chains: vec![],
                            outgoing_chains: vec![],
                            def_count: 0,
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                    Ok(Err(e)) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: format!("Error analyzing symbol focus: {}", e),
                            next_cursor: None,
                            prod_chains: vec![],
                            test_chains: vec![],
                            outgoing_chains: vec![],
                            def_count: 0,
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                    Err(e) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: format!("Task join error: {}", e),
                            next_cursor: None,
                            prod_chains: vec![],
                            test_chains: vec![],
                            outgoing_chains: vec![],
                            def_count: 0,
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                }
            }
        };

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

        // Convert ModeResult to text-only content with pagination and capture structured JSON
        let (formatted_text, next_cursor, structured_value) = match mode_result {
            types::ModeResult::Overview(mut output) => {
                // Apply summary/output size limiting logic
                // Determine if we should use summary
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

                // Apply pagination to files
                let paginated =
                    paginate_slice(&output.files, offset, page_size, PaginationMode::Default)
                        .map_err(|e| {
                            ErrorData::new(
                                rmcp::model::ErrorCode::INTERNAL_ERROR,
                                e.to_string(),
                                None,
                            )
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

                // Serialize after all mutations
                let structured = serde_json::to_value(&output).unwrap_or(Value::Null);
                (output.formatted, paginated.next_cursor, structured)
            }
            types::ModeResult::FileDetails(mut output) => {
                // Apply summary/output size limiting logic (same 3-way decision as Overview)
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
                    output.formatted = format_file_details_summary(
                        &output.semantic,
                        &params.path,
                        output.line_count,
                    );
                } else if output.formatted.len() > SIZE_LIMIT && params.force != Some(true) {
                    let estimated_tokens = output.formatted.len() / 4;
                    let message = format!(
                        "Output exceeds 50K chars ({} chars, ~{} tokens). Use one of:\n\
                         - force=true to return full output\n\
                         - Narrow your scope (smaller directory, specific file)\n\
                         - Use symbol_focus mode for targeted analysis\n\
                         - Reduce max_depth parameter",
                        output.formatted.len(),
                        estimated_tokens
                    );
                    return Err(ErrorData::new(
                        rmcp::model::ErrorCode::INVALID_REQUEST,
                        message,
                        None,
                    ));
                }

                // Paginate functions (typically the largest collection)
                let paginated = paginate_slice(
                    &output.semantic.functions,
                    offset,
                    page_size,
                    PaginationMode::Default,
                )
                .map_err(|e| {
                    ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                })?;

                // Regenerate formatted output from the paginated slice when pagination is active
                if paginated.next_cursor.is_some() || offset > 0 {
                    output.formatted = format_file_details_paginated(
                        &paginated.items,
                        paginated.total,
                        &output.semantic,
                        &params.path,
                        output.line_count,
                        offset,
                    );
                }

                // Update next_cursor in output after pagination
                output.next_cursor = paginated.next_cursor.clone();

                // Serialize after all mutations
                let structured = serde_json::to_value(&output).unwrap_or(Value::Null);
                (output.formatted, paginated.next_cursor, structured)
            }
            types::ModeResult::SymbolFocus(mut output) => {
                // Auto-detect: if no explicit summary param and output exceeds limit,
                // re-run analysis with use_summary=true (double-compute, acceptable)
                if params.summary.is_none()
                    && params.force != Some(true)
                    && output.formatted.len() > SIZE_LIMIT
                {
                    let path_owned2 = Path::new(&params.path).to_path_buf();
                    let focus_owned2 = params.focus.clone().unwrap_or_default();
                    let follow_depth2 = params.follow_depth.unwrap_or(1);
                    let max_depth2 = params.max_depth;
                    let ast_recursion_limit2 = params.ast_recursion_limit;
                    let counter2 = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
                    let ct2 = ct.clone();
                    let summary_result = tokio::task::spawn_blocking(move || {
                        analyze::analyze_focused_with_progress(
                            &path_owned2,
                            &focus_owned2,
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
                            let focus_symbol = params.focus.as_deref().unwrap_or("");
                            let base_path = Path::new(&params.path);
                            output.formatted = format_focused_paginated(
                                &paginated_items,
                                output.prod_chains.len(),
                                PaginationMode::Callers,
                                focus_symbol,
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
                            let focus_symbol = params.focus.as_deref().unwrap_or("");
                            let base_path = Path::new(&params.path);
                            output.formatted = format_focused_paginated(
                                &paginated_items,
                                output.outgoing_chains.len(),
                                PaginationMode::Callees,
                                focus_symbol,
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

                let structured = serde_json::to_value(&output).unwrap_or(Value::Null);
                (output.formatted, paginated_next_cursor, structured)
            }
        };

        // Build final text output with pagination cursor if present
        let mut final_text = formatted_text;
        if let Some(cursor) = next_cursor {
            final_text.push('\n');
            final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
        }

        let mut result = CallToolResult::success(vec![Content::text(final_text)]);
        result.structured_content = Some(structured_value);
        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for CodeAnalyzer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_logging()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_completions()
                .build(),
        )
        .with_protocol_version(ProtocolVersion::V_2025_06_18)
        .with_server_info(
            Implementation::new("code-analyze-mcp", "0.1.0")
                .with_title("Code Analyze MCP")
                .with_description("MCP server for code structure analysis using tree-sitter"),
        )
        .with_instructions("Use overview mode to map a codebase (pass a directory). Use file_details mode to extract functions, classes, and imports from a specific file (pass a file path). Use symbol_focus mode to trace call graphs for a named function or class (pass a directory and set focus to the symbol name, case-sensitive). Prefer summary=true on large directories to reduce output size. When the response includes next_cursor, pass it back as cursor to retrieve the next page.")
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
        // Dispatch on argument name: "path" or "focus"
        let argument_name = &request.argument.name;
        let argument_value = &request.argument.value;

        let completions = match argument_name.as_str() {
            "path" => {
                // Path completions: use current directory as root
                let root = Path::new(".");
                completion::path_completions(root, argument_value)
            }
            "focus" => {
                // Focus completions: need the path argument from context
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
