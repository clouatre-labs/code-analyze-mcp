use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

#[allow(unused_imports)]
use crate::analyze::{AnalysisOutput, FileAnalysisOutput, FocusedAnalysisOutput};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeDirectoryParams {
    #[schemars(description = "Directory path to analyze")]
    pub path: String,

    #[schemars(
        description = "Maximum directory traversal depth for overview mode only. 0 or unset = unlimited depth. Use 1-3 for large monorepos to manage output size. Ignored in other modes."
    )]
    pub max_depth: Option<u32>,

    #[schemars(
        description = "Return full output even when it exceeds the 50K char limit. Prefer summary=true or narrowing scope over force=true; force=true can produce very large responses."
    )]
    pub force: Option<bool>,

    #[schemars(
        description = "true = compact summary (totals plus directory tree, no per-file function lists); false = full output; unset = auto-summarize when output exceeds 50K chars."
    )]
    pub summary: Option<bool>,

    #[schemars(
        description = "Pagination cursor from a previous response's next_cursor field. Pass unchanged to retrieve the next page. Omit on the first call."
    )]
    pub cursor: Option<String>,

    #[schemars(
        description = "Files per page for pagination (default: 100). Reduce below 100 to limit response size; increase above 100 to reduce round trips."
    )]
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeFileParams {
    #[schemars(description = "File path to analyze")]
    pub path: String,

    #[schemars(
        description = "Maximum AST node depth for tree-sitter queries. Internal tuning parameter; leave unset in normal use. Increase only if query results are missing constructs in deeply nested or generated code."
    )]
    pub ast_recursion_limit: Option<usize>,

    #[schemars(
        description = "Return full output even when it exceeds the 50K char limit. Prefer summary=true or narrowing scope over force=true; force=true can produce very large responses."
    )]
    pub force: Option<bool>,

    #[schemars(
        description = "true = compact summary (no per-function details); false = full output; unset = auto-summarize when output exceeds 50K chars."
    )]
    pub summary: Option<bool>,

    #[schemars(
        description = "Pagination cursor from a previous response's next_cursor field. Pass unchanged to retrieve the next page. Omit on the first call."
    )]
    pub cursor: Option<String>,

    #[schemars(
        description = "Items per page for pagination (default: 100). Items are functions. Reduce below 100 to limit response size; increase above 100 to reduce round trips."
    )]
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeSymbolParams {
    #[schemars(description = "Directory path to search for the symbol")]
    pub path: String,

    #[schemars(
        description = "Symbol name to build call graph for (function or method). Exact case-sensitive match required; searched across all files in the specified directory. Example: 'parse_config' finds all callers and callees of that function."
    )]
    pub symbol: String,

    #[schemars(
        description = "Call graph traversal depth for this tool (default 1). Level 1 = direct callers and callees; level 2 = one more hop, etc. Output size grows exponentially with graph branching. Warn user on levels above 2."
    )]
    pub follow_depth: Option<u32>,

    #[schemars(
        description = "Maximum directory traversal depth. Unset means unlimited. Use 2-3 for large monorepos."
    )]
    pub max_depth: Option<u32>,

    #[schemars(
        description = "Maximum AST node depth for tree-sitter queries. Internal tuning parameter; leave unset in normal use. Increase only if query results are missing constructs in deeply nested or generated code."
    )]
    pub ast_recursion_limit: Option<usize>,

    #[schemars(
        description = "Return full output even when it exceeds the 50K char limit. Prefer summary=true or narrowing scope over force=true; force=true can produce very large responses."
    )]
    pub force: Option<bool>,

    #[schemars(
        description = "true = compact summary; false = full output; unset = auto-summarize when output exceeds 50K chars."
    )]
    pub summary: Option<bool>,

    #[schemars(
        description = "Pagination cursor from a previous response's next_cursor field. Pass unchanged to retrieve the next page. Omit on the first call."
    )]
    pub cursor: Option<String>,

    #[schemars(
        description = "Items per page for pagination (default: 100). Callers and callees are paginated separately. Reduce below 100 to limit response size; increase above 100 to reduce round trips."
    )]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Number of arguments passed at the call site")]
    pub arg_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssignmentInfo {
    #[schemars(description = "Variable name being assigned")]
    pub variable: String,
    #[schemars(description = "Value expression being assigned")]
    pub value: String,
    #[schemars(description = "Line number where assignment occurs")]
    pub line: usize,
    #[schemars(description = "Enclosing function scope or 'global'")]
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FieldAccessInfo {
    #[schemars(description = "Object expression being accessed")]
    pub object: String,
    #[schemars(description = "Field name being accessed")]
    pub field: String,
    #[schemars(description = "Line number where field access occurs")]
    pub line: usize,
    #[schemars(description = "Enclosing function scope or 'global'")]
    pub scope: String,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(description = "Variable assignments and reassignments")]
    pub assignments: Vec<AssignmentInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(description = "Field access patterns")]
    pub field_accesses: Vec<FieldAccessInfo>,
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
