mod fixtures;

use code_analyze_mcp::analyze::analyze_directory;
use code_analyze_mcp::traversal::walk_directory;
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

    // Check that output contains expected sections
    assert!(output.formatted.contains("SUMMARY"));
    assert!(output.formatted.contains("PATH"));
    assert!(output.formatted.contains("lib.rs"));
}

#[test]
fn test_analyze_directory_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let output = analyze_directory(root, None).unwrap();

    // Should still have SUMMARY and PATH sections
    assert!(output.formatted.contains("SUMMARY"));
    assert!(output.formatted.contains("PATH"));
}

#[test]
fn test_analyze_directory_binary_file_skipping() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a binary file (PNG header)
    fs::write(root.join("image.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    fs::write(root.join("lib.rs"), "fn test() {}").unwrap();

    // Should not panic, should skip binary file
    let output = analyze_directory(root, None).unwrap();
    assert!(output.formatted.contains("SUMMARY"));
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
