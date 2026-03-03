/// Tree-sitter query for extracting JavaScript elements (functions and classes).
pub const ELEMENT_QUERY: &str = r#"
(function_declaration
  name: (identifier) @func_name) @function
(class_declaration
  name: (identifier) @class_name) @class
(method_definition
  name: (property_identifier) @method_name) @function
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)
(call_expression
  function: (member_expression property: (property_identifier) @call))
"#;
