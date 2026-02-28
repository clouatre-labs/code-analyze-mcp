use std::collections::HashMap;
use std::path::PathBuf;

pub struct FileResult {
    pub relative_path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    pub language: Option<String>,
    pub line_count: usize,
    pub function_count: usize,
    pub class_count: usize,
}

pub fn format_structure_output(results: &[FileResult], max_depth: usize) -> String {
    let mut output = String::new();

    let files: Vec<&FileResult> = results.iter().filter(|r| !r.is_dir).collect();
    let total_files = files.len();
    let total_loc: usize = files.iter().map(|r| r.line_count).sum();
    let total_functions: usize = files.iter().map(|r| r.function_count).sum();
    let total_classes: usize = files.iter().map(|r| r.class_count).sum();

    let mut lang_counts: HashMap<&str, usize> = HashMap::new();
    for r in &files {
        if let Some(lang) = &r.language {
            *lang_counts.entry(lang.as_str()).or_insert(0) += 1;
        }
    }

    output.push_str("SUMMARY:\n");

    let mut summary = format!("Shown: {} files, {}L", total_files, total_loc);
    if total_functions > 0 {
        summary.push_str(&format!(", {}F", total_functions));
    }
    if total_classes > 0 {
        summary.push_str(&format!(", {}C", total_classes));
    }
    if max_depth > 0 {
        summary.push_str(&format!(" (max_depth={})", max_depth));
    }
    output.push_str(&summary);
    output.push('\n');

    if !lang_counts.is_empty() {
        let mut langs: Vec<(&str, usize)> = lang_counts.into_iter().collect();
        langs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let lang_strs: Vec<String> = langs
            .iter()
            .map(|(lang, count)| {
                let pct =
                    ((*count * 100) as f64 / total_files.max(1) as f64).round() as usize;
                format!("{} ({}%)", lang, pct)
            })
            .collect();
        output.push_str("Languages: ");
        output.push_str(&lang_strs.join(", "));
        output.push('\n');
    }

    output.push('\n');
    output.push_str("PATH [LOC, FUNCTIONS, CLASSES] <FLAGS>\n");

    for result in results {
        let indent = "  ".repeat(result.depth.saturating_sub(1));
        let name = result
            .relative_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| result.relative_path.to_string_lossy().to_string());

        if result.is_dir {
            if result.is_symlink {
                let target = result
                    .symlink_target
                    .as_ref()
                    .map(|t| t.display().to_string())
                    .unwrap_or_default();
                output.push_str(&format!("{}{} -> {}\n", indent, name, target));
            } else {
                output.push_str(&format!("{}{}/\n", indent, name));
            }
        } else {
            let mut metrics = format!("[{}L", result.line_count);
            if result.function_count > 0 {
                metrics.push_str(&format!(", {}F", result.function_count));
            }
            if result.class_count > 0 {
                metrics.push_str(&format!(", {}C", result.class_count));
            }
            metrics.push(']');

            if result.is_symlink {
                let target = result
                    .symlink_target
                    .as_ref()
                    .map(|t| t.display().to_string())
                    .unwrap_or_default();
                output.push_str(&format!("{}{} {} -> {}\n", indent, name, metrics, target));
            } else {
                output.push_str(&format!("{}{} {}\n", indent, name, metrics));
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_file(name: &str, depth: usize, loc: usize, funcs: usize, classes: usize) -> FileResult {
        FileResult {
            relative_path: PathBuf::from(name),
            depth,
            is_dir: false,
            is_symlink: false,
            symlink_target: None,
            language: Some("rust".to_string()),
            line_count: loc,
            function_count: funcs,
            class_count: classes,
        }
    }

    fn make_dir(name: &str, depth: usize) -> FileResult {
        FileResult {
            relative_path: PathBuf::from(name),
            depth,
            is_dir: true,
            is_symlink: false,
            symlink_target: None,
            language: None,
            line_count: 0,
            function_count: 0,
            class_count: 0,
        }
    }

    #[test]
    fn test_format_structure_summary_line() {
        let results = vec![make_file("main.rs", 1, 50, 3, 1)];
        let out = format_structure_output(&results, 3);
        assert!(out.contains("Shown: 1 files, 50L, 3F, 1C (max_depth=3)"));
    }

    #[test]
    fn test_format_omits_zero_functions_and_classes() {
        let results = vec![make_file("lib.rs", 1, 10, 0, 0)];
        let out = format_structure_output(&results, 0);
        assert!(out.contains("[10L]"));
        assert!(!out.contains("0F"));
        assert!(!out.contains("0C"));
    }

    #[test]
    fn test_format_max_depth_zero_no_depth_label() {
        let results = vec![make_file("a.rs", 1, 5, 0, 0)];
        let out = format_structure_output(&results, 0);
        assert!(!out.contains("max_depth="));
    }

    #[test]
    fn test_format_indentation() {
        let results = vec![
            make_dir("src", 1),
            make_file("src/main.rs", 2, 10, 1, 0),
        ];
        let out = format_structure_output(&results, 0);
        let lines: Vec<&str> = out.lines().collect();
        let src_line = lines.iter().find(|l| l.contains("src/")).unwrap();
        let main_line = lines.iter().find(|l| l.contains("main.rs")).unwrap();
        assert!(!src_line.starts_with(' '));
        assert!(main_line.starts_with("  "));
    }

    #[test]
    fn test_format_language_summary() {
        let results = vec![
            make_file("a.rs", 1, 10, 1, 0),
            make_file("b.rs", 1, 20, 2, 1),
        ];
        let out = format_structure_output(&results, 0);
        assert!(out.contains("Languages: rust (100%)"));
    }

    #[test]
    fn test_format_headers_present() {
        let out = format_structure_output(&[], 0);
        assert!(out.contains("SUMMARY:"));
        assert!(out.contains("PATH [LOC, FUNCTIONS, CLASSES] <FLAGS>"));
    }
}
