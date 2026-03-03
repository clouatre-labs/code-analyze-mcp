use crate::languages::get_language_info;
use crate::types::{
    ClassInfo, FunctionInfo, ImportInfo, ReferenceInfo, ReferenceType, SemanticAnalysis,
};
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;
use tracing::instrument;
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

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

/// Recursively extract `ImportInfo` entries from a use-clause node, respecting all Rust
/// use-declaration forms (`scoped_identifier`, `scoped_use_list`, `use_list`,
/// `use_as_clause`, `use_wildcard`, bare `identifier`).
fn extract_imports_from_node(
    node: &Node,
    source: &str,
    prefix: &str,
    line: usize,
    imports: &mut Vec<ImportInfo>,
) {
    match node.kind() {
        // Simple identifier: `use foo;` or an item inside `{foo, bar}`
        "identifier" | "self" | "super" | "crate" => {
            let name = source[node.start_byte()..node.end_byte()].to_string();
            imports.push(ImportInfo {
                module: prefix.to_string(),
                items: vec![name],
                line,
            });
        }
        // Qualified path: `std::collections::HashMap`
        "scoped_identifier" => {
            let item = node
                .child_by_field_name("name")
                .map(|n| source[n.start_byte()..n.end_byte()].to_string())
                .unwrap_or_default();
            let module = node
                .child_by_field_name("path")
                .map(|p| {
                    let path_text = source[p.start_byte()..p.end_byte()].to_string();
                    if prefix.is_empty() {
                        path_text
                    } else {
                        format!("{}::{}", prefix, path_text)
                    }
                })
                .unwrap_or_else(|| prefix.to_string());
            if !item.is_empty() {
                imports.push(ImportInfo {
                    module,
                    items: vec![item],
                    line,
                });
            }
        }
        // `std::{io, fs}` — path prefix followed by a brace list
        "scoped_use_list" => {
            let new_prefix = node
                .child_by_field_name("path")
                .map(|p| {
                    let path_text = source[p.start_byte()..p.end_byte()].to_string();
                    if prefix.is_empty() {
                        path_text
                    } else {
                        format!("{}::{}", prefix, path_text)
                    }
                })
                .unwrap_or_else(|| prefix.to_string());
            if let Some(list) = node.child_by_field_name("list") {
                extract_imports_from_node(&list, source, &new_prefix, line, imports);
            }
        }
        // `{HashMap, HashSet}` — brace-enclosed list of items
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "{" | "}" | "," => {}
                    _ => extract_imports_from_node(&child, source, prefix, line, imports),
                }
            }
        }
        // `std::io::*` — glob import
        "use_wildcard" => {
            let text = source[node.start_byte()..node.end_byte()].to_string();
            let module = if let Some(stripped) = text.strip_suffix("::*") {
                if prefix.is_empty() {
                    stripped.to_string()
                } else {
                    format!("{}::{}", prefix, stripped)
                }
            } else {
                prefix.to_string()
            };
            imports.push(ImportInfo {
                module,
                items: vec!["*".to_string()],
                line,
            });
        }
        // `io as stdio` or `std::io as stdio`
        "use_as_clause" => {
            let alias = node
                .child_by_field_name("alias")
                .map(|n| source[n.start_byte()..n.end_byte()].to_string())
                .unwrap_or_default();
            let module = if let Some(path_node) = node.child_by_field_name("path") {
                match path_node.kind() {
                    "scoped_identifier" => path_node
                        .child_by_field_name("path")
                        .map(|p| {
                            let p_text = source[p.start_byte()..p.end_byte()].to_string();
                            if prefix.is_empty() {
                                p_text
                            } else {
                                format!("{}::{}", prefix, p_text)
                            }
                        })
                        .unwrap_or_else(|| prefix.to_string()),
                    _ => prefix.to_string(),
                }
            } else {
                prefix.to_string()
            };
            if !alias.is_empty() {
                imports.push(ImportInfo {
                    module,
                    items: vec![alias],
                    line,
                });
            }
        }
        _ => {}
    }
}

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

        // Validate and convert ast_recursion_limit once
        let max_depth: Option<u32> = ast_recursion_limit
            .map(|limit| {
                u32::try_from(limit).map_err(|_| {
                    ParserError::ParseError(format!(
                        "ast_recursion_limit {} exceeds maximum supported value {}",
                        limit,
                        u32::MAX
                    ))
                })
            })
            .transpose()?;

        // Extract functions and classes
        let element_query = Query::new(&lang_info.language, lang_info.element_query)
            .map_err(|e| ParserError::QueryError(e.to_string()))?;
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
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
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
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
        if let Some(import_query_str) = lang_info.import_query {
            let import_query = Query::new(&lang_info.language, import_query_str)
                .map_err(|e| ParserError::QueryError(e.to_string()))?;
            let mut cursor = QueryCursor::new();
            if let Some(depth) = max_depth {
                cursor.set_max_start_depth(Some(depth));
            }

            let mut matches = cursor.matches(&import_query, tree.root_node(), source.as_bytes());
            while let Some(mat) = matches.next() {
                for capture in mat.captures {
                    let capture_name = import_query.capture_names()[capture.index as usize];
                    if capture_name == "import_path" {
                        let node = capture.node;
                        let line = node.start_position().row + 1;
                        extract_imports_from_node(&node, source, "", line, &mut imports);
                    }
                }
            }
        }

        // Populate class methods from impl blocks
        if let Some(impl_query_str) = lang_info.impl_query {
            let impl_query = Query::new(&lang_info.language, impl_query_str)
                .map_err(|e| ParserError::QueryError(e.to_string()))?;
            let mut cursor = QueryCursor::new();
            if let Some(depth) = max_depth {
                cursor.set_max_start_depth(Some(depth));
            }

            let mut matches = cursor.matches(&impl_query, tree.root_node(), source.as_bytes());
            while let Some(mat) = matches.next() {
                let mut impl_type_name = String::new();
                let mut method_name = String::new();
                let mut method_line = 0usize;
                let mut method_end_line = 0usize;
                let mut method_params = String::new();
                let mut method_return_type: Option<String> = None;

                for capture in mat.captures {
                    let capture_name = impl_query.capture_names()[capture.index as usize];
                    let node = capture.node;
                    match capture_name {
                        "impl_type" => {
                            impl_type_name = source[node.start_byte()..node.end_byte()].to_string();
                        }
                        "method_name" => {
                            method_name = source[node.start_byte()..node.end_byte()].to_string();
                        }
                        "method_params" => {
                            method_params = source[node.start_byte()..node.end_byte()].to_string();
                        }
                        "method" => {
                            method_line = node.start_position().row + 1;
                            method_end_line = node.end_position().row + 1;
                            method_return_type = node
                                .child_by_field_name("return_type")
                                .map(|r| source[r.start_byte()..r.end_byte()].to_string());
                        }
                        _ => {}
                    }
                }

                if !impl_type_name.is_empty() && !method_name.is_empty() {
                    let func = FunctionInfo {
                        name: method_name,
                        line: method_line,
                        end_line: method_end_line,
                        parameters: if method_params.is_empty() {
                            Vec::new()
                        } else {
                            vec![method_params]
                        },
                        return_type: method_return_type,
                    };
                    if let Some(class) = classes.iter_mut().find(|c| c.name == impl_type_name) {
                        class.methods.push(func);
                    }
                }
            }
        }

        // Extract references with line numbers
        if let Some(ref_query_str) = lang_info.reference_query {
            let ref_query = Query::new(&lang_info.language, ref_query_str)
                .map_err(|e| ParserError::QueryError(e.to_string()))?;
            let mut cursor = QueryCursor::new();
            if let Some(depth) = max_depth {
                cursor.set_max_start_depth(Some(depth));
            }

            let mut seen_refs = std::collections::HashSet::new();
            let mut matches = cursor.matches(&ref_query, tree.root_node(), source.as_bytes());
            while let Some(mat) = matches.next() {
                for capture in mat.captures {
                    let capture_name = ref_query.capture_names()[capture.index as usize];
                    if capture_name == "type_ref" {
                        let node = capture.node;
                        let type_ref = source[node.start_byte()..node.end_byte()].to_string();
                        if seen_refs.insert(type_ref.clone()) {
                            references.push(ReferenceInfo {
                                symbol: type_ref,
                                reference_type: ReferenceType::Usage,
                                // location is intentionally empty here; set by the caller (analyze_file)
                                location: String::new(),
                                line: node.start_position().row + 1,
                            });
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
