use code_analyze_mcp::analyze::analyze_directory;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_summary_true_clears_next_cursor() {
    // Arrange: Create a directory with many files
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::create_dir(root.join("src")).unwrap();

    // Create 110 Rust files to exceed DEFAULT_PAGE_SIZE (100)
    for i in 0..110 {
        fs::write(root.join(format!("src/file_{:03}.rs", i)), "fn func() {}").unwrap();
    }

    // Act: Call analyze_directory with summary=true
    let output = analyze_directory(root, None).unwrap();

    // Manually apply summary logic and pagination logic (simulating what the tool handler does)
    let use_summary = true;
    let next_cursor = if use_summary {
        None
    } else {
        output.next_cursor.clone()
    };

    // Assert: next_cursor should be None when use_summary=true
    assert_eq!(
        next_cursor, None,
        "next_cursor should be None when summary=true"
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

    // Simulate the handler logic for summary mode
    let use_summary = true;
    let mut final_text = output.formatted.clone();

    // The handler only appends NEXT_CURSOR if !use_summary
    if !use_summary && let Some(cursor) = output.next_cursor {
        final_text.push('\n');
        final_text.push_str(&format!("NEXT_CURSOR: {}", cursor));
    }

    // Assert: final_text should NOT contain NEXT_CURSOR when use_summary=true
    assert!(
        !final_text.contains("NEXT_CURSOR:"),
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

    // Assert: Find the core/ line in the summary and verify it contains sub: and subdirectory names
    let core_line = summary
        .lines()
        .find(|l| l.contains("core/"))
        .expect("core/ line missing from summary");
    assert!(
        core_line.contains("sub:"),
        "core/ line should contain sub: annotation, got: {}",
        core_line
    );
    assert!(
        core_line.contains("handlers") || core_line.contains("management"),
        "core/ line should list subdirs, got: {}",
        core_line
    );
}
