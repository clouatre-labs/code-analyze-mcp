use std::cell::RefCell;
use std::collections::HashMap;
use tree_sitter::{Parser, Query, QueryCursor};

use crate::languages::LanguageInfo;

thread_local! {
    static PARSERS: RefCell<HashMap<&'static str, Parser>> = RefCell::new(HashMap::new());
}

pub struct FileMetrics {
    pub line_count: usize,
    pub function_count: usize,
    pub class_count: usize,
}

pub struct ParserManager;

impl ParserManager {
    /// Runs `f` with a thread-local parser pre-configured for `language_info`.
    pub fn with_parser<F, R>(language_info: &'static LanguageInfo, f: F) -> R
    where
        F: FnOnce(&mut Parser) -> R,
    {
        PARSERS.with(|cell| {
            let mut map = cell.borrow_mut();
            let parser = map.entry(language_info.name).or_insert_with(|| {
                let mut p = Parser::new();
                let lang = (language_info.tree_sitter_language)();
                p.set_language(&lang).expect("Failed to configure tree-sitter language");
                p
            });
            f(parser)
        })
    }
}

pub struct ElementExtractor;

impl ElementExtractor {
    /// Parse `source` and return LOC, function count, and class count (structure-level only).
    pub fn extract_with_depth(source: &str, language_info: &'static LanguageInfo) -> FileMetrics {
        let line_count = source.lines().count();

        let language = (language_info.tree_sitter_language)();

        let tree = ParserManager::with_parser(language_info, |parser| {
            parser.parse(source, None)
        });

        let tree = match tree {
            Some(t) => t,
            None => {
                tracing::warn!(
                    language = language_info.name,
                    "tree-sitter parse returned None; reporting zero counts"
                );
                return FileMetrics {
                    line_count,
                    function_count: 0,
                    class_count: 0,
                }
            }
        };

        let query = match Query::new(&language, language_info.element_query) {
            Ok(q) => q,
            Err(err) => {
                tracing::warn!(
                    language = language_info.name,
                    error = %err,
                    "Failed to compile tree-sitter query; reporting zero counts"
                );
                return FileMetrics {
                    line_count,
                    function_count: 0,
                    class_count: 0,
                }
            }
        };

        let mut cursor = QueryCursor::new();
        let source_bytes = source.as_bytes();
        let matches = cursor.matches(&query, tree.root_node(), source_bytes);

        let mut function_count = 0usize;
        let mut class_count = 0usize;

        for m in matches {
            for capture in m.captures {
                match query.capture_names()[capture.index as usize] {
                    "function" => function_count += 1,
                    "class" => class_count += 1,
                    _ => {}
                }
            }
        }

        FileMetrics {
            line_count,
            function_count,
            class_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages;

    #[test]
    fn test_extract_rust_functions_and_structs() {
        let source = r#"
struct Foo { x: i32 }
enum Bar { A, B }
trait Baz {}
fn alpha() {}
fn beta() {}
impl Foo {
    fn gamma(&self) {}
}
"#;
        let info = languages::get_language_info("rust").unwrap();
        let metrics = ElementExtractor::extract_with_depth(source, info);
        assert_eq!(metrics.function_count, 3); // alpha, beta, gamma
        assert_eq!(metrics.class_count, 3); // Foo, Bar, Baz
    }

    #[test]
    fn test_extract_empty_source() {
        let info = languages::get_language_info("rust").unwrap();
        let metrics = ElementExtractor::extract_with_depth("", info);
        assert_eq!(metrics.line_count, 0);
        assert_eq!(metrics.function_count, 0);
        assert_eq!(metrics.class_count, 0);
    }

    #[test]
    fn test_extract_line_count() {
        let source = "fn a() {}\nfn b() {}\nfn c() {}\n";
        let info = languages::get_language_info("rust").unwrap();
        let metrics = ElementExtractor::extract_with_depth(source, info);
        assert_eq!(metrics.line_count, 3);
        assert_eq!(metrics.function_count, 3);
    }
}
