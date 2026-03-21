/// Tree-sitter query for extracting Fortran elements (functions and modules).
pub const ELEMENT_QUERY: &str = r#"
(subroutine
  (subroutine_statement) @function)

(function
  (function_statement) @function)

(module
  (module_statement) @class)
"#;

/// Tree-sitter query for extracting Fortran function calls.
pub const CALL_QUERY: &str = r#"
(subroutine_call
  (identifier) @call)

(call_expression
  (identifier) @call)
"#;

/// Tree-sitter query for extracting Fortran type references.
pub const REFERENCE_QUERY: &str = r#"
(name) @type_ref
"#;

/// Tree-sitter query for extracting Fortran imports (USE statements).
pub const IMPORT_QUERY: &str = r#"
(use_statement
  (module_name) @import_path)
"#;

use tree_sitter::Node;

/// Extract inheritance information from a Fortran node.
/// Fortran does not have classical inheritance; return empty.
pub fn extract_inheritance(_node: &Node, _source: &str) -> Vec<String> {
    Vec::new()
}
