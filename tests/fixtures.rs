use code_analyze_mcp::types::{AnalysisMode, AnalysisResult, FileInfo};

/// Create a basic test result for structure mode analysis.
#[allow(dead_code)]
pub fn create_test_result(path: &str) -> AnalysisResult {
    AnalysisResult {
        path: path.to_string(),
        mode: AnalysisMode::Overview,
        import_count: 0,
        main_line: None,
        files: vec![],
        functions: vec![],
        classes: vec![],
        references: vec![],
    }
}

/// Create a test result with file information.
#[allow(dead_code)]
pub fn create_test_result_with_files(path: &str, files: Vec<FileInfo>) -> AnalysisResult {
    AnalysisResult {
        path: path.to_string(),
        mode: AnalysisMode::Overview,
        import_count: 0,
        main_line: None,
        files,
        functions: vec![],
        classes: vec![],
        references: vec![],
    }
}

/// Create a test result with sample CallInfo data for downstream issues.
#[allow(dead_code)]
pub fn create_test_result_with_calls(path: &str) -> AnalysisResult {
    AnalysisResult {
        path: path.to_string(),
        mode: AnalysisMode::SymbolFocus,
        import_count: 0,
        main_line: None,
        files: vec![],
        functions: vec![],
        classes: vec![],
        references: vec![],
    }
}
