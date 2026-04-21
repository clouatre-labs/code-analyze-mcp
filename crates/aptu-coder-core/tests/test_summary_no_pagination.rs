// SPDX-FileCopyrightText: 2026 aptu-coder contributors
// SPDX-License-Identifier: Apache-2.0
use aptu_coder_core::analyze::analyze_directory;
use std::fs;
use tempfile::TempDir;

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
        aptu_coder_core::formatter::format_summary(&output.entries, &output.files, None, None);

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
