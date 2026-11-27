use indoc::indoc;
use scarb_manifest_schema::find_unknown_fields;
use test_case::test_case;

fn unknown(manifest: &str) -> Vec<String> {
    find_unknown_fields(manifest)
        .unwrap()
        .into_iter()
        .map(|path| path.join("."))
        .collect()
}

#[test]
fn valid_manifest_has_no_unknown_fields() {
    let manifest = indoc! {r#"
        [package]
        name = "hello"
        version = "0.1.0"

        [dependencies]
        starknet = "2.0.0"
        foo = { path = "foo" }
        bar = { git = "https://example.com/bar.git", tag = "v1" }

        [target-defaults.test]
        build-external-contracts = ["hello::Foo"]
    "#};
    assert!(unknown(manifest).is_empty());
}

#[test_case(indoc! {r#"
    bogus = 1

    [package]
    name = "hello"
    version = "0.1.0"
"#}, &["bogus"] ; "top_level")]
#[test_case(indoc! {r#"
    [package]
    name = "hello"
    version = "0.1.0"
    bogus = 1
"#}, &["package.bogus"] ; "package")]
#[test_case(indoc! {r#"
    [package]
    name = "hello"
    version = { workspace = true, bogus = 1 }
"#}, &["package.version.bogus"] ; "workspace_field")]
#[test_case(indoc! {r#"
    [package]
    name = "hello"
    version = "0.1.0"

    [target-defaults.test]
    build-external-contracts = ["hello::Foo"]
    build-externel-contracts = ["hello::Foo"]
"#}, &["target-defaults.test.build-externel-contracts"] ; "target_defaults_defined")]
#[test_case(indoc! {r#"
    [package]
    name = "hello"
    version = "0.1.0"

    [target-defaults.test]
    workspace = true
    bogus = 1
"#}, &["target-defaults.test.bogus"] ; "target_defaults_workspace")]
#[test_case(indoc! {r#"
    [package]
    name = "hello"
    version = "0.1.0"

    [dependencies]
    foo = { path = "foo", bogus = 1 }
"#}, &["dependencies.foo.bogus"] ; "dependency_detailed")]
#[test_case(indoc! {r#"
    [package]
    name = "hello"
    version = "0.1.0"

    [dependencies]
    foo = { workspace = true, bogus = 1 }
"#}, &["dependencies.foo.bogus"] ; "dependency_workspace")]
fn reports_only_unknown_fields(manifest: &str, expected: &[&str]) {
    assert_eq!(unknown(manifest), expected);
}
