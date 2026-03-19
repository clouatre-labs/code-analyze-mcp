use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

#[allow(unused_imports)]
use crate::analyze::{AnalysisOutput, FileAnalysisOutput, FocusedAnalysisOutput};

/// Pagination parameters shared across all tools.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaginationParams {
    /// Pagination cursor from a previous response's next_cursor field. Pass unchanged to retrieve the next page. Omit on the first call.
    pub cursor: Option<String>,
    /// Files per page for pagination (default: 100). Reduce below 100 to limit response size; increase above 100 to reduce round trips.
    #[schemars(schema_with = "crate::schema_helpers::option_page_size_schema")]
    pub page_size: Option<usize>,
}

/// Output control parameters shared across all tools.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OutputControlParams {
    /// Return full output even when it exceeds the 50K char limit. Prefer summary=true or narrowing scope over force=true; force=true can produce very large responses.
    pub force: Option<bool>,
    /// true = compact summary (totals plus directory tree, no per-file function lists); false = full output; unset = auto-summarize when output exceeds 50K chars.
    pub summary: Option<bool>,
    /// true = full output with section headers and imports (Markdown-style); false or unset = compact one-line-per-item format (default).
    pub verbose: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeDirectoryParams {
    /// Directory path to analyze
    pub path: String,

    /// Maximum directory traversal depth for overview mode only. 0 or unset = unlimited depth. Use 1-3 for large monorepos to manage output size. Ignored in other modes.
    #[schemars(schema_with = "crate::schema_helpers::option_integer_schema")]
    pub max_depth: Option<u32>,

    #[serde(flatten)]
    pub pagination: PaginationParams,

    #[serde(flatten)]
    pub output_control: OutputControlParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeFileParams {
    /// File path to analyze
    pub path: String,

    /// Maximum AST node depth for tree-sitter queries. Internal tuning parameter; leave unset in normal use. Increase only if query results are missing constructs in deeply nested or generated code.
    #[schemars(schema_with = "crate::schema_helpers::option_ast_limit_schema")]
    pub ast_recursion_limit: Option<usize>,

    #[serde(flatten)]
    pub pagination: PaginationParams,

    #[serde(flatten)]
    pub output_control: OutputControlParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeModuleParams {
    /// File path to analyze
    pub path: String,
}

/// Symbol name matching strategy for analyze_symbol.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SymbolMatchMode {
    /// Case-sensitive exact match (default). Preserves all existing behaviour.
    #[default]
    Exact,
    /// Case-insensitive exact match. Useful when casing is unknown.
    Insensitive,
    /// Case-insensitive prefix match. Returns all symbols whose name starts with the query.
    Prefix,
    /// Case-insensitive substring match. Returns all symbols whose name contains the query.
    Contains,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeSymbolParams {
    /// Directory path to search for the symbol
    pub path: String,

    /// Symbol name to build call graph for (function or method). Example: 'parse_config' finds all callers and callees of that function.
    pub symbol: String,

    /// Symbol matching mode (default: exact). exact: case-sensitive exact match. insensitive: case-insensitive exact match. prefix: case-insensitive prefix match. contains: case-insensitive substring match. When exact match fails, retry with insensitive. When prefix or contains returns multiple candidates, the response lists them so you can refine.
    pub match_mode: Option<SymbolMatchMode>,

    /// Call graph traversal depth for this tool (default 1). Level 1 = direct callers and callees; level 2 = one more hop, etc. Output size grows exponentially with graph branching. Warn user on levels above 2.
    #[schemars(schema_with = "crate::schema_helpers::option_integer_schema")]
    pub follow_depth: Option<u32>,

    /// Maximum directory traversal depth. Unset means unlimited. Use 2-3 for large monorepos.
    #[schemars(schema_with = "crate::schema_helpers::option_integer_schema")]
    pub max_depth: Option<u32>,

    /// Maximum AST node depth for tree-sitter queries. Internal tuning parameter; leave unset in normal use. Increase only if query results are missing constructs in deeply nested or generated code.
    #[schemars(schema_with = "crate::schema_helpers::option_ast_limit_schema")]
    pub ast_recursion_limit: Option<usize>,

    #[serde(flatten)]
    pub pagination: PaginationParams,

    #[serde(flatten)]
    pub output_control: OutputControlParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalysisResult {
    pub path: String,
    pub mode: AnalysisMode,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub import_count: usize,
    #[schemars(schema_with = "crate::schema_helpers::option_integer_schema")]
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
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line_count: usize,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub function_count: usize,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub class_count: usize,
    /// Whether this file is a test file.
    pub is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FunctionInfo {
    pub name: String,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line: usize,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub end_line: usize,
    /// Parameter list as string representations (e.g., ["x: i32", "y: String"]).
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
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line: usize,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub end_line: usize,
    pub methods: Vec<FunctionInfo>,
    pub fields: Vec<String>,
    /// Inherited types (parent classes, interfaces, trait bounds).
    #[schemars(description = "Inherited types (parent classes, interfaces, trait bounds)")]
    pub inherits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallInfo {
    pub caller: String,
    pub callee: String,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line: usize,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub column: usize,
    /// Number of arguments passed at the call site.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "crate::schema_helpers::option_integer_schema")]
    pub arg_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssignmentInfo {
    /// Variable name being assigned
    pub variable: String,
    /// Value expression being assigned
    pub value: String,
    /// Line number where assignment occurs
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line: usize,
    /// Enclosing function scope or 'global'
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FieldAccessInfo {
    /// Object expression being accessed
    pub object: String,
    /// Field name being accessed
    pub field: String,
    /// Line number where field access occurs
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line: usize,
    /// Enclosing function scope or 'global'
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReferenceInfo {
    pub symbol: String,
    pub reference_type: ReferenceType,
    pub location: String,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
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

/// Analysis mode for generating output.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    /// High-level directory structure and file counts.
    Overview,
    /// Detailed semantic analysis of functions, classes, and references within a file.
    FileDetails,
    /// Call graph and dataflow analysis focused on a specific symbol.
    SymbolFocus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallChain {
    pub chain: Vec<CallInfo>,
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
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
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportInfo {
    /// Full module path excluding the imported symbol (e.g., 'std::collections' for 'use std::collections::HashMap').
    pub module: String,
    /// Imported symbols (e.g., ['HashMap'] for 'use std::collections::HashMap').
    pub items: Vec<String>,
    /// Line number where import appears.
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SemanticAnalysis {
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    /// Flat list of imports; each entry carries its full module path and imported symbols.
    pub imports: Vec<ImportInfo>,
    pub references: Vec<ReferenceInfo>,
    /// Call frequency map (function name -> count).
    #[serde(skip)]
    #[schemars(skip)]
    pub call_frequency: HashMap<String, usize>,
    /// Caller-callee pairs extracted from call expressions.
    pub calls: Vec<CallInfo>,
    /// Variable assignments and reassignments.
    #[serde(skip)]
    #[schemars(skip)]
    pub assignments: Vec<AssignmentInfo>,
    /// Field access patterns.
    #[serde(skip)]
    #[schemars(skip)]
    pub field_accesses: Vec<FieldAccessInfo>,
}

/// Minimal function info for analyze_module: name and line only.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModuleFunctionInfo {
    /// Function name
    pub name: String,
    /// Line number where function is defined
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line: usize,
}

/// Minimal import info for analyze_module: module and items only.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModuleImportInfo {
    /// Full module path (e.g., 'std::collections' for 'use std::collections::HashMap')
    pub module: String,
    /// Imported symbols (e.g., ['HashMap'])
    pub items: Vec<String>,
}

/// Minimal fixed schema for analyze_module: lightweight code understanding.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModuleInfo {
    /// File name (basename only, e.g., 'lib.rs')
    pub name: String,
    /// Total line count in file
    #[schemars(schema_with = "crate::schema_helpers::integer_schema")]
    pub line_count: usize,
    /// Programming language (e.g., 'rust', 'python', 'go')
    pub language: String,
    /// Function definitions (name and line only)
    pub functions: Vec<ModuleFunctionInfo>,
    /// Import statements (module and items only)
    pub imports: Vec<ModuleImportInfo>,
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

    #[test]
    fn schema_flatten_inline() {
        use schemars::schema_for;

        // Test AnalyzeDirectoryParams: cursor, page_size, force, summary must be top-level
        let dir_schema = schema_for!(AnalyzeDirectoryParams);
        let dir_props = dir_schema
            .as_object()
            .and_then(|o| o.get("properties"))
            .and_then(|v| v.as_object())
            .expect("AnalyzeDirectoryParams must have properties");

        assert!(
            dir_props.contains_key("cursor"),
            "cursor must be top-level in AnalyzeDirectoryParams schema"
        );
        assert!(
            dir_props.contains_key("page_size"),
            "page_size must be top-level in AnalyzeDirectoryParams schema"
        );
        assert!(
            dir_props.contains_key("force"),
            "force must be top-level in AnalyzeDirectoryParams schema"
        );
        assert!(
            dir_props.contains_key("summary"),
            "summary must be top-level in AnalyzeDirectoryParams schema"
        );

        // Test AnalyzeFileParams
        let file_schema = schema_for!(AnalyzeFileParams);
        let file_props = file_schema
            .as_object()
            .and_then(|o| o.get("properties"))
            .and_then(|v| v.as_object())
            .expect("AnalyzeFileParams must have properties");

        assert!(
            file_props.contains_key("cursor"),
            "cursor must be top-level in AnalyzeFileParams schema"
        );
        assert!(
            file_props.contains_key("page_size"),
            "page_size must be top-level in AnalyzeFileParams schema"
        );
        assert!(
            file_props.contains_key("force"),
            "force must be top-level in AnalyzeFileParams schema"
        );
        assert!(
            file_props.contains_key("summary"),
            "summary must be top-level in AnalyzeFileParams schema"
        );

        // Test AnalyzeSymbolParams
        let symbol_schema = schema_for!(AnalyzeSymbolParams);
        let symbol_props = symbol_schema
            .as_object()
            .and_then(|o| o.get("properties"))
            .and_then(|v| v.as_object())
            .expect("AnalyzeSymbolParams must have properties");

        assert!(
            symbol_props.contains_key("cursor"),
            "cursor must be top-level in AnalyzeSymbolParams schema"
        );
        assert!(
            symbol_props.contains_key("page_size"),
            "page_size must be top-level in AnalyzeSymbolParams schema"
        );
        assert!(
            symbol_props.contains_key("force"),
            "force must be top-level in AnalyzeSymbolParams schema"
        );
        assert!(
            symbol_props.contains_key("summary"),
            "summary must be top-level in AnalyzeSymbolParams schema"
        );
    }
}

/// Structured error metadata for MCP error responses.
/// Serializes to camelCase JSON for inclusion in `ErrorData.data`.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMeta {
    pub error_category: &'static str,
    pub is_retryable: bool,
    pub suggested_action: &'static str,
}

#[cfg(test)]
mod error_meta_tests {
    use super::*;

    #[test]
    fn test_error_meta_serialization_camel_case() {
        let meta = ErrorMeta {
            error_category: "validation",
            is_retryable: false,
            suggested_action: "fix input",
        };
        let v = serde_json::to_value(&meta).unwrap();
        assert_eq!(v["errorCategory"], "validation");
        assert_eq!(v["isRetryable"], false);
        assert_eq!(v["suggestedAction"], "fix input");
    }

    #[test]
    fn test_error_meta_validation_not_retryable() {
        let meta = ErrorMeta {
            error_category: "validation",
            is_retryable: false,
            suggested_action: "use summary=true",
        };
        assert!(!meta.is_retryable);
    }

    #[test]
    fn test_error_meta_transient_retryable() {
        let meta = ErrorMeta {
            error_category: "transient",
            is_retryable: true,
            suggested_action: "retry the request",
        };
        assert!(meta.is_retryable);
    }
}
