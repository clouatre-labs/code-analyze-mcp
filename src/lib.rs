pub mod analyze;
pub mod cache;
pub mod completion;
pub mod formatter;
pub mod graph;
pub mod lang;
pub mod languages;
pub mod logging;
pub mod parser;
pub mod traversal;
pub mod types;

use cache::AnalysisCache;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    CancelledNotificationParam, CompleteRequestParams, CompleteResult, CompletionInfo, ErrorData,
    Implementation, InitializeResult, LoggingLevel, Notification, NumberOrString,
    ProgressNotificationParam, ProgressToken, ProtocolVersion, ServerCapabilities,
    ServerNotification, SetLevelRequestParams,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{Peer, RoleServer, ServerHandler, tool, tool_handler, tool_router};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use tracing::{instrument, warn};
use tracing_subscriber::filter::LevelFilter;
use traversal::walk_directory;
use types::{AnalysisMode, AnalysisResult, AnalyzeParams};

#[derive(Clone)]
pub struct CodeAnalyzer {
    tool_router: ToolRouter<Self>,
    cache: AnalysisCache,
    peer: Arc<TokioMutex<Option<Peer<RoleServer>>>>,
    log_level_filter: Arc<Mutex<LevelFilter>>,
}

#[tool_router]
impl CodeAnalyzer {
    pub fn new(
        peer: Arc<TokioMutex<Option<Peer<RoleServer>>>>,
        log_level_filter: Arc<Mutex<LevelFilter>>,
    ) -> Self {
        CodeAnalyzer {
            tool_router: Self::tool_router(),
            cache: AnalysisCache::new(100),
            peer,
            log_level_filter,
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
    ) -> Result<Json<AnalysisResult>, ErrorData> {
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
                        };
                        types::ModeResult::Overview(output)
                    }
                    Ok(Err(e)) => {
                        let output = analyze::AnalysisOutput {
                            formatted: format!("Error analyzing directory: {}", e),
                            files: vec![],
                        };
                        types::ModeResult::Overview(output)
                    }
                    Err(e) => {
                        let output = analyze::AnalysisOutput {
                            formatted: format!("Task join error: {}", e),
                            files: vec![],
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
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                    Ok(Err(e)) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: format!("Error analyzing symbol focus: {}", e),
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                    Err(e) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: format!("Task join error: {}", e),
                        };
                        types::ModeResult::SymbolFocus(output)
                    }
                }
            }
        };

        // Extract fields from ModeResult
        let (formatted_output, files, functions, classes, references, import_count) =
            match mode_result {
                types::ModeResult::Overview(output) => {
                    (output.formatted, output.files, vec![], vec![], vec![], 0)
                }
                types::ModeResult::FileDetails(output) => {
                    let import_count = output.semantic.imports.len();
                    let functions = output
                        .semantic
                        .functions
                        .iter()
                        .map(|f| types::FunctionInfo {
                            name: f.name.clone(),
                            line: f.line,
                            end_line: f.end_line,
                            parameters: f.parameters.clone(),
                            return_type: f.return_type.clone(),
                        })
                        .collect();
                    let classes = output
                        .semantic
                        .classes
                        .iter()
                        .map(|c| types::ClassInfo {
                            name: c.name.clone(),
                            line: c.line,
                            end_line: c.end_line,
                            methods: c.methods.clone(),
                            fields: c.fields.clone(),
                            kind: c.kind.clone(),
                        })
                        .collect();
                    let references = output.semantic.references.clone();
                    (
                        output.formatted,
                        vec![],
                        functions,
                        classes,
                        references,
                        import_count,
                    )
                }
                types::ModeResult::SymbolFocus(output) => {
                    (output.formatted, vec![], vec![], vec![], vec![], 0)
                }
            };

        // Apply output size limiting
        let line_count = formatted_output.lines().count();
        if line_count > 1000 && params.force != Some(true) {
            let estimated_tokens = line_count * 40;
            let message = format!(
                "Output exceeds 1000 lines ({} lines, ~{} tokens). Use one of:\n\
                 - force=true to return full output\n\
                 - Narrow your scope (smaller directory, specific file)\n\
                 - Use symbol_focus mode for targeted analysis\n\
                 - Reduce max_depth parameter",
                line_count, estimated_tokens
            );
            return Err(ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                message,
                None,
            ));
        }

        let result = AnalysisResult {
            path: params.path.clone(),
            mode,
            import_count,
            main_line: None,
            files,
            functions,
            classes,
            references,
        };

        Ok(Json(result))
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
        *peer_lock = Some(context.peer);
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
