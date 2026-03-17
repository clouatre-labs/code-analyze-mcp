//! Tree-sitter-based parser for extracting semantic structure from source code.
//!
//! This module provides language-agnostic parsing using tree-sitter queries to extract
//! functions, classes, imports, references, and other semantic elements from source files.
//! Two main extractors handle different use cases:
//!
//! - [`ElementExtractor`]: Quick extraction of function and class counts.
//! - [`SemanticExtractor`]: Detailed semantic analysis with calls, imports, and references.

use crate::languages::get_language_info;
use crate::types::{
    AssignmentInfo, CallInfo, ClassInfo, FieldAccessInfo, FunctionInfo, ImportInfo, ReferenceInfo,
    ReferenceType, SemanticAnalysis,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::LazyLock;
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

/// Compiled tree-sitter queries for a language.
/// Stores all query types: mandatory (element, call) and optional (import, impl, reference).
struct CompiledQueries {
    element: Query,
    call: Query,
    import: Option<Query>,
    impl_block: Option<Query>,
    reference: Option<Query>,
    assignment: Option<Query>,
    field: Option<Query>,
}

/// Build compiled queries for a given language.
fn build_compiled_queries(
    lang_info: &crate::languages::LanguageInfo,
) -> Result<CompiledQueries, ParserError> {
    let element = Query::new(&lang_info.language, lang_info.element_query).map_err(|e| {
        ParserError::QueryError(format!(
            "Failed to compile element query for {}: {}",
            lang_info.name, e
        ))
    })?;

    let call = Query::new(&lang_info.language, lang_info.call_query).map_err(|e| {
        ParserError::QueryError(format!(
            "Failed to compile call query for {}: {}",
            lang_info.name, e
        ))
    })?;

    let import = if let Some(import_query_str) = lang_info.import_query {
        Some(
            Query::new(&lang_info.language, import_query_str).map_err(|e| {
                ParserError::QueryError(format!(
                    "Failed to compile import query for {}: {}",
                    lang_info.name, e
                ))
            })?,
        )
    } else {
        None
    };

    let impl_block = if let Some(impl_query_str) = lang_info.impl_query {
        Some(
            Query::new(&lang_info.language, impl_query_str).map_err(|e| {
                ParserError::QueryError(format!(
                    "Failed to compile impl query for {}: {}",
                    lang_info.name, e
                ))
            })?,
        )
    } else {
        None
    };

    let reference = if let Some(ref_query_str) = lang_info.reference_query {
        Some(Query::new(&lang_info.language, ref_query_str).map_err(|e| {
            ParserError::QueryError(format!(
                "Failed to compile reference query for {}: {}",
                lang_info.name, e
            ))
        })?)
    } else {
        None
    };

    let assignment = if let Some(assignment_query_str) = lang_info.assignment_query {
        Some(
            Query::new(&lang_info.language, assignment_query_str).map_err(|e| {
                ParserError::QueryError(format!(
                    "Failed to compile assignment query for {}: {}",
                    lang_info.name, e
                ))
            })?,
        )
    } else {
        None
    };

    let field = if let Some(field_query_str) = lang_info.field_query {
        Some(
            Query::new(&lang_info.language, field_query_str).map_err(|e| {
                ParserError::QueryError(format!(
                    "Failed to compile field query for {}: {}",
                    lang_info.name, e
                ))
            })?,
        )
    } else {
        None
    };

    Ok(CompiledQueries {
        element,
        call,
        import,
        impl_block,
        reference,
        assignment,
        field,
    })
}

/// Initialize the query cache with compiled queries for all supported languages.
fn init_query_cache() -> HashMap<&'static str, CompiledQueries> {
    let supported_languages = ["rust", "python", "typescript", "tsx", "go", "java"];
    let mut cache = HashMap::new();

    for lang_name in &supported_languages {
        if let Some(lang_info) = get_language_info(lang_name) {
            match build_compiled_queries(&lang_info) {
                Ok(compiled) => {
                    cache.insert(*lang_name, compiled);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to compile queries for language {}: {}",
                        lang_name,
                        e
                    );
                }
            }
        }
    }

    cache
}

/// Lazily initialized cache of compiled queries per language.
static QUERY_CACHE: LazyLock<HashMap<&'static str, CompiledQueries>> =
    LazyLock::new(init_query_cache);

