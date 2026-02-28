use rayon::prelude::*;
use std::path::Path;

use crate::formatter::{format_structure_output, FileResult};
use crate::lang::language_from_extension;
use crate::languages::get_language_info;
use crate::parser::ElementExtractor;
use crate::traversal::{walk_directory, WalkOptions};
use crate::types::{AnalysisMode, AnalyzeParams};

/// Auto-detect the analysis mode from the path and params.
/// An explicit focus symbol → SymbolFocus; a directory path → Overview; otherwise → FileDetails.
pub fn determine_mode(path: &Path, params: &AnalyzeParams) -> AnalysisMode {
    if params.focus.is_some() {
        return AnalysisMode::SymbolFocus;
    }
    if path.is_dir() {
        return AnalysisMode::Overview;
    }
    AnalysisMode::FileDetails
}

/// Walk `root`, parse every file in parallel, and return a formatted structure report.
pub fn analyze_directory(root: &Path, max_depth: usize) -> String {
    let options = WalkOptions { max_depth };
    let entries = walk_directory(root, &options);

    let results: Vec<FileResult> = entries
        .par_iter()
        .map(|entry| {
            if entry.is_dir {
                return FileResult {
                    relative_path: entry.relative_path.clone(),
                    depth: entry.depth,
                    is_dir: true,
                    is_symlink: entry.is_symlink,
                    symlink_target: entry.symlink_target.clone(),
                    language: None,
                    line_count: 0,
                    function_count: 0,
                    class_count: 0,
                };
            }

            let ext = entry
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            let language = language_from_extension(ext);

            // Skip binary / non-UTF-8 files gracefully
            let source = match std::fs::read_to_string(&entry.path) {
                Ok(s) => s,
                Err(_) => {
                    return FileResult {
                        relative_path: entry.relative_path.clone(),
                        depth: entry.depth,
                        is_dir: false,
                        is_symlink: entry.is_symlink,
                        symlink_target: entry.symlink_target.clone(),
                        language: None,
                        line_count: 0,
                        function_count: 0,
                        class_count: 0,
                    };
                }
            };

            let (line_count, function_count, class_count) =
                match language.and_then(get_language_info) {
                    Some(lang_info) => {
                        let m = ElementExtractor::extract_with_depth(&source, lang_info);
                        (m.line_count, m.function_count, m.class_count)
                    }
                    // Unsupported file type: report LOC only
                    None => (source.lines().count(), 0, 0),
                };

            FileResult {
                relative_path: entry.relative_path.clone(),
                depth: entry.depth,
                is_dir: false,
                is_symlink: entry.is_symlink,
                symlink_target: entry.symlink_target.clone(),
                language: language.map(str::to_string),
                line_count,
                function_count,
                class_count,
            }
        })
        .collect();

    format_structure_output(&results, max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_directory_produces_summary() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let output = analyze_directory(&src, 3);
        assert!(output.contains("SUMMARY:"), "Output must contain SUMMARY header");
        assert!(
            output.contains("PATH [LOC, FUNCTIONS, CLASSES]"),
            "Output must contain column header"
        );
    }

    #[test]
    fn test_analyze_directory_lists_rust_files() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let output = analyze_directory(&src, 3);
        assert!(output.contains("lib.rs"), "lib.rs must appear in output");
        assert!(output.contains("main.rs"), "main.rs must appear in output");
    }

    #[test]
    fn test_analyze_directory_depth_limiting() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let shallow = analyze_directory(&src, 1);
        let deep = analyze_directory(&src, 3);

        let rs_in_shallow = shallow.lines().filter(|l| l.contains(".rs")).count();
        let rs_in_deep = deep.lines().filter(|l| l.contains(".rs")).count();
        assert!(
            rs_in_deep >= rs_in_shallow,
            "Deeper traversal must show at least as many files"
        );
    }

    #[test]
    fn test_analyze_directory_max_depth_zero_unlimited() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let output = analyze_directory(&src, 0);
        assert!(output.contains("SUMMARY:"));
        assert!(
            !output.contains("max_depth="),
            "max_depth=0 should not be printed in summary"
        );
    }

    #[test]
    fn test_determine_mode_directory() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let params = AnalyzeParams {
            path: path.to_str().unwrap().to_string(),
            mode: None,
            max_depth: None,
            focus: None,
            follow_depth: None,
        };
        assert!(matches!(determine_mode(&path, &params), AnalysisMode::Overview));
    }

    #[test]
    fn test_determine_mode_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs");
        let params = AnalyzeParams {
            path: path.to_str().unwrap().to_string(),
            mode: None,
            max_depth: None,
            focus: None,
            follow_depth: None,
        };
        assert!(matches!(
            determine_mode(&path, &params),
            AnalysisMode::FileDetails
        ));
    }

    #[test]
    fn test_determine_mode_focus_overrides() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let params = AnalyzeParams {
            path: path.to_str().unwrap().to_string(),
            mode: None,
            max_depth: None,
            focus: Some("my_fn".to_string()),
            follow_depth: None,
        };
        assert!(matches!(
            determine_mode(&path, &params),
            AnalysisMode::SymbolFocus
        ));
    }
}
