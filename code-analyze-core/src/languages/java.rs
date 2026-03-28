/// Tree-sitter query for extracting Java elements (methods and classes).
pub const ELEMENT_QUERY: &str = r"
(method_declaration
  name: (identifier) @method_name) @function
(class_declaration
  name: (identifier) @class_name) @class
(interface_declaration
  name: (identifier) @interface_name) @class
(enum_declaration
  name: (identifier) @enum_name) @class
";

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r"
(method_invocation
  name: (identifier) @call)
";

/// Tree-sitter query for extracting type references.
pub const REFERENCE_QUERY: &str = r"
(type_identifier) @type_ref
";

/// Tree-sitter query for extracting Java imports.
pub const IMPORT_QUERY: &str = r"
(import_declaration) @import_path
";

use tree_sitter::Node;

/// Extract inheritance information from a Java class node.
#[must_use]
pub fn extract_inheritance(node: &Node, source: &str) -> Vec<String> {
    let mut inherits = Vec::new();

    // Extract superclass (extends)
    if let Some(superclass) = node.child_by_field_name("superclass") {
        for i in 0..superclass.named_child_count() {
            if let Some(child) = superclass.named_child(u32::try_from(i).unwrap_or(u32::MAX))
                && child.kind() == "type_identifier"
            {
                let text = &source[child.start_byte()..child.end_byte()];
                inherits.push(format!("extends {text}"));
            }
        }
    }

    // Extract interfaces (implements)
    if let Some(interfaces) = node.child_by_field_name("interfaces") {
        for i in 0..interfaces.named_child_count() {
            if let Some(type_list) = interfaces.named_child(u32::try_from(i).unwrap_or(u32::MAX)) {
                for j in 0..type_list.named_child_count() {
                    if let Some(type_node) =
                        type_list.named_child(u32::try_from(j).unwrap_or(u32::MAX))
                        && type_node.kind() == "type_identifier"
                    {
                        let text = &source[type_node.start_byte()..type_node.end_byte()];
                        inherits.push(format!("implements {text}"));
                    }
                }
            }
        }
    }

    inherits
}
