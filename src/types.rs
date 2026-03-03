use crate::analyze::{AnalysisOutput, FileAnalysisOutput, FocusedAnalysisOutput};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Internal enum wrapping the three analysis output types.
/// Not serialized; used for type-safe dispatch in lib.rs.
#[derive(Debug)]
pub enum ModeResult {
    Overview(AnalysisOutput),
    FileDetails(FileAnalysisOutput),
    SymbolFocus(FocusedAnalysisOutput),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    #[schemars(description = "Path to the file or directory to analyze")]
    pub path: String,

    #[schemars(
        description = "Analysis mode: 'overview', 'file_details', or 'symbol_focus' (auto-detected if not provided)"
    )]
    #[serde(default)]
    pub mode: Option<AnalysisMode>,

    #[schemars(description = "Maximum recursion depth for directory traversal")]
    pub max_depth: Option<u32>,

    #[schemars(description = "Symbol to focus on for symbol_focus mode")]
    pub focus: Option<String>,

    #[schemars(description = "Call graph depth for symbol_focus mode")]
    pub follow_depth: Option<u32>,

    #[schemars(description = "Maximum AST recursion depth for tree-sitter queries")]
    pub ast_recursion_limit: Option<usize>,

    #[schemars(description = "Bypass output size limiting (default: false)")]
    pub force: Option<bool>,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportInfo {
    #[schemars(
        description = "Full module path excluding the imported symbol (e.g., 'std::collections' for 'use std::collections::HashMap')"
    )]
    pub module: String,
    #[schemars(
        description = "Imported symbols (e.g., ['HashMap'] for 'use std::collections::HashMap')"
    )]
    pub items: Vec<String>,
    #[schemars(description = "Line number where import appears")]
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SemanticAnalysis {
    #[schemars(description = "Functions with parameters and return types")]
    pub functions: Vec<FunctionInfo>,
    #[schemars(description = "Classes/structs")]
    pub classes: Vec<ClassInfo>,
    #[schemars(
        description = "Flat list of imports; each entry carries its full module path and imported symbols"
    )]
    pub imports: Vec<ImportInfo>,
    #[schemars(description = "Type references with location information")]
    pub references: Vec<ReferenceInfo>,
    #[schemars(description = "Call frequency map (function name -> count)")]
    pub call_frequency: HashMap<String, usize>,
    #[schemars(description = "Caller-callee pairs extracted from call expressions")]
    pub calls: Vec<CallInfo>,
}
