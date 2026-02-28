/// Tree-sitter query for extracting Rust elements (functions and structs/enums/traits).
pub const ELEMENT_QUERY: &str = r#"
(function_item name: (identifier) @function)
(struct_item) @class
(enum_item) @class
(trait_item) @class
"#;
