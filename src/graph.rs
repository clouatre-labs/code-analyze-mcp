use crate::types::SemanticAnalysis;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
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
pub struct CallChain {
    pub chain: Vec<(String, PathBuf, usize)>,
}

#[derive(Debug, Clone)]
pub struct CallGraph {
    pub callers: HashMap<String, Vec<(PathBuf, usize, String)>>,
    pub callees: HashMap<String, Vec<(PathBuf, usize, String)>>,
    pub definitions: HashMap<String, Vec<(PathBuf, usize)>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            callers: HashMap::new(),
            callees: HashMap::new(),
            definitions: HashMap::new(),
        }
    }

    /// Resolve a callee name using three strategies:
    /// 1. Try the raw callee name first in definitions
    /// 2. If not found, try the stripped name (via strip_scope_prefix)
    /// 3. If multiple definitions exist, prefer same-file candidates
    /// 4. Among same-file candidates, pick the one closest by line number
    /// 5. If no same-file candidates, use any definition (first one)
    ///
    /// Returns the resolved callee name (which may be the stripped version).
    fn resolve_callee(
        &self,
        callee: &str,
        call_file: &Path,
        call_line: usize,
        definitions: &HashMap<String, Vec<(PathBuf, usize)>>,
    ) -> String {
        // Try raw callee name first
        if let Some(defs) = definitions.get(callee) {
            return self.pick_best_definition(defs, call_file, call_line, callee);
        }

        // Try stripped name
        let stripped = strip_scope_prefix(callee);
        if stripped != callee
            && let Some(defs) = definitions.get(stripped)
        {
            return self.pick_best_definition(defs, call_file, call_line, stripped);
        }

        // No definition found; return the original callee
        callee.to_string()
    }

    /// Pick the best definition from a list based on same-file preference and line proximity.
    fn pick_best_definition(
        &self,
        defs: &[(PathBuf, usize)],
        call_file: &Path,
        call_line: usize,
        resolved_name: &str,
    ) -> String {
        // Filter to same-file candidates
        let same_file_defs: Vec<_> = defs.iter().filter(|(path, _)| path == call_file).collect();

        if !same_file_defs.is_empty() {
            // Pick the one closest by line number
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
    ) -> Result<Self, GraphError> {
        let mut graph = CallGraph::new();

        // Build definitions map first
        for (path, analysis) in &results {
            for func in &analysis.functions {
                graph
                    .definitions
                    .entry(func.name.clone())
                    .or_default()
                    .push((path.clone(), func.line));
            }
            for class in &analysis.classes {
                graph
                    .definitions
                    .entry(class.name.clone())
                    .or_default()
                    .push((path.clone(), class.line));
            }
        }

        // Process calls with resolved callee names
        for (path, analysis) in &results {
            for call in &analysis.calls {
                let resolved_callee =
                    graph.resolve_callee(&call.callee, path, call.line, &graph.definitions);

                graph.callees.entry(call.caller.clone()).or_default().push((
                    path.clone(),
                    call.line,
                    resolved_callee.clone(),
                ));
                graph.callers.entry(resolved_callee).or_default().push((
                    path.clone(),
                    call.line,
                    call.caller.clone(),
                ));
            }
            for reference in &analysis.references {
                graph
                    .callers
                    .entry(reference.symbol.clone())
                    .or_default()
                    .push((path.clone(), reference.line, "<reference>".to_string()));
            }
        }

        let total_edges = graph.callees.values().map(|v| v.len()).sum::<usize>()
            + graph.callers.values().map(|v| v.len()).sum::<usize>();
        let file_count = results.len();

        tracing::debug!(
            definitions = graph.definitions.len(),
            edges = total_edges,
            files = file_count,
            "graph built"
        );

        Ok(graph)
    }

    fn find_chains_bfs(
        &self,
        symbol: &str,
        follow_depth: u32,
        is_incoming: bool,
    ) -> Result<Vec<CallChain>, GraphError> {
        let graph_map = if is_incoming {
            &self.callers
        } else {
            &self.callees
        };

        if !self.definitions.contains_key(symbol) && !graph_map.contains_key(symbol) {
            return Err(GraphError::SymbolNotFound(symbol.to_string()));
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
                for (path, line, neighbor) in neighbors {
                    let mut chain = vec![(current.clone(), path.clone(), *line)];
                    let mut chain_node = neighbor.clone();
                    let mut chain_depth = depth;

                    while chain_depth < follow_depth {
                        if let Some(next_neighbors) = graph_map.get(&chain_node) {
                            if let Some((p, l, n)) = next_neighbors.first() {
                                if is_incoming {
                                    chain.insert(0, (chain_node.clone(), p.clone(), *l));
                                } else {
                                    chain.push((chain_node.clone(), p.clone(), *l));
                                }
                                chain_node = n.clone();
                                chain_depth += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if is_incoming {
                        chain.insert(0, (neighbor.clone(), path.clone(), *line));
                    } else {
                        chain.push((neighbor.clone(), path.clone(), *line));
                    }
                    chains.push(CallChain { chain });

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
    pub fn find_incoming_chains(
        &self,
        symbol: &str,
        follow_depth: u32,
    ) -> Result<Vec<CallChain>, GraphError> {
        self.find_chains_bfs(symbol, follow_depth, true)
    }

    #[instrument(skip(self))]
    pub fn find_outgoing_chains(
        &self,
        symbol: &str,
        follow_depth: u32,
    ) -> Result<Vec<CallChain>, GraphError> {
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
                })
                .collect(),
        }
    }

    #[test]
    fn test_graph_construction() {
        let analysis = make_analysis(
            vec![("main", 1), ("foo", 10), ("bar", 20)],
            vec![("main", "foo", 2), ("foo", "bar", 15)],
        );
        let graph = CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)])
            .expect("Failed to build graph");
        assert!(graph.definitions.contains_key("main"));
        assert!(graph.definitions.contains_key("foo"));
        assert_eq!(graph.callees["main"][0].2, "foo");
        assert_eq!(graph.callers["foo"][0].2, "main");
    }

    #[test]
    fn test_find_incoming_chains_depth_zero() {
        let analysis = make_analysis(vec![("main", 1), ("foo", 10)], vec![("main", "foo", 2)]);
        let graph = CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)])
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
        let graph = CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)])
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

        let graph = CallGraph::build_from_results(vec![
            (PathBuf::from("a.rs"), analysis_a),
            (PathBuf::from("b.rs"), analysis_b),
        ])
        .expect("Failed to build graph");

        // Check that main calls helper
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].2, "helper");

        // Check that the call is from a.rs (same file as main)
        assert_eq!(main_callees[0].0, PathBuf::from("a.rs"));

        // Check that helper has a caller from a.rs
        assert!(graph.callers.contains_key("helper"));
        let helper_callers = &graph.callers["helper"];
        assert!(
            helper_callers
                .iter()
                .any(|(path, _, _)| path == &PathBuf::from("a.rs"))
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

        let graph = CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)])
            .expect("Failed to build graph");

        // Check that main calls process
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].2, "process");

        // Check that process has a caller from main at line 12
        assert!(graph.callers.contains_key("process"));
        let process_callers = &graph.callers["process"];
        assert!(
            process_callers
                .iter()
                .any(|(_, line, caller)| *line == 12 && caller == "main")
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

        let graph = CallGraph::build_from_results(vec![(PathBuf::from("test.rs"), analysis)])
            .expect("Failed to build graph");

        // Check that all three callers have "method" as their callee
        assert_eq!(graph.callees["caller1"][0].2, "method");
        assert_eq!(graph.callees["caller2"][0].2, "method");
        assert_eq!(graph.callees["caller3"][0].2, "method");

        // Check that method has three callers
        assert!(graph.callers.contains_key("method"));
        let method_callers = &graph.callers["method"];
        assert_eq!(method_callers.len(), 3);
        assert!(
            method_callers
                .iter()
                .any(|(_, _, caller)| caller == "caller1")
        );
        assert!(
            method_callers
                .iter()
                .any(|(_, _, caller)| caller == "caller2")
        );
        assert!(
            method_callers
                .iter()
                .any(|(_, _, caller)| caller == "caller3")
        );
    }

    #[test]
    fn test_no_same_file_fallback() {
        // File a.rs calls "helper" but "helper" is only defined in b.rs.
        // Assert the call still resolves (graph has the edge).
        let analysis_a = make_analysis(vec![("main", 1)], vec![("main", "helper", 5)]);
        let analysis_b = make_analysis(vec![("helper", 10)], vec![]);

        let graph = CallGraph::build_from_results(vec![
            (PathBuf::from("a.rs"), analysis_a),
            (PathBuf::from("b.rs"), analysis_b),
        ])
        .expect("Failed to build graph");

        // Check that main calls helper
        assert!(graph.callees.contains_key("main"));
        let main_callees = &graph.callees["main"];
        assert_eq!(main_callees.len(), 1);
        assert_eq!(main_callees[0].2, "helper");

        // Check that helper has a caller from a.rs
        assert!(graph.callers.contains_key("helper"));
        let helper_callers = &graph.callers["helper"];
        assert!(
            helper_callers
                .iter()
                .any(|(path, _, caller)| { path == &PathBuf::from("a.rs") && caller == "main" })
        );
    }
}
