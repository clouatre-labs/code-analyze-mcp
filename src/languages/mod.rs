pub mod rust;

use tree_sitter::Language;

/// Information about a supported language for code analysis.
pub struct LanguageInfo {
    pub name: &'static str,
    pub language: Language,
    pub element_query: &'static str,
}

/// Get language information by language name.
pub fn get_language_info(lang_name: &str) -> Option<LanguageInfo> {
    match lang_name {
        "rust" => Some(LanguageInfo {
            name: "rust",
            language: tree_sitter_rust::language(),
            element_query: rust::ELEMENT_QUERY,
        }),
        _ => None,
    }
}
