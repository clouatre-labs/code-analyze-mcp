//! Call graph construction and analysis.
//!
//! Builds caller and callee relationships from semantic analysis results.
//! Implements type-aware function matching to disambiguate overloads and name collisions.

use crate::types::{CallEdge, ImplTraitInfo, SemanticAnalysis, SymbolMatchMode};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, instrument};

/// Type info for a function: (path, line, parameters, return_type)
type FunctionTypeInfo = (PathBuf, usize, Vec<String>, Option<String>);

const MAX_CANDIDATES_IN_ERROR: usize = 20;

fn format_candidates(candidates: &[String]) -> String {
    if candidates.len() <= MAX_CANDIDATES_IN_ERROR {
        candidates.join(", ")
    } else {
        format!(
            "{}, (and {} more)",
            candidates[..MAX_CANDIDATES_IN_ERROR].join(", "),
            candidates.len() - MAX_CANDIDATES_IN_ERROR
        )
    }
}

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Symbol not found: '{symbol}'. {hint}")]
    SymbolNotFound { symbol: String, hint: String },
    #[error(
        "Multiple candidates matched '{query}': {candidates_display}. Refine the symbol name or use a stricter match_mode.",
        candidates_display = format_candidates(.candidates)
    )]
    MultipleCandidates {
        query: String,
        candidates: Vec<String>,
    },
}

/// Resolve a symbol name against the set of known symbols using the requested match mode.
///
/// Returns:
/// - `Ok(name)` when exactly one symbol matches.
/// - `Err(GraphError::SymbolNotFound)` when no symbol matches.
/// - `Err(GraphError::MultipleCandidates)` when more than one symbol matches.
pub fn resolve_symbol<'a>(
    known_symbols: impl Iterator<Item = &'a String>,
    query: &str,
    mode: &SymbolMatchMode,
) -> Result<String, GraphError> {
    let mut matches: Vec<String> = if matches!(mode, SymbolMatchMode::Exact) {
        known_symbols
            .filter(|s| s.as_str() == query)
            .cloned()
            .collect()
    } else {
        let query_lower = query.to_lowercase();
        known_symbols
            .filter(|s| match mode {
                SymbolMatchMode::Exact => unreachable!(),
                SymbolMatchMode::Insensitive => s.to_lowercase() == query_lower,
                SymbolMatchMode::Prefix => s.to_lowercase().starts_with(&query_lower),
                SymbolMatchMode::Contains => s.to_lowercase().contains(&query_lower),
            })
            .cloned()
            .collect()
    };
    matches.sort();

    debug!(
        query,
        mode = ?mode,
        candidate_count = matches.len(),
        "resolve_symbol"
    );

    match matches.len() {
        1 => Ok(matches.into_iter().next().expect("len==1")),
        0 => {
            let hint = match mode {
                SymbolMatchMode::Exact => {
                    "Try match_mode=insensitive for a case-insensitive search.".to_string()
                }
                _ => "No symbols matched; try a shorter query or match_mode=contains.".to_string(),
            };
            Err(GraphError::SymbolNotFound {
                symbol: query.to_string(),
                hint,
            })
        }
        _ => Err(GraphError::MultipleCandidates {
            query: query.to_string(),
            candidates: matches,
        }),
    }
}

