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

/// Extract inheritance information from a Go type node.
pub fn extract_inheritance(node: &Node, source: &str) -> Vec<String> {
    let mut inherits = Vec::new();

    // Get the type field from type_spec
    if let Some(type_field) = node.child_by_field_name("type") {
        match type_field.kind() {
            "struct_type" => {
                // For struct embedding, walk children for field_declaration_list
                for i in 0..type_field.named_child_count() {
                    if let Some(field_list) = type_field.named_child(i as u32)
                        && field_list.kind() == "field_declaration_list"
                    {
                        // Walk field_declaration_list for field_declaration without name
                        for j in 0..field_list.named_child_count() {
                            if let Some(field) = field_list.named_child(j as u32)
                                && field.kind() == "field_declaration"
                                && field.child_by_field_name("name").is_none()
                            {
                                // Embedded type has no name field
                                if let Some(type_node) = field.child_by_field_name("type") {
                                    let text =
                                        &source[type_node.start_byte()..type_node.end_byte()];
                                    inherits.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
            "interface_type" => {
                // For interface embedding, walk children for type_elem
                for i in 0..type_field.named_child_count() {
                    if let Some(elem) = type_field.named_child(i as u32)
                        && elem.kind() == "type_elem"
                    {
                        let text = &source[elem.start_byte()..elem.end_byte()];
                        inherits.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    inherits
}
