use tree_sitter::Node;

/// Tree-sitter query for extracting Swift elements (functions and types).
pub const ELEMENT_QUERY: &str = r#"
(function_declaration) @function
(class_declaration) @class
(protocol_declaration) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call_expression
  (identifier) @call)
"#;

/// Extract function name from a function_declaration node.
pub fn extract_function_name(node: &Node, source: &str, _query_name: &str) -> Option<String> {
    if node.kind() != "function_declaration" {
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