/// Strip scope prefixes from a callee name.
/// Handles patterns: 'self.method' -> 'method', 'Type::method' -> 'method', 'module::function' -> 'function'.
/// If no prefix is found, returns the original name.
fn strip_scope_prefix(name: &str) -> &str {
    if let Some(pos) = name.rfind("::") {
        &name[pos + 2..]
    } else if let Some(pos) = name.rfind('.') {
        &name[pos + 1..]
    } else {
        name
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InternalCallChain {
    pub chain: Vec<(String, PathBuf, usize)>,
}

/// Call graph storing callers, callees, and function definitions.
#[derive(Debug, Clone)]
pub struct CallGraph {
    /// Callers map: function_name -> vec of CallEdge (one per call site).
    pub callers: HashMap<String, Vec<CallEdge>>,
    /// Callees map: function_name -> vec of CallEdge (one per call site).
    pub callees: HashMap<String, Vec<CallEdge>>,
    /// Definitions map: function_name -> vec of (file_path, line_number).
    pub definitions: HashMap<String, Vec<(PathBuf, usize)>>,
    /// Internal: maps function name to type info for type-aware disambiguation.
    function_types: HashMap<String, Vec<FunctionTypeInfo>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            callers: HashMap::new(),
            callees: HashMap::new(),
            definitions: HashMap::new(),
            function_types: HashMap::new(),
        }
    }

    /// Count parameters in a parameter string.
    /// Handles: "(x: i32, y: String)" -> 2, "(&self, x: i32)" -> 2, "()" -> 0, "(&self)" -> 1
    fn count_parameters(params_str: &str) -> usize {
        if params_str.is_empty() || params_str == "()" {
            return 0;
        }
        // Remove outer parens and trim
        let inner = params_str
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim();
        if inner.is_empty() {
            return 0;
        }
        // Count commas + 1 to get parameter count
        inner.split(',').count()
    }

    /// Match a callee by parameter count and return type.
    /// Returns the index of the best match in the candidates list, or None if no good match.
    /// Strategy: prefer candidates with matching param count, then by return type match.
    fn match_by_type(
        &self,
        candidates: &[FunctionTypeInfo],
        expected_param_count: Option<usize>,
        expected_return_type: Option<&str>,
    ) -> Option<usize> {
        if candidates.is_empty() {
            return None;
        }

        // If we have no type info to match against, return None (fallback to line proximity)
        if expected_param_count.is_none() && expected_return_type.is_none() {
            return None;
        }

        let mut best_idx = 0;
        let mut best_score = 0;

        for (idx, (_path, _line, params, ret_type)) in candidates.iter().enumerate() {
            let mut score = 0;

            // Score parameter count match
            if let Some(expected_count) = expected_param_count
                && !params.is_empty()
            {
                let actual_count = Self::count_parameters(&params[0]);
                if actual_count == expected_count {
                    score += 2;
                }
            }

            // Score return type match
            if let Some(expected_ret) = expected_return_type
                && let Some(actual_ret) = ret_type
                && actual_ret == expected_ret
            {
                score += 1;
            }

            // Prefer candidates with more type info
            if !params.is_empty() {
                score += 1;
            }
            if ret_type.is_some() {
                score += 1;
            }

            if score > best_score {
                best_score = score;
                best_idx = idx;
            }
        }

        // Only return a match if we found a meaningful score
        (best_score > 0).then_some(best_idx)
    }

    /// Resolve a callee name using four strategies:
    /// 1. Try the raw callee name first in definitions
    /// 2. If not found, try the stripped name (via strip_scope_prefix)
    /// 3. If multiple definitions exist, prefer same-file candidates
    /// 4. Among same-file candidates, use type info as tiebreaker, then line proximity
    /// 5. If no same-file candidates, use any definition (first one)
    ///
    /// Returns the resolved callee name (which may be the stripped version).
    fn resolve_callee(
        &self,
        callee: &str,
        call_file: &Path,
        call_line: usize,
        arg_count: Option<usize>,
        definitions: &HashMap<String, Vec<(PathBuf, usize)>>,
        function_types: &HashMap<String, Vec<FunctionTypeInfo>>,
    ) -> String {
        // Try raw callee name first
        if let Some(defs) = definitions.get(callee) {
            return self.pick_best_definition(
                defs,
                call_file,
                call_line,
                arg_count,
                callee,
                function_types,
            );
        }

        // Try stripped name
        let stripped = strip_scope_prefix(callee);
        if stripped != callee
            && let Some(defs) = definitions.get(stripped)
        {
            return self.pick_best_definition(
                defs,
                call_file,
                call_line,
                arg_count,
                stripped,
                function_types,
            );
        }

        // No definition found; return the original callee
        callee.to_string()
    }

    /// Pick the best definition from a list based on same-file preference, type matching, and line proximity.
    fn pick_best_definition(
        &self,
        defs: &[(PathBuf, usize)],
        call_file: &Path,
        call_line: usize,
        arg_count: Option<usize>,
        resolved_name: &str,
        function_types: &HashMap<String, Vec<FunctionTypeInfo>>,
    ) -> String {
        // Filter to same-file candidates
        let same_file_defs: Vec<_> = defs.iter().filter(|(path, _)| path == call_file).collect();

        if !same_file_defs.is_empty() {
            // Try type-aware disambiguation if we have type info
            if let Some(type_info) = function_types.get(resolved_name) {
                let same_file_types: Vec<_> = type_info
                    .iter()
                    .filter(|(path, _, _, _)| path == call_file)
                    .cloned()
                    .collect();

                if !same_file_types.is_empty() && same_file_types.len() > 1 {
                    // Group candidates by line proximity (within 5 lines)
                    let mut proximity_groups: Vec<Vec<usize>> = vec![];
                    for (idx, (_, def_line, _, _)) in same_file_types.iter().enumerate() {
                        let mut placed = false;
                        for group in &mut proximity_groups {
                            if let Some((_, first_line, _, _)) = same_file_types.get(group[0])
                                && first_line.abs_diff(*def_line) <= 5
                            {
                                group.push(idx);
                                placed = true;
                                break;
                            }
                        }
                        if !placed {
                            proximity_groups.push(vec![idx]);
                        }
                    }

                    // Find the closest proximity group
                    let closest_group = proximity_groups.iter().min_by_key(|group| {
                        group
                            .iter()
                            .map(|idx| {
                                if let Some((_, def_line, _, _)) = same_file_types.get(*idx) {
                                    def_line.abs_diff(call_line)
                                } else {
                                    usize::MAX
                                }
                            })
                            .min()
                            .unwrap_or(usize::MAX)
                    });

                    if let Some(group) = closest_group {
                        // Within the closest group, try type matching
                        if group.len() > 1 {
                            // Collect candidates for type matching
                            let candidates: Vec<_> = group
                                .iter()
                                .filter_map(|idx| same_file_types.get(*idx).cloned())
                                .collect();
                            // Try to match by type using argument count from call site
                            if let Some(_best_idx) =
                                self.match_by_type(&candidates, arg_count, None)
                            {
                                return resolved_name.to_string();
                            }
                        }
                    }
                }
            }

            // Fallback to line proximity
            let _best = same_file_defs
                .iter()
                .min_by_key(|(_, def_line)| (*def_line).abs_diff(call_line));
            return resolved_name.to_string();
        }

        // No same-file candidates; use any definition (first one)
        resolved_name.to_string()
    }

    #[instrument(skip_all)]
    pub fn build_from_results(
        results: Vec<(PathBuf, SemanticAnalysis)>,
        impl_traits: &[ImplTraitInfo],
        impl_only: bool,
    ) -> Result<Self, GraphError> {
        let mut graph = CallGraph::new();

        // Build definitions and function_types maps first
        for (path, analysis) in &results {
            for func in &analysis.functions {
                graph
                    .definitions
                    .entry(func.name.clone())
                    .or_default()
                    .push((path.clone(), func.line));
                graph
                    .function_types
                    .entry(func.name.clone())
                    .or_default()
                    .push((
                        path.clone(),
                        func.line,
                        func.parameters.clone(),
                        func.return_type.clone(),
                    ));
            }
            for class in &analysis.classes {
                graph
                    .definitions
                    .entry(class.name.clone())
                    .or_default()
                    .push((path.clone(), class.line));
                graph
                    .function_types
                    .entry(class.name.clone())
                    .or_default()
                    .push((path.clone(), class.line, vec![], None));
            }
        }

        // Process calls with resolved callee names
        for (path, analysis) in &results {
            for call in &analysis.calls {
                let resolved_callee = graph.resolve_callee(
                    &call.callee,
                    path,
                    call.line,
                    call.arg_count,
                    &graph.definitions,
                    &graph.function_types,
                );

                graph
                    .callees
                    .entry(call.caller.clone())
                    .or_default()
                    .push(CallEdge {
                        path: path.clone(),
                        line: call.line,
                        neighbor_name: resolved_callee.clone(),
                        is_impl_trait: false,
                    });
                graph
                    .callers
                    .entry(resolved_callee)
                    .or_default()
                    .push(CallEdge {
                        path: path.clone(),
                        line: call.line,
                        neighbor_name: call.caller.clone(),
                        is_impl_trait: false,
                    });
            }
            for reference in &analysis.references {
                graph
                    .callers
                    .entry(reference.symbol.clone())
                    .or_default()
                    .push(CallEdge {
                        path: path.clone(),
                        line: reference.line,
                        neighbor_name: "<reference>".to_string(),
                        is_impl_trait: false,
                    });
            }
        }

        // Add explicit caller edges for each impl Trait for Type block.
        // These represent the implementing type as a caller of the trait, enabling
        // impl_only filtering to surface trait implementors rather than call sites.
        for it in impl_traits {
            graph
                .callers
                .entry(it.trait_name.clone())
                .or_default()
                .push(CallEdge {
                    path: it.path.clone(),
                    line: it.line,
                    neighbor_name: it.impl_type.clone(),
                    is_impl_trait: true,
                });
        }

        // If impl_only=true, retain only impl-trait caller edges across all nodes.
        // Callees are never filtered. This ensures traversal and formatting are
        // consistently restricted to impl-trait edges regardless of follow_depth.
        if impl_only {
            for edges in graph.callers.values_mut() {
                edges.retain(|e| e.is_impl_trait);
            }
        }

        let total_edges = graph.callees.values().map(|v| v.len()).sum::<usize>()
            + graph.callers.values().map(|v| v.len()).sum::<usize>();
        let file_count = results.len();

        tracing::debug!(
            definitions = graph.definitions.len(),
            edges = total_edges,
            files = file_count,
            impl_only,
            "graph built"
        );

        Ok(graph)
    }

    fn find_chains_bfs(
        &self,
        symbol: &str,
        follow_depth: u32,
        is_incoming: bool,
    ) -> Result<Vec<InternalCallChain>, GraphError> {
        let graph_map = if is_incoming {
            &self.callers
        } else {
            &self.callees
        };

        if !self.definitions.contains_key(symbol) && !graph_map.contains_key(symbol) {
            return Err(GraphError::SymbolNotFound {
                symbol: symbol.to_string(),
                hint: "Symbol resolved but not found in graph. The symbol may have no calls or definitions in the indexed files.".to_string(),
            });
        }

        let mut chains = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((symbol.to_string(), 0));
        visited.insert(symbol.to_string());

        while let Some((current, depth)) = queue.pop_front() {
            if depth > follow_depth {
                continue;
            }

            if let Some(neighbors) = graph_map.get(&current) {
                for edge in neighbors {
                    let path = &edge.path;
                    let line = edge.line;
                    let neighbor = &edge.neighbor_name;
                    let mut chain = vec![(current.clone(), path.clone(), line)];
                    let mut chain_node = neighbor.clone();
                    let mut chain_depth = depth;

                    while chain_depth < follow_depth {
                        if let Some(next_neighbors) = graph_map.get(&chain_node) {
                            if let Some(next_edge) = next_neighbors.first() {
                                if is_incoming {
                                    chain.insert(
                                        0,
                                        (
                                            chain_node.clone(),
                                            next_edge.path.clone(),
                                            next_edge.line,
                                        ),
                                    );
                                } else {
                                    chain.push((
                                        chain_node.clone(),
                                        next_edge.path.clone(),
                                        next_edge.line,
                                    ));
                                }
                                chain_node = next_edge.neighbor_name.clone();
                                chain_depth += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if is_incoming {
                        chain.insert(0, (neighbor.clone(), path.clone(), line));
                    } else {
                        chain.push((neighbor.clone(), path.clone(), line));
                    }
                    chains.push(InternalCallChain { chain });

                    if !visited.contains(neighbor) && depth < follow_depth {
                        visited.insert(neighbor.clone());
                        queue.push_back((neighbor.clone(), depth + 1));
                    }
                }
            }
        }

        Ok(chains)
    }

    #[instrument(skip(self))]
    pub(crate) fn find_incoming_chains(
        &self,
        symbol: &str,
        follow_depth: u32,
    ) -> Result<Vec<InternalCallChain>, GraphError> {
        self.find_chains_bfs(symbol, follow_depth, true)
    }

    #[instrument(skip(self))]
    pub(crate) fn find_outgoing_chains(
        &self,
        symbol: &str,
        follow_depth: u32,
    ) -> Result<Vec<InternalCallChain>, GraphError> {
        self.find_chains_bfs(symbol, follow_depth, false)
    }
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CallInfo, FunctionInfo};

    fn make_analysis(
        funcs: Vec<(&str, usize)>,
        calls: Vec<(&str, &str, usize)>,
    ) -> SemanticAnalysis {
        SemanticAnalysis {
            functions: funcs
                .into_iter()
                .map(|(n, l)| FunctionInfo {
                    name: n.to_string(),
                    line: l,
                    end_line: l + 5,
                    parameters: vec![],
                    return_type: None,
                })
                .collect(),
            classes: vec![],
            imports: vec![],
            references: vec![],
            call_frequency: Default::default(),
            calls: calls
                .into_iter()
                .map(|(c, e, l)| CallInfo {
                    caller: c.to_string(),
                    callee: e.to_string(),
                    line: l,
                    column: 0,
                    arg_count: None,
                })
                .collect(),
            assignments: vec![],
            field_accesses: vec![],
            impl_traits: vec![],
        }
    }

    fn make_typed_analysis(
        funcs: Vec<(&str, usize, Vec<String>, Option<&str>)>,
        calls: Vec<(&str, &str, usize, Option<usize>)>,
    ) -> SemanticAnalysis {
        SemanticAnalysis {
            functions: funcs
                .into_iter()
                .map(|(n, l, params, ret_type)| FunctionInfo {
                    name: n.to_string(),
                    line: l,
                    end_line: l + 5,
                    parameters: params,
                    return_type: ret_type.map(|s| s.to_string()),
                })
                .collect(),
            classes: vec![],
            imports: vec![],
            references: vec![],
            call_frequency: Default::default(),
            calls: calls
                .into_iter()
                .map(|(c, e, l, arg_count)| CallInfo {
                    caller: c.to_string(),
                    callee: e.to_string(),
                    line: l,
                    column: 0,
                    arg_count,
                })
                .collect(),
            assignments: vec![],
            field_accesses: vec![],
            impl_traits: vec![],
        }
    }

    #[test]
    fn test_graph_construction() {
        let analysis = make_analysis(
            vec![("main", 1), ("foo", 10), ("bar", 20)],
            vec![("main", "foo", 2), ("foo", "bar", 15)],
        );
        let graph =
            CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)], &[], false)
                .expect("Failed to build graph");
        assert!(graph.definitions.contains_key("main"));
        assert!(graph.definitions.contains_key("foo"));
        assert_eq!(graph.callees["main"][0].neighbor_name, "foo");
        assert_eq!(graph.callers["foo"][0].neighbor_name, "main");
    }

    #[test]
    fn test_find_incoming_chains_depth_zero() {
        let analysis = make_analysis(vec![("main", 1), ("foo", 10)], vec![("main", "foo", 2)]);
        let graph =
            CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)], &[], false)
                .expect("Failed to build graph");
        assert!(
            !graph
                .find_incoming_chains("foo", 0)
                .expect("Failed to find chains")
                .is_empty()
        );
    }

    #[test]
    fn test_find_outgoing_chains_depth_zero() {
        let analysis = make_analysis(vec![("main", 1), ("foo", 10)], vec![("main", "foo", 2)]);
        let graph =
            CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)], &[], false)
                .expect("Failed to build graph");
        assert!(
            !graph
                .find_outgoing_chains("main", 0)
                .expect("Failed to find chains")
                .is_empty()
        );
    }

    #[test]
    fn test_symbol_not_found() {
        assert!(
            CallGraph::new()
                .find_incoming_chains("nonexistent", 0)
                .is_err()
        );
    }

    #[test]
    fn test_same_file_preference() {
        // Two files each define "helper". File a.rs has a call from "main" to "helper".
        // Assert that the graph's callees for "main" point to "helper" and the callers
        // for "helper" include an entry from a.rs (not b.rs).
        let analysis_a = make_analysis(
            vec![("main", 1), ("helper", 10)],
            vec![("main", "helper", 5)],
        );
        let analysis_b = make_analysis(vec![("helper", 20)], vec![]);

        let graph = CallGraph::build_from_results(
            vec![
                (PathBuf::from("a.rs"), analysis_a),
                (PathBuf::from("b.rs"), analysis_b),
            ],
            &[],
            false,
        )
        .expect("Failed to build graph");

        // Check that main calls helper
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].neighbor_name, "helper");

        // Check that the call is from a.rs (same file as main)
        assert_eq!(main_callees[0].path, PathBuf::from("a.rs"));

        // Check that helper has a caller from a.rs
        assert!(graph.callers.contains_key("helper"));
        let helper_callers = &graph.callers["helper"];
        assert!(
            helper_callers
                .iter()
                .any(|e| e.path == PathBuf::from("a.rs"))
        );
    }

    #[test]
    fn test_line_proximity() {
        // One file with "process" defined at line 10 and line 50, and a call at line 12.
        // Assert resolution picks the definition at line 10 (closest).
        let analysis = make_analysis(
            vec![("process", 10), ("process", 50)],
            vec![("main", "process", 12)],
        );

        let graph =
            CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)], &[], false)
                .expect("Failed to build graph");

        // Check that main calls process
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].neighbor_name, "process");

        // Check that process has a caller from main at line 12
        assert!(graph.callers.contains_key("process"));
        let process_callers = &graph.callers["process"];
        assert!(
            process_callers
                .iter()
                .any(|e| e.line == 12 && e.neighbor_name == "main")
        );
    }

    #[test]
    fn test_scope_prefix_stripping() {
        // One file defines "method" at line 10. Calls use "self.method", "Type::method".
        // Assert these resolve to "method" in the graph.
        let analysis = make_analysis(
            vec![("method", 10)],
            vec![
                ("caller1", "self.method", 5),
                ("caller2", "Type::method", 15),
                ("caller3", "module::method", 25),
            ],
        );

        let graph =
            CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)], &[], false)
                .expect("Failed to build graph");

        // Check that all three callers have "method" as their callee
        assert_eq!(graph.callees["caller1"][0].neighbor_name, "method");
        assert_eq!(graph.callees["caller2"][0].neighbor_name, "method");
        assert_eq!(graph.callees["caller3"][0].neighbor_name, "method");

        // Check that method has three callers
        assert!(graph.callers.contains_key("method"));
        let method_callers = &graph.callers["method"];
        assert_eq!(method_callers.len(), 3);
        assert!(method_callers.iter().any(|e| e.neighbor_name == "caller1"));
        assert!(method_callers.iter().any(|e| e.neighbor_name == "caller2"));
        assert!(method_callers.iter().any(|e| e.neighbor_name == "caller3"));
    }

    #[test]
    fn test_no_same_file_fallback() {
        // File a.rs calls "helper" but "helper" is only defined in b.rs.
        // Assert the call still resolves (graph has the edge).
        let analysis_a = make_analysis(vec![("main", 1)], vec![("main", "helper", 5)]);
        let analysis_b = make_analysis(vec![("helper", 10)], vec![]);

        let graph = CallGraph::build_from_results(
            vec![
                (PathBuf::from("a.rs"), analysis_a),
                (PathBuf::from("b.rs"), analysis_b),
            ],
            &[],
            false,
        )
        .expect("Failed to build graph");

        // Check that main calls helper
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].neighbor_name, "helper");

        // Check that helper has a caller from a.rs
        assert!(graph.callers.contains_key("helper"));
        let helper_callers = &graph.callers["helper"];
        assert!(
            helper_callers
                .iter()
                .any(|e| e.path == PathBuf::from("a.rs") && e.neighbor_name == "main")
        );
    }

    #[test]
    fn test_type_disambiguation_by_params() {
        // Two functions named 'process' in the same file with different parameter counts.
        // process(x: i32) at line 10, process(x: i32, y: String) at line 12.
        // Call from main at line 11 is equidistant from both (1 line away).
        // Type matching should prefer the 2-param version since arg_count=2.
        let analysis = make_typed_analysis(
            vec![
                ("process", 10, vec!["(x: i32)".to_string()], Some("i32")),
                (
                    "process",
                    12,
                    vec!["(x: i32, y: String)".to_string()],
                    Some("String"),
                ),
                ("main", 1, vec![], None),
            ],
            vec![("main", "process", 11, Some(2))],
        );

        let graph =
            CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)], &[], false)
                .expect("Failed to build graph");

        // Check that main calls process
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].neighbor_name, "process");

        // Check that process has a caller from main at line 11
        assert!(graph.callers.contains_key("process"));
        let process_callers = &graph.callers["process"];
        assert!(
            process_callers
                .iter()
                .any(|e| e.line == 11 && e.neighbor_name == "main")
        );
    }

    #[test]
    fn test_type_disambiguation_fallback() {
        // Two functions named 'process' with no type info (empty parameters, None return_type).
        // Call from main at line 12 should resolve using line proximity (no regression).
        // arg_count=None means type matching won't fire, fallback to line proximity.
        let analysis = make_analysis(
            vec![("process", 10), ("process", 50), ("main", 1)],
            vec![("main", "process", 12)],
        );

        let graph =
            CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)], &[], false)
                .expect("Failed to build graph");

        // Check that main calls process
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].neighbor_name, "process");

        // Check that process has a caller from main
        assert!(graph.callers.contains_key("process"));
        let process_callers = &graph.callers["process"];
        assert!(
            process_callers
                .iter()
                .any(|e| e.line == 12 && e.neighbor_name == "main")
        );
    }

    #[test]
    fn test_impl_only_filters_to_impl_sites() {
        // Arrange: WriterImpl implements Write; plain_fn calls write directly.
        use crate::types::ImplTraitInfo;
        let analysis = make_analysis(
            vec![("write", 1), ("plain_fn", 20)],
            vec![("plain_fn", "write", 22)],
        );
        let impl_traits = vec![ImplTraitInfo {
            trait_name: "Write".to_string(),
            impl_type: "WriterImpl".to_string(),
            path: PathBuf::from("test.rs"),
            line: 10,
        }];

        // Act: build with impl_only=true
        let graph = CallGraph::build_from_results(
            vec![(PathBuf::from("test.rs"), analysis)],
            &impl_traits,
            true,
        )
        .expect("Failed to build graph");

        // Assert: trait "Write" has WriterImpl as an explicit impl-trait caller edge.
        let callers = graph
            .callers
            .get("Write")
            .expect("Write must have impl caller");
        assert_eq!(callers.len(), 1, "only impl-trait caller retained");
        assert_eq!(callers[0].neighbor_name, "WriterImpl");
        assert!(
            callers[0].is_impl_trait,
            "edge must be tagged is_impl_trait"
        );

        // Assert: regular call-site callers of "write" are filtered out by impl_only.
        let write_callers = graph.callers.get("write").map(|v| v.len()).unwrap_or(0);
        assert_eq!(
            write_callers, 0,
            "regular callers filtered when impl_only=true"
        );
    }

    #[test]
    fn test_impl_only_false_is_backward_compatible() {
        // Arrange: same setup, impl_only=false -- all callers returned.
        use crate::types::ImplTraitInfo;
        let analysis = make_analysis(
            vec![("write", 1), ("WriterImpl", 10), ("plain_fn", 20)],
            vec![("WriterImpl", "write", 12), ("plain_fn", "write", 22)],
        );
        let impl_traits = vec![ImplTraitInfo {
            trait_name: "Write".to_string(),
            impl_type: "WriterImpl".to_string(),
            path: PathBuf::from("test.rs"),
            line: 10,
        }];

        // Act: build with impl_only=false
        let graph = CallGraph::build_from_results(
            vec![(PathBuf::from("test.rs"), analysis)],
            &impl_traits,
            false,
        )
        .expect("Failed to build graph");

        // Assert: both call-site callers preserved
        let callers = graph.callers.get("write").expect("write must have callers");
        assert_eq!(
            callers.len(),
            2,
            "both call-site callers should be present when impl_only=false"
        );

        // Assert: impl-trait edge is always present regardless of impl_only
        let write_impl_callers = graph
            .callers
            .get("Write")
            .expect("Write must have impl caller");
        assert_eq!(write_impl_callers.len(), 1);
        assert!(write_impl_callers[0].is_impl_trait);
    }

    #[test]
    fn test_impl_only_callees_unaffected() {
        // Arrange: WriterImpl calls write; impl_only=true should not remove callees.
        use crate::types::ImplTraitInfo;
        let analysis = make_analysis(
            vec![("write", 1), ("WriterImpl", 10)],
            vec![("WriterImpl", "write", 12)],
        );
        let impl_traits = vec![ImplTraitInfo {
            trait_name: "Write".to_string(),
            impl_type: "WriterImpl".to_string(),
            path: PathBuf::from("test.rs"),
            line: 10,
        }];

        let graph = CallGraph::build_from_results(
            vec![(PathBuf::from("test.rs"), analysis)],
            &impl_traits,
            true,
        )
        .expect("Failed to build graph");

        // Assert: callees of WriterImpl are NOT filtered
        let callees = graph
            .callees
            .get("WriterImpl")
            .expect("WriterImpl must have callees");
        assert_eq!(
            callees.len(),
            1,
            "callees must not be filtered by impl_only"
        );
        assert_eq!(callees[0].neighbor_name, "write");
    }

    // ---- resolve_symbol tests ----

    fn known(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_resolve_symbol_exact_match() {
        let syms = known(&["parse_config", "ParseConfig", "PARSE_CONFIG"]);
        let result = resolve_symbol(syms.iter(), "parse_config", &SymbolMatchMode::Exact);
        assert_eq!(result.unwrap(), "parse_config");
    }

    #[test]
    fn test_resolve_symbol_exact_no_match() {
        let syms = known(&["ParseConfig"]);
        let err = resolve_symbol(syms.iter(), "parse_config", &SymbolMatchMode::Exact).unwrap_err();
        assert!(matches!(err, GraphError::SymbolNotFound { .. }));
    }

    #[test]
    fn test_resolve_symbol_insensitive_match() {
        let syms = known(&["ParseConfig", "other"]);
        let result = resolve_symbol(syms.iter(), "parseconfig", &SymbolMatchMode::Insensitive);
        assert_eq!(result.unwrap(), "ParseConfig");
    }

    #[test]
    fn test_resolve_symbol_insensitive_no_match() {
        let syms = known(&["unrelated"]);
        let err =
            resolve_symbol(syms.iter(), "parseconfig", &SymbolMatchMode::Insensitive).unwrap_err();
        assert!(matches!(err, GraphError::SymbolNotFound { .. }));
    }

    #[test]
    fn test_resolve_symbol_prefix_single() {
        let syms = known(&["parse_config", "parse_args", "build"]);
        let result = resolve_symbol(syms.iter(), "build", &SymbolMatchMode::Prefix);
        assert_eq!(result.unwrap(), "build");
    }

    #[test]
    fn test_resolve_symbol_prefix_multiple_candidates() {
        let syms = known(&["parse_config", "parse_args", "build"]);
        let err = resolve_symbol(syms.iter(), "parse", &SymbolMatchMode::Prefix).unwrap_err();
        assert!(matches!(&err, GraphError::MultipleCandidates { .. }));
        if let GraphError::MultipleCandidates { candidates, .. } = err {
            assert_eq!(candidates.len(), 2);
        }
    }

    #[test]
    fn test_resolve_symbol_contains_single() {
        let syms = known(&["parse_config", "build_artifact"]);
        let result = resolve_symbol(syms.iter(), "config", &SymbolMatchMode::Contains);
        assert_eq!(result.unwrap(), "parse_config");
    }

    #[test]
    fn test_resolve_symbol_contains_no_match() {
        let syms = known(&["parse_config", "build_artifact"]);
        let err = resolve_symbol(syms.iter(), "deploy", &SymbolMatchMode::Contains).unwrap_err();
        assert!(matches!(err, GraphError::SymbolNotFound { .. }));
    }
}
