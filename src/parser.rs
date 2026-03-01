use crate::languages::get_language_info;
use crate::types::{ClassInfo, FunctionInfo, ImportInfo, SemanticAnalysis};
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;
use tracing::instrument;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("Failed to parse file: {0}")]
    ParseError(String),
    #[error("Invalid UTF-8 in file")]
    InvalidUtf8,
    #[error("Query error: {0}")]
    QueryError(String),
}

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new(Parser::new());
}

/// Canonical API for extracting element counts from source code.
pub struct ElementExtractor;

impl ElementExtractor {
    /// Extract function and class counts from source code.
    #[instrument(skip_all, fields(language))]
    pub fn extract_with_depth(source: &str, language: &str) -> Result<(usize, usize), ParserError> {
        let lang_info = get_language_info(language)
            .ok_or_else(|| ParserError::UnsupportedLanguage(language.to_string()))?;

        let tree = PARSER.with(|p| {
            let mut parser = p.borrow_mut();
            parser
                .set_language(&lang_info.language)
                .map_err(|e| ParserError::ParseError(format!("Failed to set language: {}", e)))?;
            parser
                .parse(source, None)
                .ok_or_else(|| ParserError::ParseError("Failed to parse".to_string()))
        })?;

        let query = Query::new(&lang_info.language, lang_info.element_query)
            .map_err(|e| ParserError::QueryError(e.to_string()))?;

        let mut cursor = QueryCursor::new();
        let mut function_count = 0;
        let mut class_count = 0;

        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = query.capture_names()[capture.index as usize];
                match capture_name {
                    "function" => function_count += 1,
                    "class" => class_count += 1,
                    _ => {}
                }
            }
        }

        Ok((function_count, class_count))
    }
}

/// Semantic extractor for file-level analysis.
pub struct SemanticExtractor;

impl SemanticExtractor {
    /// Extract semantic information from source code.
    #[instrument(skip_all, fields(language))]
    pub fn extract(
        source: &str,
        language: &str,
        ast_recursion_limit: Option<usize>,
    ) -> Result<SemanticAnalysis, ParserError> {
        let lang_info = get_language_info(language)
            .ok_or_else(|| ParserError::UnsupportedLanguage(language.to_string()))?;

        let tree = PARSER.with(|p| {
            let mut parser = p.borrow_mut();
            parser
                .set_language(&lang_info.language)
                .map_err(|e| ParserError::ParseError(format!("Failed to set language: {}", e)))?;
            parser
                .parse(source, None)
                .ok_or_else(|| ParserError::ParseError("Failed to parse".to_string()))
        })?;

        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();
        let mut references = Vec::new();
        let mut call_frequency = HashMap::new();

        // Extract functions and classes
        let element_query = Query::new(&lang_info.language, lang_info.element_query)
            .map_err(|e| ParserError::QueryError(e.to_string()))?;
        let mut cursor = QueryCursor::new();
        if let Some(limit) = ast_recursion_limit {
            cursor.set_max_start_depth(Some(limit as u32));
        }

        let mut matches = cursor.matches(&element_query, tree.root_node(), source.as_bytes());
        let mut seen_functions = std::collections::HashSet::new();

        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = element_query.capture_names()[capture.index as usize];
                let node = capture.node;

                match capture_name {
                    "function" => {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            let name =
                                source[name_node.start_byte()..name_node.end_byte()].to_string();
                            let func_key = (name.clone(), node.start_position().row);

                            if !seen_functions.contains(&func_key) {
                                seen_functions.insert(func_key);

                                let params = node
                                    .child_by_field_name("parameters")
                                    .map(|p| source[p.start_byte()..p.end_byte()].to_string())
                                    .unwrap_or_default();
                                let return_type = node
                                    .child_by_field_name("return_type")
                                    .map(|r| source[r.start_byte()..r.end_byte()].to_string());

                                functions.push(FunctionInfo {
                                    name,
                                    line: node.start_position().row + 1,
                                    end_line: node.end_position().row + 1,
                                    parameters: if params.is_empty() {
                                        Vec::new()
                                    } else {
                                        vec![params]
                                    },
                                    return_type,
                                });
                            }
                        }
                    }
                    "class" => {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            let name =
                                source[name_node.start_byte()..name_node.end_byte()].to_string();
                            classes.push(ClassInfo {
                                name,
                                line: node.start_position().row + 1,
                                end_line: node.end_position().row + 1,
                                methods: Vec::new(),
                                fields: Vec::new(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        // Extract calls
        let call_query = Query::new(&lang_info.language, lang_info.call_query)
            .map_err(|e| ParserError::QueryError(e.to_string()))?;
        let mut cursor = QueryCursor::new();
        if let Some(limit) = ast_recursion_limit {
            cursor.set_max_start_depth(Some(limit as u32));
        }

        let mut matches = cursor.matches(&call_query, tree.root_node(), source.as_bytes());
        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = call_query.capture_names()[capture.index as usize];
                if capture_name == "call" {
                    let node = capture.node;
                    let call_name = source[node.start_byte()..node.end_byte()].to_string();
                    *call_frequency.entry(call_name).or_insert(0) += 1;
                }
            }
        }

        // Extract imports
        let import_query = Query::new(&lang_info.language, crate::languages::rust::IMPORT_QUERY)
            .map_err(|e| ParserError::QueryError(e.to_string()))?;
        let mut cursor = QueryCursor::new();
        if let Some(limit) = ast_recursion_limit {
            cursor.set_max_start_depth(Some(limit as u32));
        }

        let mut matches = cursor.matches(&import_query, tree.root_node(), source.as_bytes());
        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = import_query.capture_names()[capture.index as usize];
                if capture_name == "import_path" {
                    let node = capture.node;
                    let import_path = source[node.start_byte()..node.end_byte()].to_string();
                    let parts: Vec<&str> = import_path.split("::").collect();
                    let module = parts.first().map(|s| s.to_string()).unwrap_or_default();
                    let items = parts.iter().map(|s| s.to_string()).collect();

                    imports.push(ImportInfo {
                        module,
                        items,
                        line: node.start_position().row + 1,
                    });
                }
            }
        }

        // Extract references if available
        if let Some(ref_query_str) = lang_info.reference_query {
            let ref_query = Query::new(&lang_info.language, ref_query_str)
                .map_err(|e| ParserError::QueryError(e.to_string()))?;
            let mut cursor = QueryCursor::new();
            if let Some(limit) = ast_recursion_limit {
                cursor.set_max_start_depth(Some(limit as u32));
            }

            let mut matches = cursor.matches(&ref_query, tree.root_node(), source.as_bytes());
            while let Some(mat) = matches.next() {
                for capture in mat.captures {
                    let capture_name = ref_query.capture_names()[capture.index as usize];
                    if capture_name == "type_ref" {
                        let node = capture.node;
                        let type_ref = source[node.start_byte()..node.end_byte()].to_string();
                        if !references.contains(&type_ref) {
                            references.push(type_ref);
                        }
                    }
                }
            }
        }

        Ok(SemanticAnalysis {
            functions,
            classes,
            imports,
            references,
            call_frequency,
        })
    }
}
