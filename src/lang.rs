//! Language detection by file extension.
//!
//! Maps file extensions to supported language identifiers.

const EXTENSION_MAP: &[(&str, &str)] = &[
    #[cfg(feature = "lang-fortran")]
    ("f", "fortran"),
    #[cfg(feature = "lang-fortran")]
    ("f03", "fortran"),
    #[cfg(feature = "lang-fortran")]
    ("f08", "fortran"),
    #[cfg(feature = "lang-fortran")]
    ("f77", "fortran"),
    #[cfg(feature = "lang-fortran")]
    ("f90", "fortran"),
    #[cfg(feature = "lang-fortran")]
    ("f95", "fortran"),
    #[cfg(feature = "lang-fortran")]
    ("for", "fortran"),
    #[cfg(feature = "lang-fortran")]
    ("ftn", "fortran"),
    #[cfg(feature = "lang-go")]
    ("go", "go"),
    #[cfg(feature = "lang-java")]
    ("java", "java"),
    #[cfg(feature = "lang-python")]
    ("py", "python"),
    #[cfg(feature = "lang-rust")]
    ("rs", "rust"),
    #[cfg(feature = "lang-typescript")]
    ("ts", "typescript"),
    #[cfg(feature = "lang-tsx")]
    ("tsx", "tsx"),
];

/// Returns the language identifier for the given file extension, or `None` if unsupported.
///
/// The lookup is case-insensitive. Supported extensions include `rs`, `py`, `go`, `java`,
/// `ts`, `tsx`, `f90`, `f95`, `for`, `ftn`, and other Fortran variants.
#[must_use]
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
        assert_eq!(language_from_extension("f90"), Some("fortran"));
        assert_eq!(language_from_extension("for"), Some("fortran"));
        assert_eq!(language_from_extension("ftn"), Some("fortran"));
    }

    #[test]
    fn test_language_from_extension_edge_case() {
        assert_eq!(language_from_extension("unknown"), None);
        assert_eq!(language_from_extension(""), None);
        assert_eq!(language_from_extension("RS"), Some("rust"));
        // Uppercase Fortran extensions resolved via eq_ignore_ascii_case
        assert_eq!(language_from_extension("F90"), Some("fortran"));
        assert_eq!(language_from_extension("FOR"), Some("fortran"));
    }
}
