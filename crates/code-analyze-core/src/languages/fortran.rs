// SPDX-FileCopyrightText: 2026 code-analyze-mcp contributors
// SPDX-License-Identifier: Apache-2.0
/// Tree-sitter query for extracting Fortran elements (functions and subroutines).
///
/// Module constructs are omitted: `module_statement` has no `name` field
/// in the current grammar, so `@class` captures would be counted in
/// `analyze_directory` but produce no names in `analyze_file`. Modules will
/// be added here once the grammar exposes a `name` field.
pub const ELEMENT_QUERY: &str = r"
(subroutine
  (subroutine_statement) @function)

(function
  (function_statement) @function)
";

/// Tree-sitter query for extracting Fortran function calls.
pub const CALL_QUERY: &str = r"
(subroutine_call
  (identifier) @call)

(call_expression
  (identifier) @call)
";

/// Tree-sitter query for extracting Fortran type references.
pub const REFERENCE_QUERY: &str = r"
(name) @type_ref
";

/// Tree-sitter query for extracting Fortran imports (USE statements).
pub const IMPORT_QUERY: &str = r"
(use_statement
  (module_name) @import_path)
";

use tree_sitter::Node;

/// Extract inheritance information from a Fortran node.
/// Fortran does not have classical inheritance; return empty.
#[must_use]
pub fn extract_inheritance(_node: &Node, _source: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(all(test, feature = "lang-fortran"))]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_fortran(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_fortran::LANGUAGE.into())
            .expect("failed to set Fortran language");
        let source_bytes = source.as_bytes().to_vec();
        let tree = parser.parse(&source_bytes, None).expect("failed to parse");
        (tree, source_bytes)
    }

    #[test]
    fn test_extract_inheritance_returns_empty() {
        // Arrange
        let source = "PROGRAM test\nEND PROGRAM test\n";
        let (tree, _source_bytes) = parse_fortran(source);
        let root = tree.root_node();

        // Act
        let result = extract_inheritance(&root, source);

        // Assert
        assert!(result.is_empty());
    }
}
