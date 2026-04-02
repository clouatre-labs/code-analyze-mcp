// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
use tree_sitter::Node;

/// Tree-sitter query for extracting C/C++ elements (functions, classes, and structures).
pub const ELEMENT_QUERY: &str = r"
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @func_name)) @function
(function_definition
  declarator: (function_declarator
    declarator: (qualified_identifier
      name: (identifier) @method_name))) @function
(class_specifier
  name: (type_identifier) @type_name) @class
(struct_specifier
  name: (type_identifier) @type_name) @class
(template_declaration
  (function_definition
    declarator: (function_declarator
      declarator: (identifier) @template_func))) @function
";

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r"
(call_expression
  function: (identifier) @call)
(call_expression
  function: (field_expression field: (field_identifier) @call))
";

/// Tree-sitter query for extracting type references.
pub const REFERENCE_QUERY: &str = r"
(type_identifier) @type_ref
";

/// Tree-sitter query for extracting C/C++ preprocessor directives (#include).
pub const IMPORT_QUERY: &str = r"
(preproc_include
  path: (string_literal) @include)
(preproc_include
  path: (system_lib_string) @include)
";

/// Find method name for a receiver type (class/struct context).
#[must_use]
pub fn find_method_for_receiver(
    node: &Node,
    source: &str,
    _depth: Option<usize>,
) -> Option<String> {
    if node.kind() != "function_definition" {
        return None;
    }

    // Walk up to find if we're in a class_specifier or struct_specifier
    let mut parent = node.parent();
    let mut in_class = false;
    while let Some(p) = parent {
        if p.kind() == "class_specifier" || p.kind() == "struct_specifier" {
            in_class = true;
            break;
        }
        parent = p.parent();
    }

    if !in_class {
        return None;
    }

    // Extract the method name from function_declarator
    if let Some(decl) = node.child_by_field_name("declarator") {
        extract_declarator_name(decl, source)
    } else {
        None
    }
}

/// Extract inheritance information from a class_specifier or struct_specifier node.
#[must_use]
pub fn extract_inheritance(node: &Node, source: &str) -> Vec<String> {
    let mut inherits = Vec::new();

    if node.kind() != "class_specifier" && node.kind() != "struct_specifier" {
        return inherits;
    }

    // Look for base_class_clause child
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(u32::try_from(i).unwrap_or(u32::MAX))
            && child.kind() == "base_class_clause"
        {
            // Walk base_class_clause for type_identifier nodes
            for j in 0..child.named_child_count() {
                if let Some(base) = child.named_child(u32::try_from(j).unwrap_or(u32::MAX))
                    && base.kind() == "type_identifier"
                {
                    let text = &source[base.start_byte()..base.end_byte()];
                    inherits.push(text.to_string());
                }
            }
        }
    }

    inherits
}

/// Helper: extract name from a declarator node (handles identifiers and qualified identifiers).
fn extract_declarator_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => {
            let start = node.start_byte();
            let end = node.end_byte();
            if end <= source.len() {
                Some(source[start..end].to_string())
            } else {
                None
            }
        }
        "qualified_identifier" => node.child_by_field_name("name").and_then(|n| {
            let start = n.start_byte();
            let end = n.end_byte();
            if end <= source.len() {
                Some(source[start..end].to_string())
            } else {
                None
            }
        }),
        "function_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|n| extract_declarator_name(n, source)),
        "pointer_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|n| extract_declarator_name(n, source)),
        "reference_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|n| extract_declarator_name(n, source)),
        _ => None,
    }
}

#[cfg(all(test, feature = "lang-cpp"))]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_cpp(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .expect("failed to set C++ language");
        parser.parse(source, None).expect("failed to parse source")
    }

    #[test]
    fn test_free_function() {
        // Arrange: free function definition
        let source = "int add(int a, int b) { return a + b; }";
        let tree = parse_cpp(source);
        let root = tree.root_node();
        let func_node = root.named_child(0).expect("expected function_definition");
        // Act
        let result = find_method_for_receiver(&func_node, source, None);
        // Assert: free function should not be a method
        assert_eq!(result, None);
    }

    #[test]
    fn test_class_with_method() {
        // Arrange: class with method
        let source = "class Foo { public: int getValue() { return 42; } };";
        let tree = parse_cpp(source);
        let root = tree.root_node();
        // Find the function_definition inside the class
        let func_node = find_node_by_kind(root, "function_definition").expect("expected function");
        // Act
        let result = find_method_for_receiver(&func_node, source, None);
        // Assert: method inside class should be recognized
        assert_eq!(result, Some("getValue".to_string()));
    }

    #[test]
    fn test_struct() {
        // Arrange: struct with simple field
        let source = "struct Point { int x; int y; };";
        let tree = parse_cpp(source);
        let root = tree.root_node();
        let struct_node = root
            .named_child(0)
            .expect("expected struct_specifier or declaration");
        // Act
        let result = extract_inheritance(&struct_node, source);
        // Assert: struct with no inheritance should return empty
        assert!(result.is_empty());
    }

    #[test]
    fn test_include_directive() {
        // Arrange: include directive
        let source = "#include <stdio.h>\nint main() { return 0; }";
        let tree = parse_cpp(source);
        let root = tree.root_node();
        // Find preproc_include node
        let include_node = find_node_by_kind(root, "preproc_include").expect("expected include");
        // Act: preproc_include node should exist
        // Assert
        assert_eq!(include_node.kind(), "preproc_include");
    }

    #[test]
    fn test_template_function() {
        // Arrange: template function
        let source = "template<typename T> T max(T a, T b) { return a > b ? a : b; }";
        let tree = parse_cpp(source);
        let root = tree.root_node();
        let template_node = root.named_child(0).expect("expected template_declaration");
        // Act
        // Assert: template_declaration should exist
        assert_eq!(template_node.kind(), "template_declaration");
    }

    #[test]
    fn test_class_with_inheritance() {
        // Arrange: class with base class
        let source = "class Derived : public Base { };";
        let tree = parse_cpp(source);
        let root = tree.root_node();
        let class_node = find_node_by_kind(root, "class_specifier").expect("expected class");
        // Act
        let result = extract_inheritance(&class_node, source);
        // Assert: should have "Base" as inheritance
        assert!(!result.is_empty(), "expected inheritance information");
        assert!(
            result.iter().any(|s| s.contains("Base")),
            "expected 'Base' in inheritance: {:?}",
            result
        );
    }

    /// Helper to find the first node of a given kind
    fn find_node_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(u32::try_from(i).unwrap_or(u32::MAX)) {
                if let Some(found) = find_node_by_kind(child, kind) {
                    return Some(found);
                }
            }
        }
        None
    }
}
