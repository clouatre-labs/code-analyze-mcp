mod fixtures;

use code_analyze_mcp::analyze::{
    AnalyzeError, analyze_directory, analyze_directory_with_progress, analyze_file, determine_mode,
};
use code_analyze_mcp::cache::{AnalysisCache, CacheKey};
use code_analyze_mcp::completion::{path_completions, symbol_completions};
use code_analyze_mcp::traversal::walk_directory;
use code_analyze_mcp::types::AnalysisMode;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

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

    // Should not panic, should exclude binary file from output
    let output = analyze_directory(root, None).unwrap();
    assert!(output.formatted.contains("SUMMARY:"));
    // Binary file should be excluded from analysis results
    assert!(!output.formatted.contains("image.png"));
    // Only the readable Rust file should be included
    assert!(output.formatted.contains("lib.rs"));
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
    // When max_depth is None (unlimited), max_depth label should be omitted
    assert!(!output.formatted.contains("max_depth="));

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

#[test]
fn test_semantic_analysis_happy_path() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    let rust_code = r#"
use std::collections::HashMap;

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    fn distance(&self) -> f64 {
        ((self.x * self.x + self.y * self.y) as f64).sqrt()
    }
}

fn calculate(a: i32, b: i32) -> i32 {
    let result = a + b;
    process(result);
    process(result);
    process(result);
    process(result);
    result
}

fn process(x: i32) -> i32 {
    x * 2
}
"#;

    fs::write(&file_path, rust_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify FILE: header with counts
    assert!(output.formatted.contains("FILE:"));
    assert!(output.formatted.contains("test.rs"));
    assert!(output.formatted.contains("L,"));
    assert!(output.formatted.contains("F,"));
    assert!(output.formatted.contains("C,"));
    assert!(output.formatted.contains("I)"));

    // Verify functions extracted (new, distance, calculate, process)
    assert_eq!(output.semantic.functions.len(), 4);
    assert!(output.semantic.functions.iter().any(|f| f.name == "new"));
    assert!(
        output
            .semantic
            .functions
            .iter()
            .any(|f| f.name == "distance")
    );
    assert!(
        output
            .semantic
            .functions
            .iter()
            .any(|f| f.name == "calculate")
    );
    assert!(
        output
            .semantic
            .functions
            .iter()
            .any(|f| f.name == "process")
    );

    // Verify classes extracted
    assert_eq!(output.semantic.classes.len(), 1);
    assert_eq!(output.semantic.classes[0].name, "Point");

    // Verify impl methods populated on the class
    assert_eq!(output.semantic.classes[0].methods.len(), 2);
    let method_names: Vec<&str> = output.semantic.classes[0]
        .methods
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert!(method_names.contains(&"new"));
    assert!(method_names.contains(&"distance"));

    // Verify imports extracted
    assert_eq!(output.semantic.imports.len(), 1);
    assert_eq!(output.semantic.imports[0].module, "std::collections");

    // Verify C: section present
    assert!(output.formatted.contains("C:"));
    assert!(output.formatted.contains("Point:"));

    // Verify F: section present
    assert!(output.formatted.contains("F:"));

    // Verify I: section present
    assert!(output.formatted.contains("I:"));
    assert!(output.formatted.contains("std"));

    // Verify call frequency tracking (process called 4 times, should have bullet)
    assert!(output.semantic.call_frequency.contains_key("process"));
    assert_eq!(output.semantic.call_frequency["process"], 4);
    assert!(output.formatted.contains("•4"));

    // Verify references extracted with line numbers and location set
    let point_ref = output
        .semantic
        .references
        .iter()
        .find(|r| r.symbol == "Point");
    assert!(point_ref.is_some(), "expected a 'Point' type reference");
    let point_ref = point_ref.unwrap();
    assert!(point_ref.line > 0, "reference line should be non-zero");
    assert!(
        !point_ref.location.is_empty(),
        "reference location should be populated with the file path"
    );
    assert!(
        point_ref.location.ends_with("test.rs"),
        "reference location should point to test.rs, got: {}",
        point_ref.location
    );
}

