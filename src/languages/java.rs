/// Tree-sitter query for extracting Java elements (methods and classes).
pub const ELEMENT_QUERY: &str = r#"
(method_declaration
  name: (identifier) @method_name) @function
(class_declaration
  name: (identifier) @class_name) @class
(interface_declaration
  name: (identifier) @interface_name) @class
(enum_declaration
  name: (identifier) @enum_name) @class
"#;

/// Tree-sitter query for extracting function calls.
pub const CALL_QUERY: &str = r#"
(method_invocation
  name: (identifier) @call)
"#;
