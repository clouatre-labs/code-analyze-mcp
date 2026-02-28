pub mod rust;

pub struct LanguageInfo {
    pub name: &'static str,
    pub element_query: &'static str,
    pub tree_sitter_language: fn() -> tree_sitter::Language,
}

pub fn get_language_info(language: &str) -> Option<&'static LanguageInfo> {
    match language {
        "rust" => Some(&rust::LANGUAGE_INFO),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_info_rust() {
        let info = get_language_info("rust");
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "rust");
    }

    #[test]
    fn test_get_language_info_unsupported() {
        assert!(get_language_info("cobol").is_none());
        assert!(get_language_info("").is_none());
    }
}
