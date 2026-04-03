// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
/// Tree-sitter query for extracting TypeScript elements (functions, classes, and TS-specific types).
///
/// Named sub-captures (`@func_name`, `@class_name`, etc.) align this query with the style used by
/// Java, Python, and Go handlers. The parser resolves names via `child_by_field_name("name")` on
/// the outer `@function` / `@class` capture; the sub-captures are additional metadata only.
/// `method_definition` is kept without a sub-capture because its `name` field accepts multiple
/// node types (`property_identifier`, `computed_property_name`, `private_property_identifier`,
/// etc.) and narrowing to one would silently drop computed or private method names.
pub const ELEMENT_QUERY: &str = r"
(function_declaration
  name: (identifier) @func_name) @function
(class_declaration
  name: (type_identifier) @class_name) @class
(method_definition) @function
(interface_declaration
  name: (type_identifier) @interface_name) @class
(type_alias_declaration
  name: (type_identifier) @type_name) @class
(enum_declaration
  name: (identifier) @enum_name) @class
(abstract_class_declaration
  name: (type_identifier) @abstract_class_name) @class
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

#[cfg(all(test, feature = "lang-typescript"))]
mod tests {
    use super::*;
    use tree_sitter::{Parser, StreamingIterator};

    fn parse_ts(src: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .expect("Error loading TypeScript language");
        parser.parse(src, None).expect("Failed to parse TypeScript")
    }

    fn parse_tsx(src: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .expect("Error loading TSX language");
        parser.parse(src, None).expect("Failed to parse TSX")
    }

    #[test]
    fn test_typescript_element_query_function_and_class() {
        // Arrange
        let src = "function greet(): void {}\nclass Greeter {}\n";
        let tree = parse_ts(src);
        let root = tree.root_node();

        // Act
        let query = tree_sitter::Query::new(
            &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ELEMENT_QUERY,
        )
        .expect("ELEMENT_QUERY must be valid");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, root, src.as_bytes());

        let mut captured_classes: Vec<String> = Vec::new();
        let mut captured_functions: Vec<String> = Vec::new();
        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let name = query.capture_names()[capture.index as usize];
                let node = capture.node;
                match name {
                    "class" => {
                        if let Some(n) = node.child_by_field_name("name") {
                            captured_classes.push(src[n.start_byte()..n.end_byte()].to_string());
                        }
                    }
                    "function" => {
                        if let Some(n) = node.child_by_field_name("name") {
                            captured_functions.push(src[n.start_byte()..n.end_byte()].to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        // Assert
        assert!(
            captured_classes.contains(&"Greeter".to_string()),
            "expected Greeter class, got {:?}",
            captured_classes
        );
        assert!(
            captured_functions.contains(&"greet".to_string()),
            "expected greet function, got {:?}",
            captured_functions
        );
    }

    #[test]
    fn test_typescript_element_query_interface_and_type_alias() {
        // Arrange -- edge case: interface and type alias names must not be empty
        let src = "interface Runnable { run(): void; }\ntype Runner = Runnable;\n";
        let tree = parse_ts(src);
        let root = tree.root_node();

        // Act
        let query = tree_sitter::Query::new(
            &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ELEMENT_QUERY,
        )
        .expect("ELEMENT_QUERY must be valid");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, root, src.as_bytes());

        let mut captured_names: Vec<String> = Vec::new();
        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let cap_name = query.capture_names()[capture.index as usize];
                if cap_name == "class" {
                    let node = capture.node;
                    if let Some(n) = node.child_by_field_name("name") {
                        let text = src[n.start_byte()..n.end_byte()].to_string();
                        if !text.is_empty() {
                            captured_names.push(text);
                        }
                    }
                }
            }
        }

        // Assert -- the named-capture fix ensures child_by_field_name("name") resolves
        assert!(
            captured_names.contains(&"Runnable".to_string()),
            "expected Runnable interface, got {:?}",
            captured_names
        );
        assert!(
            captured_names.contains(&"Runner".to_string()),
            "expected Runner type alias, got {:?}",
            captured_names
        );
    }

    #[test]
    fn test_tsx_element_query_with_jsx() {
        // Arrange -- a TSX functional component parsed with the TSX grammar
        let src = "function Button(): JSX.Element { return <button />; }\n";
        let tree = parse_tsx(src);
        let root = tree.root_node();

        // Act
        let query =
            tree_sitter::Query::new(&tree_sitter_typescript::LANGUAGE_TSX.into(), ELEMENT_QUERY)
                .expect("ELEMENT_QUERY must be valid for TSX");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, root, src.as_bytes());

        let mut captured_functions: Vec<String> = Vec::new();
        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let cap_name = query.capture_names()[capture.index as usize];
                if cap_name == "function" {
                    let node = capture.node;
                    if let Some(n) = node.child_by_field_name("name") {
                        captured_functions.push(src[n.start_byte()..n.end_byte()].to_string());
                    }
                }
            }
        }

        // Assert
        assert!(
            captured_functions.contains(&"Button".to_string()),
            "expected Button component, got {:?}",
            captured_functions
        );
    }
}
