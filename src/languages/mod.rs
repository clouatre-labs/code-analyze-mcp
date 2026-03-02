pub mod rust;

use tree_sitter::{Language, Node};

/// Handler to extract function name from a node.
pub type ExtractFunctionNameHandler = fn(&Node, &str, &str) -> Option<String>;

/// Handler to find method name for a receiver type.
pub type FindMethodForReceiverHandler = fn(&Node, &str, Option<usize>) -> Option<String>;

/// Handler to find receiver type for a method.
pub type FindReceiverTypeHandler = fn(&Node, &str) -> Option<String>;

/// Information about a supported language for code analysis.
pub struct LanguageInfo {
    pub name: &'static str,
    pub language: Language,
    pub element_query: &'static str,
    pub call_query: &'static str,
    pub reference_query: Option<&'static str>,
    pub extract_function_name: Option<ExtractFunctionNameHandler>,
    pub find_method_for_receiver: Option<FindMethodForReceiverHandler>,
    pub find_receiver_type: Option<FindReceiverTypeHandler>,
}

/// Get language information by language name.
pub fn get_language_info(lang_name: &str) -> Option<LanguageInfo> {
    match lang_name {
        "rust" => Some(LanguageInfo {
            name: "rust",
            language: tree_sitter_rust::LANGUAGE.into(),
            element_query: rust::ELEMENT_QUERY,
            call_query: rust::CALL_QUERY,
            reference_query: Some(rust::REFERENCE_QUERY),
            extract_function_name: Some(rust::extract_function_name),
            find_method_for_receiver: Some(rust::find_method_for_receiver),
            find_receiver_type: Some(rust::find_receiver_type),
        }),
        _ => None,
    }
}
