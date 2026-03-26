/// Tree-sitter query for extracting Python elements (functions and classes).
pub const ELEMENT_QUERY: &str = r"
(function_definition
  name: (identifier) @func_name) @function
(class_definition
  name: (identifier) @class_name) @class
";

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r"
(call
  function: (identifier) @call)
(call
  function: (attribute attribute: (identifier) @call))
";

/// Tree-sitter query for extracting type references.
/// Python grammar has no `type_identifier` node; use `(type (identifier) @type_ref)`
/// to capture type names in annotations and `generic_type` for parameterized types.
pub const REFERENCE_QUERY: &str = r"
(type (identifier) @type_ref)
(generic_type (identifier) @type_ref)
";

/// Tree-sitter query for extracting Python imports.
pub const IMPORT_QUERY: &str = r"
(import_statement) @import_path
(import_from_statement) @import_path
";

use tree_sitter::Node;

/// Extract inheritance information from a Python class node.
#[must_use]
pub fn extract_inheritance(node: &Node, source: &str) -> Vec<String> {
    let mut inherits = Vec::new();

    // Get superclasses field from class_definition
    if let Some(superclasses) = node.child_by_field_name("superclasses") {
        // superclasses contains an argument_list
        for i in 0..superclasses.named_child_count() {
            if let Some(child) = superclasses.named_child(u32::try_from(i).unwrap_or(u32::MAX))
                && matches!(child.kind(), "identifier" | "attribute")
            {
                let text = &source[child.start_byte()..child.end_byte()];
                inherits.push(text.to_string());
            }
        }
    }

    inherits
}
