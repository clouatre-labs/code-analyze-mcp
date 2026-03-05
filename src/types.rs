use crate::analyze::{AnalysisOutput, FileAnalysisOutput, FocusedAnalysisOutput};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

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

    #[schemars(
        description = "Generate compact summary instead of full output. true=force summary, false=force full, unset=auto-detect when output exceeds 50K chars"
    )]
    pub summary: Option<bool>,

    #[schemars(
        description = "Opaque cursor token for pagination (from previous response's next_cursor)"
    )]
    pub cursor: Option<String>,

    #[schemars(description = "Number of items per page (default: 100)")]
    pub page_size: Option<usize>,
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
    #[schemars(description = "Whether this file is a test file")]
    pub is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}

impl FunctionInfo {
    /// Maximum length for parameter display before truncation.
    const MAX_PARAMS_DISPLAY_LEN: usize = 80;
    /// Truncation point when parameters exceed MAX_PARAMS_DISPLAY_LEN.
    const TRUNCATION_POINT: usize = 77;

    /// Format function signature as a single-line string with truncation.
    /// Returns: `name(param1, param2, ...) -> return_type :start-end`
    /// Parameters are truncated to ~80 chars with '...' if needed.
    pub fn compact_signature(&self) -> String {
        let mut sig = String::with_capacity(self.name.len() + 40);
        sig.push_str(&self.name);
        sig.push('(');

        if !self.parameters.is_empty() {
            let params_str = self.parameters.join(", ");
            if params_str.len() > Self::MAX_PARAMS_DISPLAY_LEN {
                // Truncate at a safe char boundary to avoid panicking on multibyte UTF-8.
                let truncate_at = params_str.floor_char_boundary(Self::TRUNCATION_POINT);
                sig.push_str(&params_str[..truncate_at]);
                sig.push_str("...");
            } else {
                sig.push_str(&params_str);
            }
        }

        sig.push(')');

        if let Some(ret_type) = &self.return_type {
            sig.push_str(" -> ");
            sig.push_str(ret_type);
        }

        write!(sig, " :{}-{}", self.line, self.end_line).ok();
        sig
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    pub methods: Vec<FunctionInfo>,
    pub fields: Vec<String>,
    #[schemars(description = "Inherited types (parent classes, interfaces, trait bounds)")]
    pub inherits: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_signature_short_params() {
        let func = FunctionInfo {
            name: "add".to_string(),
            line: 10,
            end_line: 12,
            parameters: vec!["a: i32".to_string(), "b: i32".to_string()],
            return_type: Some("i32".to_string()),
        };

        let sig = func.compact_signature();
        assert_eq!(sig, "add(a: i32, b: i32) -> i32 :10-12");
    }

    #[test]
    fn test_compact_signature_long_params_truncation() {
        let func = FunctionInfo {
            name: "process".to_string(),
            line: 20,
            end_line: 50,
            parameters: vec![
                "config: ComplexConfigType".to_string(),
                "data: VeryLongDataStructureNameThatExceedsEightyCharacters".to_string(),
                "callback: Fn(Result) -> ()".to_string(),
            ],
            return_type: Some("Result<Output>".to_string()),
        };

        let sig = func.compact_signature();
        assert!(sig.contains("process("));
        assert!(sig.contains("..."));
        assert!(sig.contains("-> Result<Output>"));
        assert!(sig.contains(":20-50"));
    }

    #[test]
    fn test_compact_signature_empty_params() {
        let func = FunctionInfo {
            name: "main".to_string(),
            line: 1,
            end_line: 5,
            parameters: vec![],
            return_type: None,
        };

        let sig = func.compact_signature();
        assert_eq!(sig, "main() :1-5");
    }
}