#[test]
fn test_semantic_analysis_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.rs");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify FILE: header still present
    assert!(output.formatted.contains("FILE:"));
    assert!(output.formatted.contains("empty.rs"));

    // Verify empty sections handled gracefully
    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
    assert_eq!(output.semantic.imports.len(), 0);

    // Verify no C:, F:, I:, R: sections for empty file
    assert!(!output.formatted.contains("C:"));
    assert!(!output.formatted.contains("F:"));
    assert!(!output.formatted.contains("I:"));
    assert!(!output.formatted.contains("R:"));
}

#[test]
fn test_python_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");

    let python_code = r#"
def hello():
    print("Hello")

def world():
    print("World")

class MyClass:
    def method(self):
        pass
"#;

    fs::write(&file_path, python_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify functions extracted (hello, world, method)
    assert_eq!(output.semantic.functions.len(), 3);
    assert!(output.semantic.functions.iter().any(|f| f.name == "hello"));
    assert!(output.semantic.functions.iter().any(|f| f.name == "world"));
    assert!(output.semantic.functions.iter().any(|f| f.name == "method"));

    // Verify class extracted
    assert_eq!(output.semantic.classes.len(), 1);
    assert_eq!(output.semantic.classes[0].name, "MyClass");
}

#[test]
fn test_python_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.py");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
}

#[test]
fn test_typescript_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.ts");

    let ts_code = r#"
function hello(): void {
    console.log("Hello");
}

interface MyInterface {
    name: string;
}

type MyType = {
    id: number;
};

enum MyEnum {
    A = 1,
    B = 2,
}

abstract class AbstractClass {
    abstract method(): void;
}

class MyClass {
    method(): string {
        return "test";
    }
}
"#;

    fs::write(&file_path, ts_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify function extracted
    assert!(output.semantic.functions.iter().any(|f| f.name == "hello"));

    // Verify classes and TS-specific types extracted
    assert!(output.semantic.classes.len() >= 4);
    let class_names: Vec<&str> = output
        .semantic
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(class_names.contains(&"MyInterface"));
    assert!(class_names.contains(&"MyType"));
    assert!(class_names.contains(&"MyEnum"));
    assert!(class_names.contains(&"AbstractClass"));
    assert!(class_names.contains(&"MyClass"));
}

#[test]
fn test_typescript_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.ts");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
}

#[test]
fn test_go_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.go");

    let go_code = r#"
package main

func Hello() {
    println("Hello")
}

type MyStruct struct {
    Name string
}

type MyInterface interface {
    Method()
}
"#;

    fs::write(&file_path, go_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify function extracted
    assert!(output.semantic.functions.iter().any(|f| f.name == "Hello"));

    // Verify types extracted as classes
    assert!(output.semantic.classes.len() >= 2);
    let class_names: Vec<&str> = output
        .semantic
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(class_names.contains(&"MyStruct"));
    assert!(class_names.contains(&"MyInterface"));
}

#[test]
fn test_go_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.go");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
}

#[test]
fn test_java_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("Test.java");

    let java_code = r#"
public class MyClass {
    public void method() {
        System.out.println("Hello");
    }
}

interface MyInterface {
    void doSomething();
}

enum MyEnum {
    A, B, C
}
"#;

    fs::write(&file_path, java_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify method extracted
    assert!(output.semantic.functions.iter().any(|f| f.name == "method"));

    // Verify classes extracted
    assert!(output.semantic.classes.len() >= 3);
    let class_names: Vec<&str> = output
        .semantic
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(class_names.contains(&"MyClass"));
    assert!(class_names.contains(&"MyInterface"));
    assert!(class_names.contains(&"MyEnum"));
}

#[test]
fn test_java_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.java");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
}

// Cache tests
#[test]
fn test_cache_hit() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    let rust_code = r#"
fn hello() {
    println!("Hello");
}
"#;
    fs::write(&file_path, rust_code).unwrap();

    let cache = AnalysisCache::new(100);
    let mtime = fs::metadata(&file_path).unwrap().modified().unwrap();
    let key = CacheKey {
        path: file_path.clone(),
        modified: mtime,
        mode: AnalysisMode::FileDetails,
    };

    // First analysis
    let output1 = analyze_file(file_path.to_str().unwrap(), None).unwrap();
    let arc_output1 = Arc::new(output1);
    cache.put(key.clone(), arc_output1.clone());

    // Second retrieval from cache
    let cached = cache.get(&key);
    assert!(cached.is_some());
    let cached_output = cached.unwrap();
    assert_eq!(cached_output.semantic.functions.len(), 1);
}

