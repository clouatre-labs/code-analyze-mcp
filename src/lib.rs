pub mod analyze;
pub mod formatter;
pub mod lang;
pub mod languages;
pub mod parser;
pub mod traversal;
pub mod types;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    AnnotateAble, CallToolResult, ErrorData, Implementation, InitializeResult, ProtocolVersion,
    RawContent, Role,
};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use std::path::Path;
use tracing::instrument;
use types::{AnalysisMode, AnalysisResult, AnalyzeParams};

#[derive(Clone)]
pub struct CodeAnalyzer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CodeAnalyzer {
    pub fn new() -> Self {
        CodeAnalyzer {
            tool_router: Self::tool_router(),
        }
    }

    #[instrument(skip(self))]
    #[tool(
        description = "Analyze code structure in 3 modes: 1) Directory overview - file tree with LOC/function/class counts to max_depth. 2) File details - functions, classes, imports. 3) Symbol focus - call graphs across directory to max_depth (requires directory path, case-sensitive). Typical flow: directory → files → symbols. Functions called >3x show •N."
    )]
    async fn analyze(
        &self,
        params: Parameters<AnalyzeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Determine mode if not provided
        let mode = params
            .mode
            .unwrap_or_else(|| analyze::determine_mode(&params.path, params.focus.as_deref()));

        // Dispatch based on mode
        let (result_text, files, functions, classes, references, import_count) = match mode {
            AnalysisMode::Overview => {
                let path = Path::new(&params.path);
                match analyze::analyze_directory(path, params.max_depth) {
                    Ok(output) => (output.formatted, output.files, vec![], vec![], vec![], 0),
                    Err(e) => (
                        format!("Error analyzing directory: {}", e),
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        0,
                    ),
                }
            }
            AnalysisMode::FileDetails => {
                match analyze::analyze_file(&params.path, params.ast_recursion_limit) {
                    Ok(output) => {
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
                        let references = output
                            .semantic
                            .references
                            .iter()
                            .map(|r| types::ReferenceInfo {
                                symbol: r.clone(),
                                reference_type: types::ReferenceType::Usage,
                                location: params.path.clone(),
                                line: 0,
                            })
                            .collect();
                        (
                            output.formatted,
                            vec![],
                            functions,
                            classes,
                            references,
                            import_count,
                        )
                    }
                    Err(e) => (
                        format!("Error analyzing file: {}", e),
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        0,
                    ),
                }
            }
            AnalysisMode::SymbolFocus => (
                "Symbol focus mode not yet implemented".to_string(),
                vec![],
                vec![],
                vec![],
                vec![],
                0,
            ),
        };

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

        let json_output = serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string());

        let assistant_content = RawContent::text(json_output).with_audience(vec![Role::Assistant]);

        let user_content = RawContent::text(result_text)
            .with_audience(vec![Role::User])
            .with_priority(0.0);

        Ok(CallToolResult::success(vec![
            assistant_content,
            user_content,
        ]))
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
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: Default::default(),
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
            instructions: None,
        }
    }
}
