/// Tree-sitter query for extracting TypeScript elements (functions, classes, and TS-specific types).
pub const ELEMENT_QUERY: &str = r#"
(function_declaration) @function
(class_declaration) @class
(method_definition) @function
(interface_declaration) @class
(type_alias_declaration) @class
(enum_declaration) @class
(abstract_class_declaration) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)
(call_expression
  function: (member_expression property: (property_identifier) @call))
"#;

/// Tree-sitter query for extracting TypeScript imports.
pub const IMPORT_QUERY: &str = r#"
(import_statement) @import_path
"#;
