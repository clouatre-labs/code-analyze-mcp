/// Shared test fixtures for code-analyze-mcp integration tests.
///
/// Other integration test files can include these helpers via:
///   #[path = "fixtures.rs"] mod fixtures;
use code_analyze_mcp::types::{CallInfo, FileInfo};

// ---------------------------------------------------------------------------
// Fixture helpers (reused by subsequent issues)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn create_test_result() -> FileInfo {
    FileInfo {
        path: "test.rs".to_string(),
        language: "rust".to_string(),
        line_count: 100,
        function_count: 5,
        class_count: 2,
    }
}

#[allow(dead_code)]
pub fn create_test_result_with_calls() -> (FileInfo, Vec<CallInfo>) {
    let file = create_test_result();
    let calls = vec![CallInfo {
        caller: "main".to_string(),
        callee: "foo".to_string(),
        line: 10,
        column: 5,
    }];
    (file, calls)
}

// ---------------------------------------------------------------------------
// Integration tests: structure mode
// ---------------------------------------------------------------------------

#[test]
fn test_structure_output_has_headers() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let output = code_analyze_mcp::analyze::analyze_directory(&src, 3);

    assert!(output.contains("SUMMARY:"), "Missing SUMMARY header");
    assert!(
        output.contains("PATH [LOC, FUNCTIONS, CLASSES] <FLAGS>"),
        "Missing column header"
    );
    assert!(output.contains("Languages:"), "Missing language summary");
}

#[test]
fn test_structure_output_lists_known_files() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let output = code_analyze_mcp::analyze::analyze_directory(&src, 3);

    assert!(output.contains("lib.rs"), "lib.rs must be listed");
    assert!(output.contains("main.rs"), "main.rs must be listed");
    assert!(output.contains("analyze.rs"), "analyze.rs must be listed");
}

#[test]
fn test_structure_output_shows_loc_and_functions() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let output = code_analyze_mcp::analyze::analyze_directory(&src, 3);

    // Match lines that contain a metrics bracket like "[42L" indicating file metrics
    let file_lines: Vec<&str> = output
        .lines()
        .filter(|l| {
            // File metric lines follow the pattern: "  name.ext [NL..." or "name.ext [NL..."
            let trimmed = l.trim_start();
            trimmed.contains(" [") && trimmed.contains('L') && trimmed.contains(']')
        })
        .collect();
    assert!(!file_lines.is_empty(), "No file metric lines found");
    for line in &file_lines {
        assert!(line.contains('['), "Metric bracket missing in: {}", line);
        assert!(line.contains('L'), "LOC marker missing in: {}", line);
    }
}

#[test]
fn test_depth_limiting() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let shallow = code_analyze_mcp::analyze::analyze_directory(&src, 1);
    let deep = code_analyze_mcp::analyze::analyze_directory(&src, 3);

    // src/languages/ is at depth 1, its contents at depth 2.
    // A depth-3 run must include at least as many .rs files.
    let count = |s: &str| s.lines().filter(|l| l.contains(".rs")).count();
    assert!(
        count(&deep) >= count(&shallow),
        "Deeper traversal should expose at least as many .rs files"
    );
}

#[test]
fn test_max_depth_zero_means_unlimited() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let unlimited = code_analyze_mcp::analyze::analyze_directory(&src, 0);
    let limited = code_analyze_mcp::analyze::analyze_directory(&src, 1);

    assert!(
        !unlimited.contains("max_depth="),
        "max_depth=0 must not print depth label"
    );

    let count = |s: &str| s.lines().filter(|l| l.contains(".rs")).count();
    assert!(count(&unlimited) >= count(&limited));
}

#[test]
fn test_gitignore_patterns_respected() {
    use std::fs;
    let tmp = std::env::temp_dir().join("fixtures_gitignore_test");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join(".gitignore"), "secret.rs\n").unwrap();
    fs::write(tmp.join("secret.rs"), "fn secret() {}").unwrap();
    fs::write(tmp.join("public.rs"), "fn public() {}").unwrap();

    let output = code_analyze_mcp::analyze::analyze_directory(&tmp, 0);

    assert!(output.contains("public.rs"), "public.rs should be shown");
    assert!(
        !output.contains("secret.rs"),
        "secret.rs is gitignored and must be hidden"
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_fixture_helpers_create_valid_structs() {
    let result = create_test_result();
    assert_eq!(result.language, "rust");
    assert_eq!(result.function_count, 5);
    assert_eq!(result.class_count, 2);

    let (file, calls) = create_test_result_with_calls();
    assert_eq!(file.path, "test.rs");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].callee, "foo");
}