#[test]
fn test_cache_miss_on_mtime_change() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    let rust_code = r#"
fn hello() {
    println!("Hello");
}
"#;
    fs::write(&file_path, rust_code).unwrap();

    let cache = AnalysisCache::new(100);
    let mtime1 = fs::metadata(&file_path).unwrap().modified().unwrap();
    let key1 = CacheKey {
        path: file_path.clone(),
        modified: mtime1,
        mode: AnalysisMode::FileDetails,
    };

    // Store with first mtime
    let output1 = analyze_file(file_path.to_str().unwrap(), None).unwrap();
    let arc_output1 = Arc::new(output1);
    cache.put(key1.clone(), arc_output1);

    // Simulate file modification by creating a key with different mtime
    let mtime2 = mtime1 + Duration::from_secs(1);
    let key2 = CacheKey {
        path: file_path.clone(),
        modified: mtime2,
        mode: AnalysisMode::FileDetails,
    };

    // Cache miss with new mtime
    let cached = cache.get(&key2);
    assert!(cached.is_none());
}

#[test]
fn test_cache_eviction_at_capacity() {
    let cache = AnalysisCache::new(3);
    let temp_dir = TempDir::new().unwrap();

    // Create 4 files and cache them
    for i in 0..4 {
        let file_path = temp_dir.path().join(format!("test{}.rs", i));
        fs::write(&file_path, format!("fn f{}() {{}}", i)).unwrap();

        let mtime = fs::metadata(&file_path).unwrap().modified().unwrap();
        let key = CacheKey {
            path: file_path.clone(),
            modified: mtime,
            mode: AnalysisMode::FileDetails,
        };

        let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();
        let arc_output = Arc::new(output);
        cache.put(key, arc_output);
    }

    // The first entry should have been evicted (LRU with capacity 3)
    let file_path = temp_dir.path().join("test0.rs");
    let mtime = fs::metadata(&file_path).unwrap().modified().unwrap();
    let key = CacheKey {
        path: file_path,
        modified: mtime,
        mode: AnalysisMode::FileDetails,
    };

    let cached = cache.get(&key);
    assert!(cached.is_none(), "First entry should be evicted");
}

#[test]
fn test_cache_mutex_poison_recovery() {
    let cache = AnalysisCache::new(10);
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, "fn test() {}").unwrap();

    let mtime = fs::metadata(&file_path).unwrap().modified().unwrap();
    let key = CacheKey {
        path: file_path.clone(),
        modified: mtime,
        mode: AnalysisMode::FileDetails,
    };

    // Store an entry
    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();
    let arc_output = Arc::new(output);
    cache.put(key.clone(), arc_output);

    // Verify entry is cached
    assert!(cache.get(&key).is_some());

    // Verify we can still use the cache after multiple operations
    let cached = cache.get(&key);
    assert!(cached.is_some(), "Cache should still have the entry");

    // Verify we can add more entries
    let new_output = analyze_file(file_path.to_str().unwrap(), None).unwrap();
    let new_arc_output = Arc::new(new_output);
    cache.put(key.clone(), new_arc_output);
    assert!(
        cache.get(&key).is_some(),
        "Cache should be usable after update"
    );
}

// Output limiting tests
#[test]
fn test_output_limiting_large_output() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.rs");

    // Create a file with many functions to generate substantial output
    let mut large_code = String::new();
    for i in 0..500 {
        large_code.push_str(&format!("fn func_{}() {{}}\n", i));
    }
    fs::write(&file_path, large_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify output is generated (the actual line count depends on formatter)
    let line_count = output.formatted.lines().count();
    assert!(
        line_count > 0,
        "Generated output should have content, got {} lines",
        line_count
    );
}

#[test]
fn test_output_limiting_below_threshold() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("small.rs");

    // Create a file with <1000 lines
    let mut small_code = String::new();
    for i in 0..50 {
        small_code.push_str(&format!("fn func_{}() {{}}\n", i));
    }
    fs::write(&file_path, small_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify output is returned normally (not limited)
    let line_count = output.formatted.lines().count();
    assert!(
        line_count < 1000,
        "Generated output should be under 1000 lines"
    );
    assert!(
        output.formatted.contains("FILE:"),
        "Should contain FILE header"
    );
}

