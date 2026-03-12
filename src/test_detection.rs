//! Test file detection using path heuristics.
//!
//! Identifies test files based on directory and filename patterns.
//! Supports Rust, Python, Go, Java, TypeScript, and JavaScript.

use std::path::Path;

/// Detect if a file path represents a test file based on path-based heuristics.
///
/// Checks for:
/// - Directory patterns: tests/, test/, __tests__/, spec/
/// - Filename patterns:
///   - Rust: test_*.rs, *_test.rs
///   - Python: test_*.py, *_test.py
///   - Go: *_test.go
///   - Java: Test*.java, *Test.java
///   - TypeScript/JavaScript: *.test.ts, *.test.js, *.spec.ts, *.spec.js
///
/// Returns true if the path matches any test heuristic, false otherwise.
pub fn is_test_file(path: &Path) -> bool {
    // Check directory components for test directories
    for component in path.components() {
        if let Some("tests" | "test" | "__tests__" | "spec") = component.as_os_str().to_str() {
            return true;
        }
    }

    // Check filename patterns
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // Rust patterns
    if file_name.starts_with("test_") && file_name.ends_with(".rs") {
        return true;
    }
    if file_name.ends_with("_test.rs") {
        return true;
    }

    // Python patterns
    if file_name.starts_with("test_") && file_name.ends_with(".py") {
        return true;
    }
    if file_name.ends_with("_test.py") {
        return true;
    }

    // Go patterns
    if file_name.ends_with("_test.go") {
        return true;
    }

    // Java patterns
    if file_name.starts_with("Test") && file_name.ends_with(".java") {
        return true;
    }
    if file_name.ends_with("Test.java") {
        return true;
    }

    // TypeScript/JavaScript patterns
    if file_name.ends_with(".test.ts") || file_name.ends_with(".test.js") {
        return true;
    }
    if file_name.ends_with(".spec.ts") || file_name.ends_with(".spec.js") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_pattern_detects_test_file() {
        assert!(is_test_file(Path::new("test_utils.rs")));
        assert!(is_test_file(Path::new("utils_test.rs")));
    }

    #[test]
    fn filename_pattern_rejects_production_file() {
        assert!(!is_test_file(Path::new("utils.rs")));
        assert!(!is_test_file(Path::new("main.rs")));
    }

    #[test]
    fn directory_pattern_detects_test_file() {
        assert!(is_test_file(Path::new("tests/utils.rs")));
    }

    #[test]
    fn directory_pattern_detects_nested_test_file() {
        assert!(is_test_file(Path::new("src/tests/utils.rs")));
        assert!(!is_test_file(Path::new("src/utils.rs")));
    }
}
