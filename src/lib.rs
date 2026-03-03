pub mod analyze;
pub mod cache;
pub mod formatter;
pub mod graph;
pub mod lang;
pub mod languages;
pub mod parser;
pub mod traversal;
pub mod types;

use cache::AnalysisCache;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{
    ErrorData, Implementation, InitializeResult, ProtocolVersion, ServerCapabilities,
};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use std::path::Path;
use tracing::instrument;
use types::{AnalysisMode, AnalysisResult, AnalyzeParams};

#[derive(Clone)]
pub struct CodeAnalyzer {
    tool_router: ToolRouter<Self>,
    cache: AnalysisCache,
}

#[tool_router]
impl CodeAnalyzer {
    pub fn new() -> Self {
        CodeAnalyzer {
            tool_router: Self::tool_router(),
            cache: AnalysisCache::new(100),
        }
    }

    #[instrument(skip(self))]
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
    ) -> Result<Json<AnalysisResult>, ErrorData> {
        let params = params.0;

        // Determine mode if not provided
        let mode = params
            .mode
            .unwrap_or_else(|| analyze::determine_mode(&params.path, params.focus.as_deref()));

        // Dispatch based on mode and construct ModeResult
        let mode_result = match mode {
            AnalysisMode::Overview => {
                let path = Path::new(&params.path);
                match analyze::analyze_directory(path, params.max_depth) {
                    Ok(output) => types::ModeResult::Overview(output),
                    Err(e) => {
                        let output = analyze::AnalysisOutput {
                            formatted: format!("Error analyzing directory: {}", e),
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
                match analyze::analyze_focused(
                    Path::new(&params.path),
                    focus,
                    follow_depth,
                    params.max_depth,
                    params.ast_recursion_limit,
                ) {
                    Ok(output) => types::ModeResult::SymbolFocus(output),
                    Err(e) => {
                        let output = analyze::FocusedAnalysisOutput {
                            formatted: format!("Error analyzing symbol focus: {}", e),
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

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for CodeAnalyzer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
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
}