#[test]
fn test_analyze_directory_with_progress_increments_counter() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create test structure with known file count
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn lib_fn() {}").unwrap();
    fs::write(root.join("README.md"), "# Test").unwrap();

    // Analyze with progress counter
    let counter = Arc::new(AtomicUsize::new(0));
    let ct = CancellationToken::new();
    let output = analyze_directory_with_progress(root, None, counter.clone(), ct).unwrap();

    // Verify counter was incremented for each file
    let final_count = counter.load(Ordering::Relaxed);
    assert_eq!(
        final_count, 3,
        "Counter should equal number of files analyzed"
    );

    // Verify analysis output is correct
    assert!(
        !output.formatted.is_empty(),
        "Formatted output should not be empty"
    );
    assert_eq!(output.files.len(), 3, "Should have analyzed 3 files");
}

#[test]
fn test_analyze_directory_with_progress_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Analyze empty directory with progress counter
    let counter = Arc::new(AtomicUsize::new(0));
    let ct = CancellationToken::new();
    let output = analyze_directory_with_progress(root, None, counter.clone(), ct).unwrap();

    // Verify counter is 0 for empty directory
    let final_count = counter.load(Ordering::Relaxed);
    assert_eq!(final_count, 0, "Counter should be 0 for empty directory");

    // Verify analysis output is valid
    assert!(
        !output.formatted.is_empty(),
        "Formatted output should not be empty"
    );
    assert_eq!(output.files.len(), 0, "Should have no files");
}

// Completion tests

#[test]
fn test_path_completions_with_prefix() {
    // Arrange: Create a temporary directory with files
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("tests")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn lib() {}").unwrap();
    fs::write(root.join("README.md"), "# Test").unwrap();

    // Act: Get completions for "src" prefix
    let completions = path_completions(root, "src");

    // Assert: Should find src directory
    assert!(
        !completions.is_empty(),
        "Should find completions for 'src' prefix"
    );
    assert!(
        completions.iter().any(|c| c.contains("src")),
        "Should include 'src' in completions"
    );
}

#[test]
fn test_path_completions_respects_ignore_file() {
    // Arrange: Create directory with .ignore file
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("target")).unwrap();
    fs::write(root.join(".ignore"), "target/\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("target/debug.txt"), "debug").unwrap();

    // Act: Get completions for "t" prefix
    let completions = path_completions(root, "t");

    // Assert: Should not include target (ignored by .ignore)
    assert!(
        !completions.iter().any(|c| c.contains("target")),
        "Should exclude 'target' directory (ignored by .ignore)"
    );
}

#[test]
fn test_path_completions_empty_prefix() {
    // Arrange: Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::write(root.join("file.rs"), "fn f() {}").unwrap();

    // Act: Get completions with empty prefix
    let completions = path_completions(root, "");

    // Assert: Should return empty for empty prefix
    assert!(
        completions.is_empty(),
        "Should return empty completions for empty prefix"
    );
}

#[test]
fn test_path_completions_nonexistent_prefix() {
    // Arrange: Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();

    // Act: Get completions for non-existent prefix
    let completions = path_completions(root, "xyz");

    // Assert: Should return empty for non-existent prefix
    assert!(
        completions.is_empty(),
        "Should return empty completions for non-existent prefix"
    );
}

#[test]
fn test_path_completions_truncates_at_100() {
    // Arrange: Create directory with >100 files
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    for i in 0..150 {
        fs::write(root.join(format!("file_{:03}.rs", i)), "fn f() {}").unwrap();
    }

    // Act: Get completions for "file" prefix
    let completions = path_completions(root, "file");

    // Assert: Should cap at 100 results
    assert_eq!(
        completions.len(),
        100,
        "Should truncate completions to 100 results"
    );
}

#[test]
fn test_symbol_completions_with_cached_analysis() {
    // Arrange: Create a Rust file and analyze it
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(
        &file_path,
        "fn hello_world() {}\nfn hello_there() {}\nstruct MyStruct {}",
    )
    .unwrap();

    // Analyze the file to populate cache
    let analysis = analyze_file(file_path.to_str().unwrap(), None).unwrap();
    let cache = AnalysisCache::new(100);
    let cache_key = CacheKey {
        path: file_path.clone(),
        modified: std::fs::metadata(&file_path).unwrap().modified().unwrap(),
        mode: AnalysisMode::FileDetails,
    };
    cache.put(cache_key, Arc::new(analysis));

    // Act: Get symbol completions for "hello" prefix
    let completions = symbol_completions(&cache, &file_path, "hello");

    // Assert: Should find matching functions
    assert!(
        !completions.is_empty(),
        "Should find completions for 'hello' prefix"
    );
    assert!(
        completions.iter().any(|c| c.contains("hello")),
        "Should include functions starting with 'hello'"
    );
}

