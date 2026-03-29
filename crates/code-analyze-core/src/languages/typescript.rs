// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
/// Tree-sitter query for extracting TypeScript elements (functions, classes, and TS-specific types).
pub const ELEMENT_QUERY: &str = r"
(function_declaration) @function
(class_declaration) @class
(method_definition) @function
(interface_declaration) @class
(type_alias_declaration) @class
(enum_declaration) @class
(abstract_class_declaration) @class
";

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r"
(call_expression
  function: (identifier) @call)
(call_expression
  function: (member_expression property: (property_identifier) @call))
";

/// Tree-sitter query for extracting type references.
pub const REFERENCE_QUERY: &str = r"
(type_identifier) @type_ref
";

/// Tree-sitter query for extracting TypeScript imports.
pub const IMPORT_QUERY: &str = r"
(import_statement) @import_path
";

use tree_sitter::Node;

/// Extract inheritance information from a TypeScript class node.
#[must_use]
pub fn extract_inheritance(node: &Node, source: &str) -> Vec<String> {
    let mut inherits = Vec::new();

    // Walk children to find class_heritage node
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(u32::try_from(i).unwrap_or(u32::MAX))
            && child.kind() == "class_heritage"
        {
            // Walk class_heritage children for extends_clause and implements_clause
            for j in 0..child.named_child_count() {
                if let Some(clause) = child.named_child(u32::try_from(j).unwrap_or(u32::MAX)) {
                    if clause.kind() == "extends_clause" {
                        // Extract extends type
                        if let Some(value) = clause.child_by_field_name("value") {
                            let text = &source[value.start_byte()..value.end_byte()];
                            inherits.push(format!("extends {text}"));
                        }
                    } else if clause.kind() == "implements_clause" {
                        // Extract implements types
                        for k in 0..clause.named_child_count() {
                            if let Some(type_node) =
                                clause.named_child(u32::try_from(k).unwrap_or(u32::MAX))
                                && type_node.kind() == "type_identifier"
                            {
                                let text = &source[type_node.start_byte()..type_node.end_byte()];
                                inherits.push(format!("implements {text}"));
                            }
                        }
                    }
                }
            }
        }
    }

    inherits
}
