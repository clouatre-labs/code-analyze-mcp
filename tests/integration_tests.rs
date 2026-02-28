mod fixtures;

use code_analyze_mcp::analyze::{analyze_directory, determine_mode};
use code_analyze_mcp::traversal::walk_directory;
use code_analyze_mcp::types::AnalysisMode;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_walk_directory_basic() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create test structure
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("README.md"), "# Test").unwrap();

    let entries = walk_directory(root, None).unwrap();

    // Should have root + src dir + 2 files
    assert!(entries.len() >= 3);

    // Check that we have files and directories
    let has_dir = entries.iter().any(|e| e.is_dir && e.path.ends_with("src"));
    let has_file = entries
        .iter()
        .any(|e| !e.is_dir && e.path.ends_with("main.rs"));
    assert!(has_dir);
    assert!(has_file);
}

#[test]
fn test_walk_directory_max_depth_limiting() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create nested structure
    fs::create_dir_all(root.join("a/b/c")).unwrap();
    fs::write(root.join("a/file1.rs"), "fn f1() {}").unwrap();
    fs::write(root.join("a/b/file2.rs"), "fn f2() {}").unwrap();
    fs::write(root.join("a/b/c/file3.rs"), "fn f3() {}").unwrap();

    // With max_depth=1, should only get root and immediate children
    let entries = walk_directory(root, Some(1)).unwrap();

    // Check that depth 2+ entries are not included
    let max_depth = entries.iter().map(|e| e.depth).max().unwrap_or(0);
    assert!(max_depth <= 1);
}

#[test]
fn test_walk_directory_symlink_detection() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::write(root.join("target.rs"), "fn target() {}").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        unix_fs::symlink(root.join("target.rs"), root.join("link.rs")).unwrap();

        let entries = walk_directory(root, None).unwrap();
        let symlink_entry = entries.iter().find(|e| e.path.ends_with("link.rs"));

        assert!(symlink_entry.is_some());
        let entry = symlink_entry.unwrap();
        assert!(entry.is_symlink);
        assert!(entry.symlink_target.is_some());
    }
}

#[test]
fn test_analyze_directory_with_rust_file() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a simple Rust file with a function
    let rust_code = r#"
fn hello() {
    println!("Hello");
}

fn world() {
    println!("World");
}
"#;
    fs::write(root.join("lib.rs"), rust_code).unwrap();

    let output = analyze_directory(root, None).unwrap();

    // Check that output contains expected sections with new format
    assert!(output.formatted.contains("SUMMARY:"));
    assert!(output.formatted.contains("Shown:"));
    assert!(output.formatted.contains("PATH [LOC, FUNCTIONS, CLASSES]"));
    assert!(output.formatted.contains("lib.rs"));
    assert!(output.formatted.contains("2F")); // 2 functions
}

#[test]
fn test_analyze_directory_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let output = analyze_directory(root, None).unwrap();

    // Should still have SUMMARY and PATH sections with new format
    assert!(output.formatted.contains("SUMMARY:"));
    assert!(output.formatted.contains("Shown: 0 files"));
    assert!(output.formatted.contains("PATH [LOC, FUNCTIONS, CLASSES]"));
}

#[test]
fn test_analyze_directory_binary_file_skipping() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a binary file (PNG header)
    fs::write(root.join("image.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    fs::write(root.join("lib.rs"), "fn test() {}").unwrap();

    // Should not panic, should include binary file with LOC only
    let output = analyze_directory(root, None).unwrap();
    assert!(output.formatted.contains("SUMMARY:"));
    // Binary file should be included with 0 LOC
    assert!(output.formatted.contains("image.png"));
}

#[test]
fn test_walk_directory_ignore_file_respected() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Arrange: Create .ignore file that excludes ignored.rs
    fs::write(root.join(".ignore"), "ignored.rs\n").unwrap();
    fs::write(root.join("ignored.rs"), "fn ignored() {}").unwrap();
    fs::write(root.join("kept.rs"), "fn kept() {}").unwrap();

    // Act: Walk the directory
    let entries = walk_directory(root, None).unwrap();

    // Assert: ignored.rs should not be in results, kept.rs should be
    let has_ignored = entries.iter().any(|e| e.path.ends_with("ignored.rs"));
    let has_kept = entries.iter().any(|e| e.path.ends_with("kept.rs"));

    assert!(!has_ignored, "ignored.rs should be excluded by .ignore");
    assert!(has_kept, "kept.rs should be included");
}