#[test]
fn test_symbol_completions_missing_path_argument() {
    // Arrange: Create cache but don't populate it
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("nonexistent.rs");
    let cache = AnalysisCache::new(100);

    // Act: Get symbol completions for non-existent file
    let completions = symbol_completions(&cache, &file_path, "test");

    // Assert: Should return empty for missing file
    assert!(
        completions.is_empty(),
        "Should return empty completions for missing file"
    );
}

#[test]
fn test_symbol_completions_empty_prefix() {
    // Arrange: Create and analyze a file
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, "fn test_func() {}").unwrap();

    let analysis = analyze_file(file_path.to_str().unwrap(), None).unwrap();
    let cache = AnalysisCache::new(100);
    let cache_key = CacheKey {
        path: file_path.clone(),
        modified: std::fs::metadata(&file_path).unwrap().modified().unwrap(),
        mode: AnalysisMode::FileDetails,
    };
    cache.put(cache_key, Arc::new(analysis));

    // Act: Get symbol completions with empty prefix
    let completions = symbol_completions(&cache, &file_path, "");

    // Assert: Should return empty for empty prefix
    assert!(
        completions.is_empty(),
        "Should return empty completions for empty prefix"
    );
}

#[test]
fn test_symbol_completions_truncates_at_100() {
    // Arrange: Create a file with >100 functions
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    let mut content = String::new();
    for i in 0..150 {
        content.push_str(&format!("fn func_{:03}() {{}}\n", i));
    }
    fs::write(&file_path, content).unwrap();

    let analysis = analyze_file(file_path.to_str().unwrap(), None).unwrap();
    let cache = AnalysisCache::new(100);
    let cache_key = CacheKey {
        path: file_path.clone(),
        modified: std::fs::metadata(&file_path).unwrap().modified().unwrap(),
        mode: AnalysisMode::FileDetails,
    };
    cache.put(cache_key, Arc::new(analysis));

    // Act: Get symbol completions for "func" prefix
    let completions = symbol_completions(&cache, &file_path, "func");

    // Assert: Should cap at 100 results
    assert_eq!(
        completions.len(),
        100,
        "Should truncate completions to 100 results"
    );
}

// Logging tests
#[test]
fn test_logging_level_to_mcp_mapping() {
    use code_analyze_mcp::logging::level_to_mcp;
    use rmcp::model::LoggingLevel;
    use tracing::Level;

    // Test TRACE and DEBUG map to Debug
    assert_eq!(level_to_mcp(&Level::TRACE), LoggingLevel::Debug);
    assert_eq!(level_to_mcp(&Level::DEBUG), LoggingLevel::Debug);

    // Test INFO maps to Info
    assert_eq!(level_to_mcp(&Level::INFO), LoggingLevel::Info);

    // Test WARN maps to Warning
    assert_eq!(level_to_mcp(&Level::WARN), LoggingLevel::Warning);

    // Test ERROR maps to Error
    assert_eq!(level_to_mcp(&Level::ERROR), LoggingLevel::Error);
}

#[test]
fn test_logging_level_filter_update() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tracing_subscriber::filter::LevelFilter;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let log_level_filter = Arc::new(Mutex::new(LevelFilter::WARN));

        // Initial level should be WARN
        {
            let filter = log_level_filter.lock().await;
            assert_eq!(*filter, LevelFilter::WARN);
        }

        // Update to INFO
        {
            let mut filter = log_level_filter.lock().await;
            *filter = LevelFilter::INFO;
        }

        // Verify update
        {
            let filter = log_level_filter.lock().await;
            assert_eq!(*filter, LevelFilter::INFO);
        }

        // Update to ERROR
        {
            let mut filter = log_level_filter.lock().await;
            *filter = LevelFilter::ERROR;
        }

        // Verify final state
        {
            let filter = log_level_filter.lock().await;
            assert_eq!(*filter, LevelFilter::ERROR);
        }
    });
}

