use indoc::indoc;
use scarb_manifest_schema::{SchemaTraverser, get_manifest_schema};
use serde_json::json;
use test_case::test_case;

fn setup_schema() -> SchemaTraverser {
    let schema = get_manifest_schema();
    SchemaTraverser::new(schema)
}

#[test_case(vec!["cairo",], indoc!{r#"
        Global Cairo compiler configuration for this package or workspace profile.
        - See official documentation at: https://docs.swmansion.com/scarb/docs/reference/manifest.html#cairo"#} ; "simple_traversal")]
#[test_case(vec!["package", "version"], indoc!{r#"
        Package version obeying Semantic Versioning (semver), e.g. `"0.1.0"`.
        Can be inherited from the workspace via `{ workspace = true }`.
        - See official documentation at: https://docs.swmansion.com/scarb/docs/reference/manifest.html#version"#} ; "nested_traversal1")]
#[test_case(vec!["workspace", "require-audits"], indoc!{r#"
        Setting this field to true will cause Scarb to ignore any versions of dependencies, including transitive ones, that are not marked as audited in the registry.
        If unable to resolve the dependency tree due to this, Scarb will exit with an error.
        By default, this field is set to false. This policy applies to the entire workspace.
        This field is ignored in member packages manifest files, and only the one defined in the workspace root manifest is applied when compiling member packages.

        You may whitelist specific packages to ignore the require-audits setting by specifying them in the allow-no-audits key:
        ```toml
        [workspace]
        allow-no-audits = ["alexandria_math"]
        ```
        - See official documentation at: https://docs.swmansion.com/scarb/docs/reference/workspaces.html#security-and-audits"#} ; "nested_traversal2")]
fn test_traverse(path: Vec<&str>, expected_description: &str) {
    let traverser = setup_schema();
    let result = traverser.traverse(path.to_vec()).unwrap();
    assert_eq!(
        result["description"].as_str().unwrap(),
        expected_description
    );
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
fn test_error_missing_key() {
    let traverser = setup_schema();
    let result = traverser.traverse(vec!["non_existent_field"]);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Property 'non_existent_field' not found")
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
        "Property 'something_else' not found in schema"
    );
}