#[test]
fn test_walk_directory_ignore_precedence_over_gitignore() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Arrange: Create .gitignore that excludes foo.rs, then .ignore that includes it
    fs::write(root.join(".gitignore"), "foo.rs\n").unwrap();
    fs::write(root.join(".ignore"), "!foo.rs\n").unwrap();
    fs::write(root.join("foo.rs"), "fn foo() {}").unwrap();

    // Act: Walk the directory
    let entries = walk_directory(root, None).unwrap();

    // Assert: foo.rs should be included (proving .ignore precedence)
    let has_foo = entries.iter().any(|e| e.path.ends_with("foo.rs"));
    assert!(
        has_foo,
        "foo.rs should be included due to .ignore precedence over .gitignore"
    );
}

#[test]
fn test_analyze_unsupported_file_type() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create an unsupported file type (plain text)
    fs::write(
        root.join("notes.txt"),
        "This is a text file\nWith multiple lines",
    )
    .unwrap();
    fs::write(root.join("lib.rs"), "fn test() {}").unwrap();

    let output = analyze_directory(root, None).unwrap();

    // Should include both files
    assert!(output.formatted.contains("notes.txt"));
    assert!(output.formatted.contains("lib.rs"));

    // Verify unsupported file has LOC but no functions/classes
    let txt_analysis = output.files.iter().find(|f| f.path.contains("notes.txt"));
    assert!(txt_analysis.is_some());
    let txt = txt_analysis.unwrap();
    assert_eq!(txt.line_count, 2);
    assert_eq!(txt.function_count, 0);
    assert_eq!(txt.class_count, 0);
    assert_eq!(txt.language, "unknown");
}

#[test]
fn test_output_format_compliance() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a Rust file with struct and function
    let rust_code = r#"
struct Point {
    x: i32,
    y: i32,
}

fn distance() -> f64 {
    0.0
}
"#;
    fs::write(root.join("lib.rs"), rust_code).unwrap();

    let output = analyze_directory(root, None).unwrap();

    // Verify SUMMARY: format with colon
    assert!(output.formatted.contains("SUMMARY:"));

    // Verify Shown: format with compact metrics
    assert!(output.formatted.contains("Shown: 1 files"));
    assert!(output.formatted.contains("L,"));
    assert!(output.formatted.contains("F,"));
    assert!(output.formatted.contains("C (max_depth=0)"));

    // Verify PATH header format
    assert!(output.formatted.contains("PATH [LOC, FUNCTIONS, CLASSES]"));

    // Verify compact file metrics format [NL, NF, NC]
    assert!(output.formatted.contains("["));
    assert!(output.formatted.contains("]"));
}

#[test]
fn test_determine_mode_directory() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let mode = determine_mode(root.to_str().unwrap(), None);
    assert_eq!(mode, AnalysisMode::Overview);
}

#[test]
fn test_determine_mode_file() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::write(root.join("test.rs"), "fn test() {}").unwrap();

    let file_path = root.join("test.rs");
    let mode = determine_mode(file_path.to_str().unwrap(), None);
    assert_eq!(mode, AnalysisMode::FileDetails);
}

#[test]
fn test_determine_mode_with_focus() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let mode = determine_mode(root.to_str().unwrap(), Some("my_function"));
    assert_eq!(mode, AnalysisMode::SymbolFocus);
}