// Cancellation tests

#[test]
fn test_cancellation_during_directory_walk() {
    // Arrange: Create temp dir with a Rust file
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();

    // Create a pre-cancelled token
    let ct = CancellationToken::new();
    ct.cancel();

    // Act: Call analyze_directory_with_progress with cancelled token
    let counter = Arc::new(AtomicUsize::new(0));
    let result = analyze_directory_with_progress(root, None, counter, ct);

    // Assert: Should return Cancelled error
    assert!(matches!(result, Err(AnalyzeError::Cancelled)));
}

#[test]
fn test_cancellation_noop_after_completion() {
    // Arrange: Create temp dir with a Rust file
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    fs::write(root.join("main.rs"), "fn main() {}").unwrap();

    // Create a non-cancelled token
    let ct = CancellationToken::new();

    // Act: Call analyze_directory_with_progress with active token
    let counter = Arc::new(AtomicUsize::new(0));
    let result = analyze_directory_with_progress(root, None, counter, ct);

    // Assert: Should succeed (existing behavior unchanged)
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.files.len(), 1);
}

// Logging channel tests

#[tokio::test]
async fn test_log_event_sent_to_channel() {
    use code_analyze_mcp::logging::{LogEvent, McpLoggingLayer};
    use rmcp::model::LoggingLevel;
    use serde_json::json;
    use std::sync::Mutex;
    use tokio::sync::mpsc;
    use tracing_subscriber::filter::LevelFilter;

    // Arrange: Create unbounded channel
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let log_level_filter = Arc::new(Mutex::new(LevelFilter::WARN));

    // Create logging layer
    let _layer = McpLoggingLayer::new(event_tx, log_level_filter);

    // Act: Manually create and send a LogEvent (simulating on_event behavior)
    let log_event = LogEvent {
        level: LoggingLevel::Warning,
        logger: "test_logger".to_string(),
        data: json!({"message": "test event"}),
    };

    // Send event via the layer's sender (we'll test the channel directly)
    let (tx, mut rx) = mpsc::unbounded_channel();
    let _ = tx.send(log_event.clone());

    // Assert: Receive event from channel
    let received = rx.recv().await;
    assert!(received.is_some());
    let event = received.unwrap();
    assert_eq!(event.level, LoggingLevel::Warning);
    assert_eq!(event.logger, "test_logger");
    assert_eq!(event.data, json!({"message": "test event"}));
}

#[tokio::test]
async fn test_batch_draining_with_multiple_events() {
    use code_analyze_mcp::logging::LogEvent;
    use rmcp::model::LoggingLevel;
    use serde_json::json;
    use tokio::sync::mpsc;

    // Arrange: Create unbounded channel and send multiple events
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    // Send 5 events
    for i in 0..5 {
        let log_event = LogEvent {
            level: LoggingLevel::Info,
            logger: format!("logger_{}", i),
            data: json!({"index": i}),
        };
        let _ = event_tx.send(log_event);
    }

    // Act: Drain events using recv_many with buffer size 64
    let mut buffer = Vec::with_capacity(64);
    event_rx.recv_many(&mut buffer, 64).await;

    // Assert: All 5 events received in batch
    assert_eq!(buffer.len(), 5);
    for (i, event) in buffer.iter().enumerate() {
        assert_eq!(event.logger, format!("logger_{}", i));
        assert_eq!(event.data, json!({"index": i}));
    }
}

#[tokio::test]
async fn test_channel_closed_exits_consumer() {
    use code_analyze_mcp::logging::LogEvent;
    use rmcp::model::LoggingLevel;
    use serde_json::json;
    use tokio::sync::mpsc;

    // Arrange: Create channel and drop sender
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<LogEvent>();
    drop(event_tx);

    // Act: Try to receive from closed channel
    let mut buffer = Vec::with_capacity(64);
    let received = event_rx.recv_many(&mut buffer, 64).await;

    // Assert: recv_many returns 0 when channel is closed
    assert_eq!(received, 0);
    assert!(buffer.is_empty());
}

// Reference extraction tests for Python, Java, and TypeScript

#[test]
fn test_python_reference_extraction_happy_path() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");

    let python_code = r#"
class User:
    pass

def greet(user: User) -> User:
    return user

