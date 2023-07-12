use std::collections::BTreeMap;

use assert_fs::prelude::*;
use serde_json::json;

use scarb_metadata::{Cfg, ManifestMetadataBuilder, Metadata, PackageMetadata};

use crate::support::command::{CommandExt, Scarb};
use crate::support::project_builder::ProjectBuilder;

fn packages_by_name(meta: Metadata) -> BTreeMap<String, PackageMetadata> {
    meta.packages
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect::<BTreeMap<_, _>>()
}

fn packages_and_deps(meta: Metadata) -> BTreeMap<String, Vec<String>> {
    meta.packages
        .into_iter()
        .map(|p| {
            let deps = p
                .dependencies
                .into_iter()
                .map(|d| d.name)
                .collect::<Vec<_>>();
            (p.name, deps)
        })
        .collect::<BTreeMap<_, _>>()
}

#[test]
fn simple() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
}

#[test]
fn includes_compilation_units() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    let output = Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert!(!output.compilation_units.is_empty());
    let unit = &output.compilation_units[0];
    assert!(unit.package.repr.starts_with("hello "));
    assert_eq!(unit.target.name, "hello");
    assert!(!unit.components.is_empty());
    assert!(unit
        .cfg
        .contains(&Cfg::KV("target".into(), unit.target.kind.clone())));
}

#[test]
fn fails_without_format_version() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);

    Scarb::quick_snapbox()
        .arg("metadata")
        .current_dir(&t)
        .assert()
        .failure();
}

fn create_local_dependencies_setup(t: &assert_fs::TempDir) {
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "x"
            version = "1.0.0"

            [dependencies]
            y = { path = "y" }
            "#,
        )
        .unwrap();

    t.child("src/lib.cairo")
        .write_str(r"fn f() -> felt252 { y::f() }")
        .unwrap();

    t.child("y/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "y"
            version = "1.0.0"

            [dependencies]
            q = { path = "../q" }
            z = { path = "../z" }
            "#,
        )
        .unwrap();

    t.child("y/src/lib.cairo")
        .write_str(r"fn f() -> felt252 { z::f() }")
        .unwrap();

    t.child("z/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "z"
            version = "1.0.0"

            [dependencies]
            q = { path = "../q" }
            "#,
        )
        .unwrap();

    t.child("z/src/lib.cairo")
        .write_str(r"fn f() -> felt252 { q::f() }")
        .unwrap();

    t.child("q/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "q"
            version = "1.0.0"
            "#,
        )
        .unwrap();

    t.child("q/src/lib.cairo")
        .write_str(r"fn f() -> felt252 { 42 }")
        .unwrap();
}

#[test]
fn local_dependencies() {
    let t = assert_fs::TempDir::new().unwrap();
    create_local_dependencies_setup(&t);
    let meta = Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    assert_eq!(
        packages_and_deps(meta),
        BTreeMap::from_iter([
            ("core".to_string(), vec![]),
            ("q".to_string(), vec!["core".to_string()]),
            ("x".to_string(), vec!["core".to_string(), "y".to_string()]),
            (
                "y".to_string(),
                vec!["core".to_string(), "q".to_string(), "z".to_string()]
            ),
            ("z".to_string(), vec!["core".to_string(), "q".to_string()]),
        ])
    )
}

#[test]
fn no_dep() {
    let t = assert_fs::TempDir::new().unwrap();
    create_local_dependencies_setup(&t);
    let meta = Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(meta),
        BTreeMap::from_iter([("x".to_string(), vec!["core".to_string(), "y".to_string()])])
    );
}

#[test]
fn manifest_targets_and_metadata() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            description = "Some interesting description to read!"
            authors = ["John Doe <john.doe@swmansion.com>", "Jane Doe <jane.doe@swmansion.com>"]
            keywords = ["some", "project", "keywords"]

            homepage = "https://www.homepage.com/"
            documentation = "https://docs.homepage.com/"
            repository = "https://github.com/johndoe/repo"

            license = "MIT License"
            license-file = "./license.md"
            readme = "./readme.md"

            [package.urls]
            hello = "https://world.com/"

            [package.metadata]
            meta = "data"
            numeric = "1231"
            key = "value"

            [tool]
            meta = "data"
            numeric = 1231

            [tool.table]
            key = "value"

            [lib]
            sierra = false
            casm = true

            [[target.example]]
            string = "bar"
            number = 1234
            bool = true
            array = ["a", 1]
            table = { x = "y" }

            "#,
        )
        .unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        meta.packages
            .into_iter()
            .find(|p| p.name == "hello")
            .unwrap()
            .manifest_metadata,
        ManifestMetadataBuilder::default()
            .authors(Some(vec![
                "John Doe <john.doe@swmansion.com>".to_string(),
                "Jane Doe <jane.doe@swmansion.com>".to_string(),
            ]))
            .urls(BTreeMap::from_iter([(
                "hello".to_string(),
                "https://world.com/".to_string()
            )]))
            .description(Some("Some interesting description to read!".to_string()))
            .documentation(Some("https://docs.homepage.com/".to_string()))
            .homepage(Some("https://www.homepage.com/".to_string()))
            .keywords(Some(vec![
                "some".to_string(),
                "project".to_string(),
                "keywords".to_string(),
            ]))
            .license(Some("MIT License".to_string()))
            .license_file(Some("./license.md".to_string()))
            .readme(Some("./readme.md".to_string()))
            .repository(Some("https://github.com/johndoe/repo".to_string()))
            .tool(Some(BTreeMap::from_iter([
                ("meta".to_string(), json!("data")),
                ("numeric".to_string(), json!(1231)),
                ("table".to_string(), json!({ "key": "value" }))
            ])))
            .build()
            .unwrap()
    );
}

#[test]
fn tool_metadata_is_packaged_contained() {
    let t = assert_fs::TempDir::new().unwrap();
    create_local_dependencies_setup(&t);
    t.child("q/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "q"
            version = "1.0.0"

            [tool.table]
            key = "value"
            "#,
        )
        .unwrap();
    let meta = Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    assert_eq!(
        packages_by_name(meta)
            .into_iter()
            .map(|(k, p)| (k, p.manifest_metadata.tool))
            .collect::<BTreeMap<_, _>>(),
        BTreeMap::from_iter([
            ("core".to_string(), None),
            (
                "q".to_string(),
                Some(BTreeMap::from_iter([(
                    "table".to_string(),
                    json!({ "key": "value" })
                )]))
            ),
            ("x".to_string(), None),
            ("y".to_string(), None),
            ("z".to_string(), None),
        ])
    )
}

#[test]
fn json_output_is_not_pretty() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches("{\"version\":1,[..]}\n");
}

// TODO(#12): Add tests with workspaces
