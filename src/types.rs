use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    #[schemars(description = "Path to the file or directory to analyze")]
    pub path: String,

    #[schemars(description = "Analysis mode: 'overview', 'file_details', or 'symbol_focus'. Auto-detected from path when omitted.")]
    pub mode: Option<AnalysisMode>,

    #[schemars(description = "Maximum recursion depth for directory traversal")]
    pub max_depth: Option<u32>,

    #[schemars(description = "Symbol to focus on for symbol_focus mode")]
    pub focus: Option<String>,

    #[schemars(description = "Call graph depth for symbol_focus mode")]
    pub follow_depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalysisResult {
    pub path: String,
    pub mode: AnalysisMode,
    pub import_count: usize,
    pub main_line: Option<usize>,
    pub files: Vec<FileInfo>,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub references: Vec<ReferenceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileInfo {
    pub path: String,
    pub language: String,
    pub line_count: usize,
    pub function_count: usize,
    pub class_count: usize,
}

/// Internal representation of a single directory-walk entry used by the
/// formatter and the analyzer.  Not part of the public JSON API.
pub struct FileResult {
    pub relative_path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    pub language: Option<String>,
    pub line_count: usize,
    pub function_count: usize,
    pub class_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    pub methods: Vec<FunctionInfo>,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallInfo {
    pub caller: String,
    pub callee: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReferenceInfo {
    pub symbol: String,
    pub reference_type: ReferenceType,
    pub location: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReferenceType {
    Definition,
    Usage,
    Import,
    Export,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    File,
    Directory,
    Function,
    Class,
    Variable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    Overview,
    FileDetails,
    SymbolFocus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallChain {
    pub chain: Vec<CallInfo>,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FocusedAnalysisData {
    pub symbol: String,
    pub definition: Option<FunctionInfo>,
    pub call_chains: Vec<CallChain>,
    pub references: Vec<ReferenceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ElementQueryResult {
    pub query: String,
    pub results: Vec<String>,
    pub count: usize,
}
