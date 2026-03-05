use crate::types::SemanticAnalysis;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::instrument;

/// DataflowGraph tracks variable assignments and field accesses.
/// Internal struct (no Serialize/Deserialize); provides query interface for focused analysis.
#[derive(Debug, Clone)]
pub struct DataflowGraph {
    /// Map: variable_name -> vec of (file_path, line, scope)
    pub assignments: HashMap<String, Vec<(PathBuf, usize, String)>>,
    /// Map: object.field -> vec of (file_path, line, scope)
    pub field_accesses: HashMap<String, Vec<(PathBuf, usize, String)>>,
}

impl DataflowGraph {
    /// Create a new empty DataflowGraph.
    pub fn new() -> Self {
        Self {
            assignments: HashMap::new(),
            field_accesses: HashMap::new(),
        }
    }

    /// Build a DataflowGraph from analysis results across multiple files.
    #[instrument(skip(results))]
    pub fn build_from_results(results: &[(PathBuf, SemanticAnalysis)]) -> Self {
        let mut graph = Self::new();
        for (path, analysis) in results {
            for assignment in &analysis.assignments {
                graph
                    .assignments
                    .entry(assignment.variable.clone())
                    .or_default()
                    .push((path.clone(), assignment.line, assignment.scope.clone()));
            }
            for field_access in &analysis.field_accesses {
                let key = format!("{}.{}", field_access.object, field_access.field);
                graph.field_accesses.entry(key).or_default().push((
                    path.clone(),
                    field_access.line,
                    field_access.scope.clone(),
                ));
            }
        }
        graph
    }

    /// Find all assignments to a variable by name.
    pub fn find_assignments(&self, symbol: &str) -> Vec<(PathBuf, usize, String)> {
        self.assignments.get(symbol).cloned().unwrap_or_default()
    }

    /// Find all field accesses where the object matches the given symbol.
    /// Searches for keys prefixed with `symbol.` (e.g., symbol "user" matches "user.name", "user.age").
    pub fn find_field_accesses(&self, symbol: &str) -> Vec<(PathBuf, usize, String)> {
        let prefix = format!("{}.", symbol);
        self.field_accesses
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .flat_map(|(_, entries)| entries.clone())
            .collect()
    }
}

impl Default for DataflowGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssignmentInfo, FieldAccessInfo};

    #[test]
    fn test_dataflow_graph_construction() {
        let mut analysis1 = SemanticAnalysis {
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            references: Vec::new(),
            call_frequency: Default::default(),
            calls: Vec::new(),
            assignments: vec![
                AssignmentInfo {
                    variable: "x".to_string(),
                    value: "42".to_string(),
                    line: 5,
                    scope: "main".to_string(),
                },
                AssignmentInfo {
                    variable: "y".to_string(),
                    value: "x + 1".to_string(),
                    line: 6,
                    scope: "main".to_string(),
                },
            ],
            field_accesses: vec![FieldAccessInfo {
                object: "obj".to_string(),
                field: "name".to_string(),
                line: 7,
                scope: "main".to_string(),
            }],
        };

        let path1 = PathBuf::from("test.rs");
        let graph = DataflowGraph::build_from_results(&[(path1.clone(), analysis1)]);

        let x_assignments = graph.find_assignments("x");
        assert_eq!(x_assignments.len(), 1);
        assert_eq!(x_assignments[0].1, 5);
        assert_eq!(x_assignments[0].2, "main");

        let y_assignments = graph.find_assignments("y");
        assert_eq!(y_assignments.len(), 1);
        assert_eq!(y_assignments[0].1, 6);

        let field_accesses = graph.find_field_accesses("obj");
        assert_eq!(field_accesses.len(), 1);
        assert_eq!(field_accesses[0].1, 7);
    }

    #[test]
    fn test_dataflow_graph_shadowed_variables() {
        let analysis1 = SemanticAnalysis {
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            references: Vec::new(),
            call_frequency: Default::default(),
            calls: Vec::new(),
            assignments: vec![
                AssignmentInfo {
                    variable: "x".to_string(),
                    value: "10".to_string(),
                    line: 3,
                    scope: "outer".to_string(),
                },
                AssignmentInfo {
                    variable: "x".to_string(),
                    value: "20".to_string(),
                    line: 8,
                    scope: "inner".to_string(),
                },
            ],
            field_accesses: Vec::new(),
        };

        let path1 = PathBuf::from("test.rs");
        let graph = DataflowGraph::build_from_results(&[(path1, analysis1)]);

        let x_assignments = graph.find_assignments("x");
        assert_eq!(
            x_assignments.len(),
            2,
            "Both shadowed assignments should be tracked"
        );
        assert_eq!(x_assignments[0].2, "outer");
        assert_eq!(x_assignments[1].2, "inner");
    }

    #[test]
    fn test_find_field_accesses_no_false_prefix_match() {
        let analysis = SemanticAnalysis {
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            references: Vec::new(),
            call_frequency: Default::default(),
            calls: Vec::new(),
            assignments: Vec::new(),
            field_accesses: vec![
                FieldAccessInfo {
                    object: "objective".to_string(),
                    field: "status".to_string(),
                    line: 5,
                    scope: "run".to_string(),
                },
                FieldAccessInfo {
                    object: "obj".to_string(),
                    field: "name".to_string(),
                    line: 10,
                    scope: "run".to_string(),
                },
            ],
        };

        let path = PathBuf::from("test.rs");
        let graph = DataflowGraph::build_from_results(&[(path, analysis)]);

        let matches = graph.find_field_accesses("obj");
        assert_eq!(
            matches.len(),
            1,
            "must not match 'objective.status' for symbol 'obj'"
        );
        assert_eq!(matches[0].1, 10);
    }
}
