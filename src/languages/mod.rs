pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod typescript;

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
    pub import_query: Option<&'static str>,
    pub impl_query: Option<&'static str>,
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
            import_query: Some(rust::IMPORT_QUERY),
            impl_query: Some(rust::IMPL_QUERY),
            extract_function_name: Some(rust::extract_function_name),
            find_method_for_receiver: Some(rust::find_method_for_receiver),
            find_receiver_type: Some(rust::find_receiver_type),
        }),
        "python" => Some(LanguageInfo {
            name: "python",
            language: tree_sitter_python::LANGUAGE.into(),
            element_query: python::ELEMENT_QUERY,
            call_query: python::CALL_QUERY,
            reference_query: None,
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
        }),
        "typescript" => Some(LanguageInfo {
            name: "typescript",
            language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            element_query: typescript::ELEMENT_QUERY,
            call_query: typescript::CALL_QUERY,
            reference_query: None,
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
        }),
        "tsx" => Some(LanguageInfo {
            name: "tsx",
            language: tree_sitter_typescript::LANGUAGE_TSX.into(),
            element_query: typescript::ELEMENT_QUERY,
            call_query: typescript::CALL_QUERY,
            reference_query: None,
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
        }),
        "go" => Some(LanguageInfo {
            name: "go",
            language: tree_sitter_go::LANGUAGE.into(),
            element_query: go::ELEMENT_QUERY,
            call_query: go::CALL_QUERY,
            reference_query: Some(go::REFERENCE_QUERY),
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: Some(go::find_method_for_receiver),
            find_receiver_type: None,
        }),
        "java" => Some(LanguageInfo {
            name: "java",
            language: tree_sitter_java::LANGUAGE.into(),
            element_query: java::ELEMENT_QUERY,
            call_query: java::CALL_QUERY,
            reference_query: None,
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
        }),
        _ => None,
    }
}
