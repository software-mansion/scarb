use scarb_manifest_schema::{SchemaTraverser, get_manifest_schema};
use serde_json::json;

fn setup_schema() -> SchemaTraverser {
    let schema = get_manifest_schema();
    SchemaTraverser::new(schema)
}

#[test]
fn test_simple_traversal() {
    let traverser = setup_schema();
    let result = traverser.traverse(vec!["cairo"]).unwrap();
    assert!(result["description"].is_null());
}

#[test]
fn test_nested_traversal_with_ref() {
    let traverser = setup_schema();
    let result = traverser.traverse(vec!["package", "no-core"]).unwrap();
    let expected = json!({
          "description": "**UNSTABLE** This package does not depend on Cairo's `core`.",
          "type": [
            "boolean",
            "null"
          ]
    });
    assert_eq!(*result, expected);
}

#[test]
fn test_nested_traversal_with_ref2() {
    let traverser = setup_schema();
    let result = traverser
        .traverse(vec!["workspace", "require-audits"])
        .unwrap();
    assert!(result["description"].is_null());
}

#[test]
fn test_double_ref_resolution() {
    let traverser = setup_schema();
    let result = traverser.traverse(vec!["package", "version"]).unwrap();
    assert!(result["description"].is_null());
}

#[test]
fn test_error_missing_key() {
    let traverser = setup_schema();
    let result = traverser.traverse(vec!["non_existent_field"]);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Couldn't resolve 'non_existent_field' at key 'non_existent_field'.")
    );
}

#[test]
fn test_error_traversing_into_leaf() {
    let traverser = setup_schema();
    let result = traverser.traverse(vec!["cairo", "something_else"]);
    assert!(result.is_err());

    let e = result.unwrap_err();
    assert_eq!(
        e.to_string(),
        "Couldn't resolve 'cairo.something_else' at key 'something_else'."
    );
}
