/// Tree-sitter query for extracting Python elements (functions and classes).
pub const ELEMENT_QUERY: &str = r#"
(function_definition
  name: (identifier) @func_name) @function
(class_definition
  name: (identifier) @class_name) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call
  function: (identifier) @call)
(call
  function: (attribute attribute: (identifier) @call))
"#;