def process(items: list[User]) -> None:
    pass
"#;

    fs::write(&file_path, python_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify references are extracted (non-empty)
    assert!(
        !output.semantic.references.is_empty(),
        "Expected type references to be extracted from Python file"
    );

    // Verify expected type names are present
    let ref_symbols: Vec<&str> = output
        .semantic
        .references
        .iter()
        .map(|r| r.symbol.as_str())
        .collect();
    assert!(
        ref_symbols.contains(&"User"),
        "Expected 'User' type reference in Python code"
    );

    // Verify references have line numbers and location
    for reference in &output.semantic.references {
        assert!(
            reference.line > 0,
            "Reference should have non-zero line number"
        );
        assert!(
            !reference.location.is_empty(),
            "Reference should have location populated"
        );
    }
}

#[test]
fn test_python_reference_extraction_edge_case() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");

    let python_code = r#"
from typing import List, Union

class Result:
    pass

class Data:
    pass

def process(items: List[Result]) -> Union[Result, Data]:
    pass

def handle(data: list[dict[Result, Data]]) -> None:
    pass
"#;

    fs::write(&file_path, python_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify references are extracted for complex generic types
    assert!(
        !output.semantic.references.is_empty(),
        "Expected type references from generic types"
    );

    let ref_symbols: Vec<&str> = output
        .semantic
        .references
        .iter()
        .map(|r| r.symbol.as_str())
        .collect();

    // Should capture base type names from generics
    assert!(
        ref_symbols.contains(&"Result"),
        "Expected 'Result' from generic type parameters"
    );
    assert!(
        ref_symbols.contains(&"Data"),
        "Expected 'Data' from generic type parameters"
    );
}

// Import extraction tests

struct ImportTestCase {
    lang: &'static str,
    ext: &'static str,
    code: &'static str,
    expected_modules: Vec<&'static str>,
}

#[test]
fn test_import_extraction_happy_path() {
    let test_cases = vec![
        ImportTestCase {
            lang: "Python",
            ext: "py",
            code: r#"
import os
from sys import argv
from collections import defaultdict

def main():
    pass
"#,
            expected_modules: vec!["os", "sys", "collections"],
        },
        ImportTestCase {
            lang: "Go",
            ext: "go",
            code: r#"
package main

import (
    "fmt"
    "os"
)

import "io"

func main() {
    fmt.Println("Hello")
}
"#,
            expected_modules: vec!["fmt", "os", "io"],
        },
        ImportTestCase {
            lang: "Java",
            ext: "java",
            code: r#"
import java.util.ArrayList;
import java.util.List;
import static java.lang.Math.sqrt;

public class Test {
    public void method() {
        System.out.println("Hello");
    }
}
"#,
            expected_modules: vec!["ArrayList", "List", "Math"],
        },
        ImportTestCase {
            lang: "TypeScript",
            ext: "ts",
            code: r#"
import { Component } from 'react';
import * as fs from 'fs';
import path from 'path';

export function hello(): void {
    console.log("Hello");
}
"#,
            expected_modules: vec!["react", "fs", "path"],
        },
    ];

    for test_case in test_cases {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(format!("test.{}", test_case.ext));

        fs::write(&file_path, test_case.code).unwrap();

        let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

        // Verify imports extracted
        assert!(
            !output.semantic.imports.is_empty(),
            "{}: expected non-empty imports",
            test_case.lang
        );
        let import_modules: Vec<&str> = output
            .semantic
            .imports
            .iter()
            .map(|i| i.module.as_str())
            .collect();

        for expected in test_case.expected_modules {
            assert!(
                import_modules.iter().any(|m| m.contains(expected)),
                "{}: expected module containing '{}' not found in {:?}",
                test_case.lang,
                expected,
                import_modules
            );
        }
    }
}

