//! Language-specific handlers and query definitions for tree-sitter parsing.
//!
//! Provides query strings and extraction handlers for supported languages:
//! Rust, Go, Java, Python, TypeScript, TSX, and Fortran.

pub mod fortran;
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

/// Handler to extract inheritance information from a class node.
pub type ExtractInheritanceHandler = fn(&Node, &str) -> Vec<String>;

/// Information about a supported language for code analysis.
pub struct LanguageInfo {
    pub name: &'static str,
    pub language: Language,
    pub element_query: &'static str,
    pub call_query: &'static str,
    pub reference_query: Option<&'static str>,
    pub import_query: Option<&'static str>,
    pub impl_query: Option<&'static str>,
    pub assignment_query: Option<&'static str>,
    pub field_query: Option<&'static str>,
    pub extract_function_name: Option<ExtractFunctionNameHandler>,
    pub find_method_for_receiver: Option<FindMethodForReceiverHandler>,
    pub find_receiver_type: Option<FindReceiverTypeHandler>,
    pub extract_inheritance: Option<ExtractInheritanceHandler>,
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
            assignment_query: Some(rust::ASSIGNMENT_QUERY),
            field_query: Some(rust::FIELD_QUERY),
            extract_function_name: Some(rust::extract_function_name),
            find_method_for_receiver: Some(rust::find_method_for_receiver),
            find_receiver_type: Some(rust::find_receiver_type),
            extract_inheritance: Some(rust::extract_inheritance),
        }),
        "python" => Some(LanguageInfo {
            name: "python",
            language: tree_sitter_python::LANGUAGE.into(),
            element_query: python::ELEMENT_QUERY,
            call_query: python::CALL_QUERY,
            reference_query: Some(python::REFERENCE_QUERY),
            import_query: Some(python::IMPORT_QUERY),
            impl_query: None,
            assignment_query: None,
            field_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
            extract_inheritance: Some(python::extract_inheritance),
        }),
        "typescript" => Some(LanguageInfo {
            name: "typescript",
            language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            element_query: typescript::ELEMENT_QUERY,
            call_query: typescript::CALL_QUERY,
            reference_query: Some(typescript::REFERENCE_QUERY),
            import_query: Some(typescript::IMPORT_QUERY),
            impl_query: None,
            assignment_query: None,
            field_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
            extract_inheritance: Some(typescript::extract_inheritance),
        }),
        "tsx" => Some(LanguageInfo {
            name: "tsx",
            language: tree_sitter_typescript::LANGUAGE_TSX.into(),
            element_query: typescript::ELEMENT_QUERY,
            call_query: typescript::CALL_QUERY,
            reference_query: Some(typescript::REFERENCE_QUERY),
            import_query: Some(typescript::IMPORT_QUERY),
            impl_query: None,
            assignment_query: None,
            field_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
            extract_inheritance: Some(typescript::extract_inheritance),
        }),
        "go" => Some(LanguageInfo {
            name: "go",
            language: tree_sitter_go::LANGUAGE.into(),
            element_query: go::ELEMENT_QUERY,
            call_query: go::CALL_QUERY,
            reference_query: Some(go::REFERENCE_QUERY),
            import_query: Some(go::IMPORT_QUERY),
            impl_query: None,
            assignment_query: None,
            field_query: None,
            extract_function_name: None,
            find_method_for_receiver: Some(go::find_method_for_receiver),
            find_receiver_type: None,
            extract_inheritance: Some(go::extract_inheritance),
        }),
        "java" => Some(LanguageInfo {
            name: "java",
            language: tree_sitter_java::LANGUAGE.into(),
            element_query: java::ELEMENT_QUERY,
            call_query: java::CALL_QUERY,
            reference_query: Some(java::REFERENCE_QUERY),
            import_query: Some(java::IMPORT_QUERY),
            impl_query: None,
            assignment_query: None,
            field_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
            extract_inheritance: Some(java::extract_inheritance),
        }),
        "fortran" => Some(LanguageInfo {
            name: "fortran",
            language: tree_sitter_fortran::LANGUAGE.into(),
            element_query: fortran::ELEMENT_QUERY,
            call_query: fortran::CALL_QUERY,
            reference_query: Some(fortran::REFERENCE_QUERY),
            import_query: Some(fortran::IMPORT_QUERY),
            impl_query: None,
            assignment_query: None,
            field_query: None,
            extract_function_name: None,
            find_method_for_receiver: None,
            find_receiver_type: None,
            extract_inheritance: Some(fortran::extract_inheritance),
        }),
        _ => None,
    }
}
