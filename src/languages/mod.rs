pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod swift;
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
        "javascript" => Some(LanguageInfo {
            name: "javascript",
            language: tree_sitter_javascript::LANGUAGE.into(),
            element_query: javascript::ELEMENT_QUERY,
            call_query: javascript::CALL_QUERY,
            reference_query: None,
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
        }),
        "jsx" => Some(LanguageInfo {
            name: "jsx",
            language: tree_sitter_javascript::LANGUAGE.into(),
            element_query: javascript::ELEMENT_QUERY,
            call_query: javascript::CALL_QUERY,
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
        "kotlin" => Some(LanguageInfo {
            name: "kotlin",
            language: tree_sitter_kotlin_ng::LANGUAGE.into(),
            element_query: kotlin::ELEMENT_QUERY,
            call_query: kotlin::CALL_QUERY,
            reference_query: None,
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
        }),
        "swift" => Some(LanguageInfo {
            name: "swift",
            language: tree_sitter_swift::LANGUAGE.into(),
            element_query: swift::ELEMENT_QUERY,
            call_query: swift::CALL_QUERY,
            reference_query: None,
            import_query: None,
            impl_query: None,
            extract_function_name: Some(swift::extract_function_name),
            find_method_for_receiver: None,
            find_receiver_type: None,
        }),
        "ruby" => Some(LanguageInfo {
            name: "ruby",
            language: tree_sitter_ruby::LANGUAGE.into(),
            element_query: ruby::ELEMENT_QUERY,
            call_query: ruby::CALL_QUERY,
            reference_query: Some(ruby::REFERENCE_QUERY),
            import_query: None,
            impl_query: None,
            extract_function_name: None,
            find_method_for_receiver: Some(ruby::find_method_for_receiver),
            find_receiver_type: None,
        }),
        _ => None,
    }
}
