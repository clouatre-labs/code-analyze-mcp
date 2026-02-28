/// Tree-sitter query for extracting Rust elements (functions and structs/impls).
pub const ELEMENT_QUERY: &str = r#"
(function_item name: (identifier) @function)
(impl_item) @class
"#;
