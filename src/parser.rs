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
    CallInfo, ClassInfo, FunctionInfo, ImplTraitInfo, ImportInfo, ReferenceInfo, ReferenceType,
    SemanticAnalysis,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    impl_trait: Option<Query>,
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

    let impl_trait = if let Some(impl_trait_query_str) = lang_info.impl_trait_query {
        Some(
            Query::new(&lang_info.language, impl_trait_query_str).map_err(|e| {
                ParserError::QueryError(format!(
                    "Failed to compile impl_trait query for {}: {}",
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
        impl_trait,
    })
}

/// Initialize the query cache with compiled queries for all supported languages.
fn init_query_cache() -> HashMap<&'static str, CompiledQueries> {
    let supported_languages = [
        "rust",
        "python",
        "typescript",
        "tsx",
        "go",
        "java",
        "fortran",
    ];
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

        // 0 is not a useful depth (visits root node only, returning zero results).
        // Treat 0 as None (unlimited). See #339.
        let max_depth: Option<u32> = ast_recursion_limit
            .filter(|&limit| limit > 0)
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

        // Extract impl-trait blocks for Rust files (empty for other languages)
        let impl_traits = if language == "rust" {
            Self::extract_impl_traits_from_tree(source, compiled, root)
        } else {
            vec![]
        };

        tracing::debug!(language = %language, functions = functions.len(), classes = classes.len(), imports = imports.len(), references = references.len(), calls = calls.len(), impl_traits = impl_traits.len(), "extraction complete");

        Ok(SemanticAnalysis {
            functions,
            classes,
            imports,
            references,
            call_frequency,
            calls,
            impl_traits,
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

    /// Returns the name of the enclosing function/method/subroutine for a given AST node,
    /// by walking ancestors and matching all language-specific function container kinds.
    fn enclosing_function_name(mut node: tree_sitter::Node<'_>, source: &str) -> Option<String> {
        let mut depth = 0u32;
        while let Some(parent) = node.parent() {
            depth += 1;
            // Cap at 64 hops: real function nesting rarely exceeds ~10 levels; 64 is a generous
            // upper bound that guards against pathological/malformed ASTs without false negatives
            // on legitimate code. Returns None (treated as <module>) when the cap is hit.
            if depth > 64 {
                return None;
            }
            let name_node = match parent.kind() {
                // Direct name field: Rust, Python, Go, Java, TypeScript/TSX
                "function_item"
                | "method_item"
                | "function_definition"
                | "function_declaration"
                | "method_declaration"
                | "method_definition" => parent.child_by_field_name("name"),
                // Fortran subroutine: name is inside subroutine_statement child
                "subroutine" => {
                    let mut cursor = parent.walk();
                    parent
                        .children(&mut cursor)
                        .find(|c| c.kind() == "subroutine_statement")
                        .and_then(|s| s.child_by_field_name("name"))
                }
                // Fortran function: name is inside function_statement child
                "function" => {
                    let mut cursor = parent.walk();
                    parent
                        .children(&mut cursor)
                        .find(|c| c.kind() == "function_statement")
                        .and_then(|s| s.child_by_field_name("name"))
                }
                _ => {
                    node = parent;
                    continue;
                }
            };
            return name_node.map(|n| source[n.start_byte()..n.end_byte()].to_string());
        }
        // The loop exits here only when no parent was found (i.e., we reached the tree root
        // without finding a function container). If the depth cap fired, we returned None early
        // above. Nothing to assert here.
        None
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

                let caller = Self::enclosing_function_name(node, source)
                    .unwrap_or_else(|| "<module>".to_string());

                let mut arg_count = None;
                let mut arg_node = node;
                let mut hop = 0u32;
                let mut cap_hit = false;
                while let Some(parent) = arg_node.parent() {
                    hop += 1;
                    // Bounded parent traversal: cap at 16 hops to guard against pathological
                    // walks on malformed/degenerate trees. Real call-expression nesting is
                    // shallow (typically 1-3 levels). When the cap is hit we stop searching and
                    // leave arg_count as None; the caller is still recorded, just without
                    // argument-count information.
                    if hop > 16 {
                        cap_hit = true;
                        break;
                    }
                    if parent.kind() == "call_expression" {
                        if let Some(args) = parent.child_by_field_name("arguments") {
                            arg_count = Some(args.named_child_count());
                        }
                        break;
                    }
                    arg_node = parent;
                }
                debug_assert!(
                    !cap_hit,
                    "extract_calls: parent traversal cap reached (hop > 16)"
                );

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

    /// Extract impl-trait blocks from an already-parsed tree.
    ///
    /// Called during `extract()` for Rust files to avoid a second parse.
    /// Returns an empty vec if the query is not available.
    fn extract_impl_traits_from_tree(
        source: &str,
        compiled: &CompiledQueries,
        root: Node<'_>,
    ) -> Vec<ImplTraitInfo> {
        let Some(query) = &compiled.impl_trait else {
            return vec![];
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root, source.as_bytes());
        let mut results = Vec::new();

        while let Some(mat) = matches.next() {
            let mut trait_name = String::new();
            let mut impl_type = String::new();
            let mut line = 0usize;

            for capture in mat.captures {
                let capture_name = query.capture_names()[capture.index as usize];
                let node = capture.node;
                let text = source[node.start_byte()..node.end_byte()].to_string();
                match capture_name {
                    "trait_name" => {
                        trait_name = text;
                        line = node.start_position().row + 1;
                    }
                    "impl_type" => {
                        impl_type = text;
                    }
                    _ => {}
                }
            }

            if !trait_name.is_empty() && !impl_type.is_empty() {
                results.push(ImplTraitInfo {
                    trait_name,
                    impl_type,
                    path: PathBuf::new(), // Path will be set by caller
                    line,
                });
            }
        }

        results
    }
}

/// Extract `impl Trait for Type` blocks from Rust source.
///
/// Runs independently of `extract_references` to avoid shared deduplication state.
/// Returns an empty vec for non-Rust source (no error; caller decides).
pub fn extract_impl_traits(source: &str, path: &Path) -> Vec<ImplTraitInfo> {
    let lang_info = match get_language_info("rust") {
        Some(info) => info,
        None => return vec![],
    };

    let compiled = match get_compiled_queries("rust") {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let query = match &compiled.impl_trait {
        Some(q) => q,
        None => return vec![],
    };

    let tree = match PARSER.with(|p| {
        let mut parser = p.borrow_mut();
        let _ = parser.set_language(&lang_info.language);
        parser.parse(source, None)
    }) {
        Some(t) => t,
        None => return vec![],
    };

    let root = tree.root_node();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source.as_bytes());
    let mut results = Vec::new();

    while let Some(mat) = matches.next() {
        let mut trait_name = String::new();
        let mut impl_type = String::new();
        let mut line = 0usize;

        for capture in mat.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            let node = capture.node;
            let text = source[node.start_byte()..node.end_byte()].to_string();
            match capture_name {
                "trait_name" => {
                    trait_name = text;
                    line = node.start_position().row + 1;
                }
                "impl_type" => {
                    impl_type = text;
                }
                _ => {}
            }
        }

        if !trait_name.is_empty() && !impl_type.is_empty() {
            results.push(ImplTraitInfo {
                trait_name,
                impl_type,
                path: path.to_path_buf(),
                line,
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_recursion_limit_zero_is_unlimited() {
        let source = r#"fn hello() -> u32 { 42 }"#;
        let result_none = SemanticExtractor::extract(source, "rust", None);
        let result_zero = SemanticExtractor::extract(source, "rust", Some(0));
        assert!(result_none.is_ok(), "extract with None failed");
        assert!(result_zero.is_ok(), "extract with Some(0) failed");
        let analysis_none = result_none.unwrap();
        let analysis_zero = result_zero.unwrap();
        assert!(
            analysis_none.functions.len() >= 1,
            "extract with None should find at least one function in the test source"
        );
        assert_eq!(
            analysis_none.functions.len(),
            analysis_zero.functions.len(),
            "ast_recursion_limit=0 should behave identically to unset (unlimited)"
        );
    }
}
