//! Language detection by file extension.
//!
//! Maps file extensions to supported language identifiers.

const EXTENSION_MAP: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("py", "python"),
    ("go", "go"),
    ("java", "java"),
    ("ts", "typescript"),
    ("tsx", "tsx"),
];

pub fn language_from_extension(ext: &str) -> Option<&'static str> {
    EXTENSION_MAP
        .iter()
        .find(|(e, _)| e.eq_ignore_ascii_case(ext))
        .map(|(_, lang)| *lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension_happy_path() {
        assert_eq!(language_from_extension("rs"), Some("rust"));
        assert_eq!(language_from_extension("py"), Some("python"));
        assert_eq!(language_from_extension("go"), Some("go"));
        assert_eq!(language_from_extension("java"), Some("java"));
        assert_eq!(language_from_extension("ts"), Some("typescript"));
        assert_eq!(language_from_extension("tsx"), Some("tsx"));
    }

    #[test]
    fn test_language_from_extension_edge_case() {
        assert_eq!(language_from_extension("unknown"), None);
        assert_eq!(language_from_extension(""), None);
        assert_eq!(language_from_extension("RS"), Some("rust"));
    }
}
