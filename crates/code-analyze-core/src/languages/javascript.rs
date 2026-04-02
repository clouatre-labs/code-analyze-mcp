// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
/// Tree-sitter query for extracting JavaScript elements (functions, classes, and related).
#[cfg(feature = "lang-javascript")]
pub const ELEMENT_QUERY: &str = r"
(function_declaration) @function
(class_declaration) @class
(method_definition) @function
(generator_function_declaration) @function

";

/// Tree-sitter query for extracting function calls.
#[cfg(feature = "lang-javascript")]
pub const CALL_QUERY: &str = r"
(call_expression
  function: (identifier) @call)
(call_expression
  function: (member_expression property: (property_identifier) @call))
";

/// Tree-sitter query for extracting JavaScript imports (ESM and CommonJS).
#[cfg(feature = "lang-javascript")]
pub const IMPORT_QUERY: &str = r#"
(import_statement) @import_path
(call_expression
  function: (identifier) @_fn (#eq? @_fn "require")
  arguments: (arguments (string) @import_path))
"#;

use tree_sitter::Node;

/// Extract inheritance information from a JavaScript class node.
#[cfg(feature = "lang-javascript")]
#[must_use]
pub fn extract_inheritance(node: &Node, source: &str) -> Vec<String> {
    let mut inherits = Vec::new();
    // Walk children to find class_heritage node
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(u32::try_from(i).unwrap_or(u32::MAX))
            && child.kind() == "class_heritage"
        {
            // Walk class_heritage children for extends_clause
            for j in 0..child.named_child_count() {
                if let Some(clause) = child.named_child(u32::try_from(j).unwrap_or(u32::MAX))
                    && clause.kind() == "extends_clause"
                    && let Some(value) = clause.child_by_field_name("value")
                {
                    let text = &source[value.start_byte()..value.end_byte()];
                    inherits.push(format!("extends {text}"));
                }
            }
        }
    }
    inherits
}

#[cfg(test)]
#[cfg(feature = "lang-javascript")]
mod tests {
    use tree_sitter::Parser;

    fn parse_js(src: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .unwrap();
        parser.parse(src, None).unwrap()
    }

    fn find_node_by_kind<'a>(
        node: tree_sitter::Node<'a>,
        kind: &str,
    ) -> Option<tree_sitter::Node<'a>> {
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

    #[test]
    fn test_function_declaration() {
        let src = "function greet() { return 42; }";
        let tree = parse_js(src);
        let root = tree.root_node();
        let func = find_node_by_kind(root, "function_declaration");
        assert!(func.is_some(), "expected to find function_declaration");
    }

    #[test]
    fn test_arrow_function() {
        let src = "const add = (a, b) => a + b;";
        let tree = parse_js(src);
        let root = tree.root_node();
        let arrow = find_node_by_kind(root, "arrow_function");
        assert!(arrow.is_some(), "expected to find arrow_function");
    }

    #[test]
    fn test_class_declaration() {
        let src = "class Foo extends Bar { method() {} }";
        let tree = parse_js(src);
        let root = tree.root_node();
        let class = find_node_by_kind(root, "class_declaration");
        assert!(class.is_some(), "expected to find class_declaration");
    }

    #[test]
    fn test_es_import() {
        let src = "import {x} from 'module';";
        let tree = parse_js(src);
        let root = tree.root_node();
        let import = find_node_by_kind(root, "import_statement");
        assert!(import.is_some(), "expected to find import_statement");
    }

    #[test]
    fn test_commonjs_require() {
        let src = "const lib = require('lib');";
        let tree = parse_js(src);
        let root = tree.root_node();
        let call = find_node_by_kind(root, "call_expression");
        assert!(call.is_some(), "expected to find call_expression");
    }
}
