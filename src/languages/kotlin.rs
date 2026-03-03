/// Tree-sitter query for extracting Kotlin elements (functions and classes).
pub const ELEMENT_QUERY: &str = r#"
(function_declaration) @function
(class_declaration) @class
(object_declaration) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call_expression
  (identifier) @call)
"#;
