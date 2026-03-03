mod fixtures;

use code_analyze_mcp::analyze::{analyze_directory, analyze_file, determine_mode};
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
fn test_javascript_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.js");

    let js_code = r#"
function hello() {
    console.log("Hello");
}

class MyClass {
    method() {
        return 42;
    }
}

const arrow = () => {
    return "arrow";
};
"#;

    fs::write(&file_path, js_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify functions extracted (hello, method)
    assert_eq!(output.semantic.functions.len(), 2);
    assert!(output.semantic.functions.iter().any(|f| f.name == "hello"));

    // Verify class extracted
    assert_eq!(output.semantic.classes.len(), 1);
    assert_eq!(output.semantic.classes[0].name, "MyClass");
}

#[test]
fn test_javascript_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.js");

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

#[test]
fn test_kotlin_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.kt");

    let kotlin_code = r#"
fun hello() {
    println("Hello")
}

class MyClass {
    fun method() {
        println("Method")
    }
}

object MySingleton {
    fun doSomething() {
        println("Singleton")
    }
}
"#;

    fs::write(&file_path, kotlin_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify functions extracted
    assert!(output.semantic.functions.iter().any(|f| f.name == "hello"));
    assert!(output.semantic.functions.iter().any(|f| f.name == "method"));
    assert!(
        output
            .semantic
            .functions
            .iter()
            .any(|f| f.name == "doSomething")
    );

    // Verify classes extracted
    assert!(output.semantic.classes.len() >= 2);
    let class_names: Vec<&str> = output
        .semantic
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(class_names.contains(&"MyClass"));
    assert!(class_names.contains(&"MySingleton"));
}

#[test]
fn test_kotlin_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.kt");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
}

#[test]
fn test_swift_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.swift");

    let swift_code = r#"
func hello() {
    print("Hello")
}

class MyClass {
    func method() {
        print("Method")
    }
}

struct MyStruct {
    var name: String
}

protocol MyProtocol {
    func doSomething()
}

enum MyEnum {
    case a
    case b
}
"#;

    fs::write(&file_path, swift_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify function extracted
    assert!(output.semantic.functions.iter().any(|f| f.name == "hello"));
    assert!(output.semantic.functions.iter().any(|f| f.name == "method"));

    // Verify types extracted
    assert!(output.semantic.classes.len() >= 4);
    let class_names: Vec<&str> = output
        .semantic
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(class_names.contains(&"MyClass"));
    assert!(class_names.contains(&"MyStruct"));
    assert!(class_names.contains(&"MyProtocol"));
    assert!(class_names.contains(&"MyEnum"));
}

#[test]
fn test_swift_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.swift");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
}

#[test]
fn test_ruby_parse_and_extract() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rb");

    let ruby_code = r#"
def hello
  puts "Hello"
end

class MyClass
  def method
    puts "Method"
  end
end

module MyModule
  def module_method
    puts "Module"
  end
end
"#;

    fs::write(&file_path, ruby_code).unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    // Verify methods extracted
    assert!(output.semantic.functions.iter().any(|f| f.name == "hello"));
    assert!(output.semantic.functions.iter().any(|f| f.name == "method"));
    assert!(
        output
            .semantic
            .functions
            .iter()
            .any(|f| f.name == "module_method")
    );

    // Verify classes/modules extracted
    assert!(output.semantic.classes.len() >= 2);
    let class_names: Vec<&str> = output
        .semantic
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(class_names.contains(&"MyClass"));
    assert!(class_names.contains(&"MyModule"));
}

#[test]
fn test_ruby_edge_case_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.rb");

    fs::write(&file_path, "").unwrap();

    let output = analyze_file(file_path.to_str().unwrap(), None).unwrap();

    assert_eq!(output.semantic.functions.len(), 0);
    assert_eq!(output.semantic.classes.len(), 0);
}
