use super::LanguageInfo;

pub const ELEMENT_QUERY: &str = "
(function_item name: (identifier) @function)
(struct_item name: (type_identifier) @class)
(enum_item name: (type_identifier) @class)
(trait_item name: (type_identifier) @class)
";

pub static LANGUAGE_INFO: LanguageInfo = LanguageInfo {
    name: "rust",
    element_query: ELEMENT_QUERY,
    tree_sitter_language: tree_sitter_rust::language,
};

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::{Parser, Query};

    #[test]
    fn test_element_query_is_valid() {
        let lang = (LANGUAGE_INFO.tree_sitter_language)();
        let result = Query::new(&lang, ELEMENT_QUERY);
        assert!(result.is_ok(), "ELEMENT_QUERY must be a valid tree-sitter query");
    }

    #[test]
    fn test_language_parses_rust_source() {
        let lang = (LANGUAGE_INFO.tree_sitter_language)();
        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let source = "fn main() { println!(\"hello\"); }";
        let tree = parser.parse(source, None);
        assert!(tree.is_some());
    }
}
