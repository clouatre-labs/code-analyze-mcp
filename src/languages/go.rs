use tree_sitter::Node;

/// Tree-sitter query for extracting Go elements (functions, methods, and types).
pub const ELEMENT_QUERY: &str = r#"
(function_declaration
  name: (identifier) @func_name) @function
(method_declaration
  name: (field_identifier) @method_name) @function
(type_spec
  name: (type_identifier) @type_name
  type: (struct_type)) @class
(type_spec
  name: (type_identifier) @type_name
  type: (interface_type)) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)
(call_expression
  function: (selector_expression field: (field_identifier) @call))
"#;

/// Tree-sitter query for extracting type references.
pub const REFERENCE_QUERY: &str = r#"
(type_identifier) @type_ref
"#;

/// Tree-sitter query for extracting Go imports.
pub const IMPORT_QUERY: &str = r#"
(import_declaration) @import_path
"#;

/// Find method name for a receiver type.
pub fn find_method_for_receiver(
    node: &Node,
    source: &str,
    _depth: Option<usize>,
) -> Option<String> {
    if node.kind() != "method_declaration" && node.kind() != "function_declaration" {
        return None;
    }
    node.child_by_field_name("name").and_then(|n| {
        let start = n.start_byte();
        let end = n.end_byte();
        if end <= source.len() {
            Some(source[start..end].to_string())
        } else {
            None
        }
    })
}
