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

/// Tree-sitter query for extracting type references.
/// Python grammar has no type_identifier node; use (type (identifier) @type_ref)
/// to capture type names in annotations and generic_type for parameterized types.
pub const REFERENCE_QUERY: &str = r#"
(type (identifier) @type_ref)
(generic_type (identifier) @type_ref)
"#;
