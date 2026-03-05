pub mod analyze;
pub mod cache;
pub mod completion;
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
use formatter::{format_structure_paginated, format_summary};
use logging::LogEvent;
use pagination::{DEFAULT_PAGE_SIZE, decode_cursor, paginate_slice};
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
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::{Mutex as TokioMutex, mpsc};
use tracing::{instrument, warn};
use tracing_subscriber::filter::LevelFilter;
use traversal::walk_directory;
use types::{AnalysisMode, AnalyzeParams};

const SIZE_LIMIT: usize = 50_000;

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
        description = "Analyze code structure in 3 modes: 1) Directory overview - file tree with LOC/function/class counts to max_depth. 2) File details - functions, classes, imports. 3) Symbol focus - call graphs across directory to max_depth (requires directory path, case-sensitive). Typical flow: directory → files → symbols. Functions called >3x show •N.",
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
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                    Ok(Err(e)) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: format!("Error analyzing symbol focus: {}", e),
                            next_cursor: None,
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                    Err(e) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: format!("Task join error: {}", e),
                            next_cursor: None,
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

        // Convert ModeResult to text-only content with pagination
        let (formatted_text, next_cursor) = match mode_result {
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
                    output.formatted =
                        format_summary(&output.entries, &output.files, params.max_depth);
                }

                // Apply pagination to files
                let paginated = paginate_slice(&output.files, offset, page_size).map_err(|e| {
                    ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                })?;

                if paginated.next_cursor.is_some() || offset > 0 {
                    output.formatted = format_structure_paginated(
                        &paginated.items,
                        paginated.total,
                        params.max_depth,
                    );
                }

                (output.formatted, paginated.next_cursor)
            }
            types::ModeResult::FileDetails(output) => {
                // Apply output size limiting
                if output.formatted.len() > SIZE_LIMIT && params.force != Some(true) {
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
                let paginated = paginate_slice(&output.semantic.functions, offset, page_size)
                    .map_err(|e| {
                        ErrorData::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None)
                    })?;

                (output.formatted, paginated.next_cursor)
            }
            types::ModeResult::SymbolFocus(output) => {
                // Apply output size limiting
                if output.formatted.len() > SIZE_LIMIT && params.force != Some(true) {
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
                // SymbolFocus: no semantic data to paginate
                (output.formatted, output.next_cursor)
            }
        };

        // Build final text output with pagination cursor if present
        let mut final_text = formatted_text;
        if let Some(cursor) = next_cursor {
            final_text.push('\n');
            final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
        }

        Ok(CallToolResult::success(vec![Content::text(final_text)]))
    }
}

#[tool_handler]
impl ServerHandler for CodeAnalyzer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder()
                .enable_logging()
                .enable_tools()
                .enable_completions()
                .build(),
            server_info: Implementation {
                name: "code-analyze-mcp".into(),
                version: "0.1.0".into(),
                description: Some(
                    "MCP server for code structure analysis using tree-sitter".into(),
                ),
                title: Some("Code Analyze MCP".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some("Analyze code structure using three modes: directory overview (file tree with metrics), file details (functions/classes/imports), or symbol focus (call graphs). Provide a path and optionally specify mode and max_depth.".into()),
        }
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

        Ok(CompleteResult {
            completion: completion_info,
        })
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