/// Get compiled queries for a language from the cache.
fn get_compiled_queries(language: &str) -> Result<&'static CompiledQueries, ParserError> {
    QUERY_CACHE
        .get(language)
        .ok_or_else(|| ParserError::UnsupportedLanguage(language.to_string()))
}

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new(Parser::new());
}

/// Canonical API for extracting element counts from source code.
pub struct ElementExtractor;

impl ElementExtractor {
    /// Extract function and class counts from source code.
    ///
    /// # Errors
    ///
    /// Returns `ParserError::UnsupportedLanguage` if the language is not recognized.
    /// Returns `ParserError::ParseError` if the source code cannot be parsed.
    /// Returns `ParserError::QueryError` if the tree-sitter query fails.
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

        let compiled = get_compiled_queries(language)?;

        let mut cursor = QueryCursor::new();
        let mut function_count = 0;
        let mut class_count = 0;

        let mut matches = cursor.matches(&compiled.element, tree.root_node(), source.as_bytes());
        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = compiled.element.capture_names()[capture.index as usize];
                match capture_name {
                    "function" => function_count += 1,
                    "class" => class_count += 1,
                    _ => {}
                }
            }
        }

        tracing::debug!(language = %language, functions = function_count, classes = class_count, "parse complete");

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
        // Python import_from_statement: `from module import name` or `from . import *`
        "import_from_statement" => {
            extract_python_import_from(node, source, line, imports);
        }
        // Fallback for non-Rust import nodes: capture full text as module
        _ => {
            let text = source[node.start_byte()..node.end_byte()]
                .trim()
                .to_string();
            if !text.is_empty() {
                imports.push(ImportInfo {
                    module: text,
                    items: vec![],
                    line,
                });
            }
        }
    }
}

/// Extract an item name from a dotted_name or aliased_import child node.
fn extract_import_item_name(child: &Node, source: &str) -> Option<String> {
    match child.kind() {
        "dotted_name" => {
            let name = source[child.start_byte()..child.end_byte()]
                .trim()
                .to_string();
            if name.is_empty() { None } else { Some(name) }
        }
        "aliased_import" => child.child_by_field_name("name").and_then(|n| {
            let name = source[n.start_byte()..n.end_byte()].trim().to_string();
            if name.is_empty() { None } else { Some(name) }
        }),
        _ => None,
    }
}

/// Collect wildcard/named imports from an import_list node or from direct named children.
fn collect_import_items(
    node: &Node,
    source: &str,
    is_wildcard: &mut bool,
    items: &mut Vec<String>,
) {
    // Prefer import_list child (wraps `from x import a, b`)
    if let Some(import_list) = node.child_by_field_name("import_list") {
        let mut cursor = import_list.walk();
        for child in import_list.named_children(&mut cursor) {
            if child.kind() == "wildcard_import" {
                *is_wildcard = true;
            } else if let Some(name) = extract_import_item_name(&child, source) {
                items.push(name);
            }
        }
        return;
    }
    // No import_list: single-name or wildcard as direct child (skip first named child = module_name)
    let mut cursor = node.walk();
    let mut first = true;
    for child in node.named_children(&mut cursor) {
        if first {
            first = false;
            continue;
        }
        if child.kind() == "wildcard_import" {
            *is_wildcard = true;
        } else if let Some(name) = extract_import_item_name(&child, source) {
            items.push(name);
        }
    }
}

/// Handle Python `import_from_statement` node.
fn extract_python_import_from(
    node: &Node,
    source: &str,
    line: usize,
    imports: &mut Vec<ImportInfo>,
) {
    let module = if let Some(m) = node.child_by_field_name("module_name") {
        source[m.start_byte()..m.end_byte()].trim().to_string()
    } else if let Some(r) = node.child_by_field_name("relative_import") {
        source[r.start_byte()..r.end_byte()].trim().to_string()
    } else {
        String::new()
    };

    let mut is_wildcard = false;
    let mut items = Vec::new();
    collect_import_items(node, source, &mut is_wildcard, &mut items);

    if !module.is_empty() {
        imports.push(ImportInfo {
            module,
            items: if is_wildcard {
                vec!["*".to_string()]
            } else {
                items
            },
            line,
        });
    }
}

pub struct SemanticExtractor;

impl SemanticExtractor {
    /// Extract semantic information from source code.
    ///
    /// # Errors
    ///
    /// Returns `ParserError::UnsupportedLanguage` if the language is not recognized.
    /// Returns `ParserError::ParseError` if the source code cannot be parsed.
    /// Returns `ParserError::QueryError` if the tree-sitter query fails.
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

