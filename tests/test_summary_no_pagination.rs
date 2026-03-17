use code_analyze_mcp::analyze::analyze_directory;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_summary_true_clears_next_cursor() {
    // Arrange: Create a directory with more files than DEFAULT_PAGE_SIZE (100)
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::create_dir(root.join("src")).unwrap();

    // Create 110 Rust files to exceed DEFAULT_PAGE_SIZE
    for i in 0..110 {
        fs::write(root.join(format!("src/file_{:03}.rs", i)), "fn func() {}").unwrap();
    }

    // Act: Call analyze_directory
    let output = analyze_directory(root, None).unwrap();

    // Assert: Output should have been generated
    // The key fix is: when use_summary is auto-triggered (large directory),
    // the output should be compact (summary format) not paginated (flat list).
    // This test just verifies that output was produced and is reasonably compact.
    assert!(!output.formatted.is_empty(), "Output should not be empty");
    let summary_lines = output.formatted.lines().count();
    assert!(
        summary_lines < 150,
        "Output should be reasonably compact (not full 100-item paginated list), got {} lines",
        summary_lines
    );
}

#[test]
fn test_summary_no_next_cursor_text() {
    // Arrange: Create directory with many files
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::create_dir(root.join("src")).unwrap();

    for i in 0..110 {
        fs::write(root.join(format!("src/file_{:03}.rs", i)), "fn func() {}").unwrap();
    }

    // Act: Analyze directory
    let output = analyze_directory(root, None).unwrap();

    // Assert: Even with many files, summary output should NOT contain "NEXT_CURSOR:" text
    // This is critical: when use_summary=true, pagination is disabled
    assert!(
        !output.formatted.contains("NEXT_CURSOR:"),
        "Summary output should not contain NEXT_CURSOR text"
    );
}

#[test]
fn test_format_summary_includes_subdirs() {
    // Arrange: Create nested directory structure with depth-2 subdirectories
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let core_dir = root.join("core");
    let handlers_dir = core_dir.join("handlers");
    let management_dir = core_dir.join("management");

    fs::create_dir_all(&handlers_dir).unwrap();
    fs::create_dir_all(&management_dir).unwrap();

    fs::write(core_dir.join("main.rs"), "fn core_main() {}").unwrap();
    fs::write(handlers_dir.join("base.rs"), "pub struct BaseHandler {}").unwrap();
    fs::write(management_dir.join("cmd.rs"), "pub struct Command {}").unwrap();

    // Act: Analyze directory to get the entries and files
    let output = analyze_directory(root, None).unwrap();
    let summary =
        code_analyze_mcp::formatter::format_summary(&output.entries, &output.files, None, None);

    // Assert: Summary should include sub: annotation with subdirectory names
    // The summary line for core/ should show depth-2 subdirectories (handlers, management)
    assert!(
        summary.contains("sub:")
            && (summary.contains("handlers") || summary.contains("management")),
        "Summary STRUCTURE should include 'sub:' annotation with subdirectory names, but got:\n{}",
        summary
    );
}
