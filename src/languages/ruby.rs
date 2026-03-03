use tree_sitter::Node;

/// Tree-sitter query for extracting Ruby elements (methods and classes).
pub const ELEMENT_QUERY: &str = r#"
(method
  name: (identifier) @method_name) @function
(singleton_method
  name: (identifier) @singleton_name) @function
(class
  name: (constant) @class_name) @class
(module
  name: (constant) @module_name) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call
  method: (identifier) @call)
(call
  method: (constant) @call)
"#;

/// Tree-sitter query for extracting type references.
pub const REFERENCE_QUERY: &str = r#"
(constant) @type_ref
"#;

/// Find method name for a receiver type.
pub fn find_method_for_receiver(
    node: &Node,
    source: &str,
    _depth: Option<usize>,
) -> Option<String> {
    if node.kind() != "method" && node.kind() != "singleton_method" {
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