        let compiled = get_compiled_queries(language)?;
        let root = tree.root_node();

        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();
        let mut references = Vec::new();
        let mut call_frequency = HashMap::new();
        let mut calls = Vec::new();
        let mut assignments: Vec<AssignmentInfo> = Vec::new();
        let mut field_accesses: Vec<FieldAccessInfo> = Vec::new();

        Self::extract_elements(
            source,
            compiled,
            root,
            max_depth,
            &lang_info,
            &mut functions,
            &mut classes,
        );
        Self::extract_calls(
            source,
            compiled,
            root,
            max_depth,
            &mut calls,
            &mut call_frequency,
        );
        Self::extract_imports(source, compiled, root, max_depth, &mut imports);
        Self::extract_impl_methods(source, compiled, root, max_depth, &mut classes);
        Self::extract_references(source, compiled, root, max_depth, &mut references);
        Self::extract_assignments(source, compiled, root, max_depth, &mut assignments);
        Self::extract_field_accesses(source, compiled, root, max_depth, &mut field_accesses);

        tracing::debug!(language = %language, functions = functions.len(), classes = classes.len(), imports = imports.len(), references = references.len(), calls = calls.len(), "extraction complete");

        Ok(SemanticAnalysis {
            functions,
            classes,
            imports,
            references,
            call_frequency,
            calls,
            assignments,
            field_accesses,
        })
    }

    fn extract_elements(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
        max_depth: Option<u32>,
        lang_info: &crate::languages::LanguageInfo,
        functions: &mut Vec<FunctionInfo>,
        classes: &mut Vec<ClassInfo>,
    ) {
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
        }
        let mut matches = cursor.matches(&compiled.element, root, source.as_bytes());
        let mut seen_functions = std::collections::HashSet::new();

        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = compiled.element.capture_names()[capture.index as usize];
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
                            let inherits = if let Some(handler) = lang_info.extract_inheritance {
                                handler(&node, source)
                            } else {
                                Vec::new()
                            };
                            classes.push(ClassInfo {
                                name,
                                line: node.start_position().row + 1,
                                end_line: node.end_position().row + 1,
                                methods: Vec::new(),
                                fields: Vec::new(),
                                inherits,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn extract_calls(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
        max_depth: Option<u32>,
        calls: &mut Vec<CallInfo>,
        call_frequency: &mut HashMap<String, usize>,
    ) {
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
        }
        let mut matches = cursor.matches(&compiled.call, root, source.as_bytes());

        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = compiled.call.capture_names()[capture.index as usize];
                if capture_name != "call" {
                    continue;
                }
                let node = capture.node;
                let call_name = source[node.start_byte()..node.end_byte()].to_string();
                *call_frequency.entry(call_name.clone()).or_insert(0) += 1;

                let mut current = node;
                let mut caller = "<module>".to_string();
                while let Some(parent) = current.parent() {
                    if parent.kind() == "function_item"
                        && let Some(name_node) = parent.child_by_field_name("name")
                    {
                        caller = source[name_node.start_byte()..name_node.end_byte()].to_string();
                        break;
                    }
                    current = parent;
                }

                let mut arg_count = None;
                let mut arg_node = node;
                while let Some(parent) = arg_node.parent() {
                    if parent.kind() == "call_expression" {
                        if let Some(args) = parent.child_by_field_name("arguments") {
                            arg_count = Some(args.named_child_count());
                        }
                        break;
                    }
                    arg_node = parent;
                }

                calls.push(CallInfo {
                    caller,
                    callee: call_name,
                    line: node.start_position().row + 1,
                    column: node.start_position().column,
                    arg_count,
                });
            }
        }
    }

    fn extract_imports(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
        max_depth: Option<u32>,
        imports: &mut Vec<ImportInfo>,
    ) {
        let Some(ref import_query) = compiled.import else {
            return;
        };
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
        }
        let mut matches = cursor.matches(import_query, root, source.as_bytes());

        while let Some(mat) = matches.next() {
            for capture in mat.captures {
                let capture_name = import_query.capture_names()[capture.index as usize];
                if capture_name == "import_path" {
                    let node = capture.node;
                    let line = node.start_position().row + 1;
                    extract_imports_from_node(&node, source, "", line, imports);
                }
            }
        }
    }

    fn extract_impl_methods(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
        max_depth: Option<u32>,
        classes: &mut [ClassInfo],
    ) {
        let Some(ref impl_query) = compiled.impl_block else {
            return;
        };
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
        }
        let mut matches = cursor.matches(impl_query, root, source.as_bytes());

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

    fn extract_references(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
        max_depth: Option<u32>,
        references: &mut Vec<ReferenceInfo>,
    ) {
        let Some(ref ref_query) = compiled.reference else {
            return;
        };
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
        }
        let mut seen_refs = std::collections::HashSet::new();
        let mut matches = cursor.matches(ref_query, root, source.as_bytes());

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

    fn extract_assignments(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
        max_depth: Option<u32>,
        assignments: &mut Vec<AssignmentInfo>,
    ) {
        let Some(ref assignment_query) = compiled.assignment else {
            return;
        };
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
        }
        let mut matches = cursor.matches(assignment_query, root, source.as_bytes());

        while let Some(mat) = matches.next() {
            let mut variable = String::new();
            let mut value = String::new();
            let mut line = 0usize;

            for capture in mat.captures {
                let capture_name = assignment_query.capture_names()[capture.index as usize];
                let node = capture.node;
                match capture_name {
                    "variable" => {
                        variable = source[node.start_byte()..node.end_byte()].to_string();
                    }
                    "value" => {
                        value = source[node.start_byte()..node.end_byte()].to_string();
                        line = node.start_position().row + 1;
                    }
                    _ => {}
                }
            }

            if !variable.is_empty() && !value.is_empty() {
                let mut current = mat.captures[0].node;
                let mut scope = "global".to_string();
                while let Some(parent) = current.parent() {
                    if parent.kind() == "function_item"
                        && let Some(name_node) = parent.child_by_field_name("name")
                    {
                        scope = source[name_node.start_byte()..name_node.end_byte()].to_string();
                        break;
                    }
                    current = parent;
                }
                assignments.push(AssignmentInfo {
                    variable,
                    value,
                    line,
                    scope,
                });
            }
        }
    }

    fn extract_field_accesses(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
        max_depth: Option<u32>,
        field_accesses: &mut Vec<FieldAccessInfo>,
    ) {
        let Some(ref field_query) = compiled.field else {
            return;
        };
        let mut cursor = QueryCursor::new();
        if let Some(depth) = max_depth {
            cursor.set_max_start_depth(Some(depth));
        }
        let mut matches = cursor.matches(field_query, root, source.as_bytes());

        while let Some(mat) = matches.next() {
            let mut object = String::new();
            let mut field = String::new();
            let mut line = 0usize;

            for capture in mat.captures {
                let capture_name = field_query.capture_names()[capture.index as usize];
                let node = capture.node;
                match capture_name {
                    "object" => {
                        object = source[node.start_byte()..node.end_byte()].to_string();
                    }
                    "field" => {
                        field = source[node.start_byte()..node.end_byte()].to_string();
                        line = node.start_position().row + 1;
                    }
                    _ => {}
                }
            }

            if !object.is_empty() && !field.is_empty() {
                let mut current = mat.captures[0].node;
                let mut scope = "global".to_string();
                while let Some(parent) = current.parent() {
                    if parent.kind() == "function_item"
                        && let Some(name_node) = parent.child_by_field_name("name")
                    {
                        scope = source[name_node.start_byte()..name_node.end_byte()].to_string();
                        break;
                    }
                    current = parent;
                }
                field_accesses.push(FieldAccessInfo {
                    object,
                    field,
                    line,
                    scope,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_assignments() {
        let source = r#"
fn main() {
    let x = 42;
    let y = x + 1;
}
"#;
        let result = SemanticExtractor::extract(source, "rust", None);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(!analysis.assignments.is_empty());
        assert_eq!(analysis.assignments[0].variable, "x");
        assert_eq!(analysis.assignments[0].value, "42");
        assert_eq!(analysis.assignments[0].scope, "main");
    }

    #[test]
    fn test_extract_field_accesses() {
        let source = r#"
fn process(user: &User) {
    let name = user.name;
    let age = user.age;
}
"#;
        let result = SemanticExtractor::extract(source, "rust", None);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(!analysis.field_accesses.is_empty());
        assert!(
            analysis
                .field_accesses
                .iter()
                .any(|fa| fa.object == "user" && fa.field == "name")
        );
        assert_eq!(analysis.field_accesses[0].scope, "process");
    }
}