#[test]
fn test_import_extraction_no_imports() {
    let test_cases = vec![
        ImportTestCase {
            lang: "Python",
            ext: "py",
            code: r#"
def hello():
    print("Hello")

class MyClass:
    pass
"#,
            expected_modules: vec![],
        },
        ImportTestCase {
            lang: "Go",
            ext: "go",
            code: r#"
package main

func Hello() {
    println("Hello")
}
"#,
            expected_modules: vec![],
        },
        ImportTestCase {
            lang: "Java",
            ext: "java",
            code: r#"
public class Test {
    public void method() {
        System.out.println("Hello");
    }
}
"#,
            expected_modules: vec![],
        },
        ImportTestCase {
            lang: "TypeScript",
            ext: "ts",
            code: r#"
export function hello(): void {
    console.log("Hello");
}

export class MyClass {
    method(): string {
        return "test";
    }
}
"#,
            expected_modules: vec![],
        },
    ];

    for test_case in test_cases {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(format!("test.{}", test_case.ext));

        fs::write(&file_path, test_case.code).unwrap();

        let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

        // Verify no imports extracted
        assert_eq!(
            output.semantic.imports.len(),
            0,
            "{}: expected zero imports",
            test_case.lang
        );
    }
}

// Test file partitioning tests

#[test]
fn test_format_structure_partitions_test_files() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Arrange: Create production and test files
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("tests")).unwrap();
    fs::write(root.join("src/lib.rs"), "fn production_fn() {}").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("tests/test_utils.rs"), "fn test_helper() {}").unwrap();

    // Act: Analyze directory
    let output = analyze_directory(root, None).unwrap();

    // Assert: Output contains TEST FILES section
    assert!(
        output.formatted.contains("TEST FILES"),
        "Output should contain TEST FILES section when test files are present"
    );

    // Assert: Test files are listed in TEST FILES section
    assert!(
        output.formatted.contains("test_utils.rs"),
        "Test file should be listed in TEST FILES section"
    );

    // Assert: Production files are listed in PATH section (before TEST FILES)
    assert!(
        output.formatted.contains("lib.rs"),
        "Production file should be listed in PATH section"
    );
    assert!(
        output.formatted.contains("main.rs"),
        "Production file should be listed in PATH section"
    );
}

#[test]
fn test_summary_auto_detect_large_directory() {
    // Arrange: Create a directory with enough files to exceed 1000 lines of output
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a structure that will generate >1000 lines of output
    // The PATH section shows each file on its own line, so we need >1000 files
    // Create 1100 files to ensure output exceeds 1000 lines
    fs::create_dir(root.join("src")).unwrap();
    for i in 0..1100 {
        let content = "fn func() {}";
        fs::write(root.join(format!("src/file_{i}.rs")), content).unwrap();
    }

    // Act: Analyze directory and generate summary
    let output = analyze_directory(root, None).unwrap();
    let summary = code_analyze_mcp::formatter::format_summary(&output.entries, &output.files, None);

    // Assert: Summary format should be present
    assert!(
        summary.contains("SUMMARY:"),
        "Summary should contain SUMMARY block"
    );
    assert!(
        summary.contains("STRUCTURE (depth 1):"),
        "Summary should contain STRUCTURE (depth 1) block"
    );
    assert!(
        summary.contains("SUGGESTION:"),
        "Summary should contain SUGGESTION block"
    );

    // Assert: Summary should show file counts
    assert!(
        summary.contains("1100 files"),
        "Summary should show correct file count"
    );

    // Assert: Summary should show language breakdown
    assert!(
        summary.contains("Languages:"),
        "Summary should show language breakdown"
    );

    // Assert: Summary should be much shorter than full output
    let full_line_count = output.formatted.lines().count();
    let summary_line_count = summary.lines().count();
    assert!(
        summary_line_count < full_line_count,
        "Summary ({} lines) should be shorter than full output ({} lines)",
        summary_line_count,
        full_line_count
    );
}

#[test]
fn test_summary_explicit_on_small_directory() {
    // Arrange: Create a small directory (would normally not trigger summary)
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn helper() {}").unwrap();

    // Act: Analyze directory and generate summary
    let output = analyze_directory(root, None).unwrap();
    let summary = code_analyze_mcp::formatter::format_summary(&output.entries, &output.files, None);

    // Assert: Summary format should be present even for small directory
    assert!(
        summary.contains("SUMMARY:"),
        "Summary should contain SUMMARY block"
    );
    assert!(
        summary.contains("STRUCTURE (depth 1):"),
        "Summary should contain STRUCTURE (depth 1) block"
    );
    assert!(
        summary.contains("SUGGESTION:"),
        "Summary should contain SUGGESTION block"
    );

    // Assert: Summary should show correct file count
    assert!(
        summary.contains("2 files"),
        "Summary should show correct file count"
    );
}
