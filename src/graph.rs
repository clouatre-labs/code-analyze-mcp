use crate::types::SemanticAnalysis;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
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

    #[instrument(skip_all)]
    pub fn build_from_results(
        results: Vec<(PathBuf, SemanticAnalysis)>,
    ) -> Result<Self, GraphError> {
        let mut graph = CallGraph::new();

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

        for (path, analysis) in &results {
            for call in &analysis.calls {
                graph.callees.entry(call.caller.clone()).or_default().push((
                    path.clone(),
                    call.line,
                    call.callee.clone(),
                ));
                graph.callers.entry(call.callee.clone()).or_default().push((
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
}
