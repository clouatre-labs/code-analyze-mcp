use tree_sitter::Node;

/// Tree-sitter query for extracting Rust elements (functions and structs/enums/traits).
pub const ELEMENT_QUERY: &str = r#"
(function_item
  name: (identifier) @func_name
  parameters: (parameters) @params) @function
(struct_item) @class
(enum_item) @class
(trait_item) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call_expression function: (identifier) @call)
(call_expression function: (field_expression field: (field_identifier) @call))
(call_expression function: (scoped_identifier name: (identifier) @call))
"#;

/// Tree-sitter query for extracting type references.
pub const REFERENCE_QUERY: &str = r#"
(type_identifier) @type_ref
"#;

/// Tree-sitter query for extracting imports.
pub const IMPORT_QUERY: &str = r#"
(use_declaration argument: (_) @import_path) @import
"#;

/// Tree-sitter query for extracting impl blocks and methods.
pub const IMPL_QUERY: &str = r#"
(impl_item
  type: (type_identifier) @impl_type
  body: (declaration_list
    (function_item
      name: (identifier) @method_name
      parameters: (parameters) @method_params) @method))
"#;

/// Extract function name from a function node.
pub fn extract_function_name(node: &Node, source: &str, _query_name: &str) -> Option<String> {
    if node.kind() != "function_item" {
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

/// Find method name for a receiver type.
pub fn find_method_for_receiver(
    node: &Node,
    _source: &str,
    _depth: Option<usize>,
) -> Option<String> {
    if node.kind() != "method_item" && node.kind() != "function_item" {
        return None;
    }
    node.child_by_field_name("name").and_then(|n| {
        let text = n.utf8_text(&[]).ok()?;
        Some(text.to_string())
    })
}

/// Find receiver type for a method.
pub fn find_receiver_type(node: &Node, source: &str) -> Option<String> {
    if node.kind() != "impl_item" {
        return None;
    }
    node.child_by_field_name("type").and_then(|n| {
        let start = n.start_byte();
        let end = n.end_byte();
        if end <= source.len() {
            Some(source[start..end].to_string())
        } else {
            None
        }
    })
}
