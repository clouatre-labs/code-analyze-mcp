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
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use tracing::instrument;
use types::AnalyzeParams;

use analyze::{analyze_directory, determine_mode};
use types::AnalysisMode;

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
        let path = std::path::Path::new(&params.path);
        let mode = determine_mode(path, &params);
        let max_depth = params.max_depth.unwrap_or(0) as usize;

        let output = match mode {
            AnalysisMode::Overview => analyze_directory(path, max_depth),
            _ => format!(
                "Mode '{:?}' is not yet implemented. Provide a directory path for structure overview.",
                mode
            ),
        };

        let assistant_content =
            RawContent::text(output.clone()).with_audience(vec![Role::Assistant]);
        let user_content = RawContent::text(output)
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
