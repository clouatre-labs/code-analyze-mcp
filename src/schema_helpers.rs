use schemars::Schema;
use serde_json::json;

/// Returns a plain integer schema without the non-standard "format": "uint"
/// that schemars emits by default for usize/u32 fields.
pub fn integer_schema(_gen: &mut schemars::SchemaGenerator) -> Schema {
    let map = json!({
        "type": "integer",
        "minimum": 0
    })
    .as_object()
    .unwrap()
    .clone();
    Schema::from(map)
}

/// Returns a nullable integer schema for Option<usize> / Option<u32> fields.
pub fn option_integer_schema(_gen: &mut schemars::SchemaGenerator) -> Schema {
    let map = json!({
        "type": ["integer", "null"],
        "minimum": 0
    })
    .as_object()
    .unwrap()
    .clone();
    Schema::from(map)
}

/// Returns a nullable integer schema for Option<usize> ast_recursion_limit fields.
/// Enforces minimum: 1 because 0 would limit tree-sitter traversal to the root
/// node only, silently returning zero results. Values below 1 are treated as
/// unlimited at runtime; the schema minimum signals to callers that 0 is not useful.
pub fn option_ast_limit_schema(_gen: &mut schemars::SchemaGenerator) -> Schema {
    let map = json!({
        "type": ["integer", "null"],
        "minimum": 1
    })
    .as_object()
    .unwrap()
    .clone();
    Schema::from(map)
}

/// Returns a nullable integer schema for Option<usize> page_size fields.
/// Enforces minimum: 1 to prevent callers from sending page_size=0, which
/// would cause paginate_slice to make no progress and loop on the same cursor.
pub fn option_page_size_schema(_gen: &mut schemars::SchemaGenerator) -> Schema {
    let map = json!({
        "type": ["integer", "null"],
        "minimum": 1
    })
    .as_object()
    .unwrap()
    .clone();
    Schema::from(map)
}
