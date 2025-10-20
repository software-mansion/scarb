use std::collections::BTreeMap;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;
use itertools::Itertools;
use serde_json::json;

use scarb_metadata::{Cfg, DepKind, ManifestMetadataBuilder, Metadata, PackageMetadata};
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

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

fn units_and_components(meta: Metadata) -> BTreeMap<String, Vec<String>> {
    meta.compilation_units
        .iter()
        .map(|cu| {
            (
                cu.target.name.clone(),
                cu.components.iter().map(|c| c.name.clone()).collect_vec(),
            )
        })
        .collect::<BTreeMap<_, _>>()
}

fn units_and_plugins(meta: Metadata) -> BTreeMap<String, Vec<String>> {
    meta.compilation_units
        .iter()
        .map(|cu| {
            (
                cu.target.name.clone(),
                cu.cairo_plugins
                    .iter()
                    .map(|c| {
                        meta.packages
                            .iter()
                            .find(|p| p.id == c.package)
                            .map(|p| p.name.clone())
                            .unwrap()
                    })
                    .collect_vec(),
            )
        })
        .collect::<BTreeMap<_, _>>()
}

#[test]
fn simple() {
    let t = TempDir::new().unwrap();
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
        .stdout_json::<Metadata>();
}

#[test]
fn includes_compilation_units() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    let output = Scarb::quick_snapbox()
        .arg("--json")
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
    assert!(
        unit.cfg
            .contains(&Cfg::KV("target".into(), unit.target.kind.clone()))
    );
}

#[test]
fn fails_without_format_version() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);

    Scarb::quick_snapbox()
        .arg("metadata")
        .current_dir(&t)
        .assert()
        .failure();
}

fn create_local_dependencies_setup(t: &TempDir) {
    ProjectBuilder::start()
        .name("q")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { 42 }")
        .dep_cairo_test()
        .build(&t.child("q"));

    ProjectBuilder::start()
        .name("z")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { q::f() }")
        .dep_cairo_test()
        .dep("q", Dep.path("../q"))
        .build(&t.child("z"));

    ProjectBuilder::start()
        .name("y")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { z::f() }")
        .dep_cairo_test()
        .dep("z", Dep.path("../z"))
        .dep("q", Dep.path("../q"))
        .build(&t.child("y"));

    ProjectBuilder::start()
        .name("x")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { y::f() }")
        .dep_cairo_test()
        .dep("y", Dep.path("y"))
        .build(t);
}

#[test]
fn local_dependencies() {
    let t = TempDir::new().unwrap();
    create_local_dependencies_setup(&t);
    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    assert_eq!(
        packages_and_deps(meta),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            ("cairo_test".to_string(), vec![]),
            (
                "q".to_string(),
                vec!["cairo_test".to_string(), "core".to_string(),]
            ),
            (
                "x".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "y".to_string()
                ]
            ),
            (
                "y".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "q".to_string(),
                    "z".to_string()
                ]
            ),
            (
                "z".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "q".to_string(),
                ]
            ),
        ])
    )
}

#[test]
fn dev_dependencies() {
    let t = TempDir::new().unwrap();
    let q = t.child("q");
    ProjectBuilder::start().name("q").dep_cairo_test().build(&q);
    ProjectBuilder::start()
        .name("x")
        .dep("q", Dep.path("./q"))
        .dep_cairo_test()
        .dev_dep("q", Dep.path("./q"))
        .build(&t);
    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    assert_eq!(
        packages_and_deps(meta.clone()),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            ("cairo_test".to_string(), vec![]),
            (
                "x".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "q".to_string(),
                    "q".to_string(),
                ]
            ),
            (
                "q".to_string(),
                vec!["cairo_test".to_string(), "core".to_string(),]
            )
        ])
    );
    assert_eq!(
        meta.packages
            .into_iter()
            .filter(|p| p.name == "x")
            .flat_map(|p| {
                p.dependencies
                    .into_iter()
                    .map(|d| (d.name, d.kind))
                    .collect::<Vec<_>>()
            })
            .filter(|(n, _)| n == "q")
            .collect::<Vec<_>>(),
        vec![
            ("q".to_string(), None),
            ("q".to_string(), Some(DepKind::Dev)),
        ]
    );
}

#[test]
fn dev_deps_are_not_propagated() {
    let t = TempDir::new().unwrap();

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .dep_cairo_test()
        .build(&dep1);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .dep_cairo_test()
        .dev_dep("dep1", &dep1)
        .build(&dep2);

    let dep3 = t.child("dep3");
    ProjectBuilder::start()
        .name("dep3")
        .dep_cairo_test()
        .dep("dep2", &dep2)
        .build(&dep3);

    let pkg = t.child("pkg");
    ProjectBuilder::start()
        .name("x")
        .dep_cairo_test()
        .dev_dep("dep3", &dep3)
        .build(&pkg);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&pkg)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(metadata.clone()),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            ("cairo_test".to_string(), vec![]),
            (
                "x".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "dep3".to_string(),
                ]
            ),
            (
                "dep2".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "dep1".to_string(),
                ]
            ),
            (
                "dep3".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "dep2".to_string(),
                ]
            )
        ])
    );

    assert_eq!(
        units_and_components(metadata),
        BTreeMap::from_iter(vec![
            ("x".to_string(), vec!["core".to_string(), "x".to_string()]),
            (
                "x_unittest".to_string(),
                vec![
                    "core".to_string(),
                    // With dev-deps propagation enabled, this would be included
                    // "dep1".to_string(),
                    "dep2".to_string(),
                    "dep3".to_string(),
                    "x".to_string()
                ]
            ),
        ])
    );
}

#[test]
fn dev_deps_are_not_propagated_for_ws_members() {
    let t = TempDir::new().unwrap();

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .dep_cairo_test()
        .build(&dep1);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .dep_cairo_test()
        .dev_dep("dep1", &dep1)
        .build(&dep2);

    let pkg = t.child("pkg");
    ProjectBuilder::start()
        .name("x")
        .dep_cairo_test()
        .dep("dep2", &dep2)
        .build(&pkg);

    WorkspaceBuilder::start()
        .add_member("dep2")
        .add_member("pkg")
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        units_and_components(metadata),
        BTreeMap::from_iter(vec![
            (
                "dep2".to_string(),
                vec!["core".to_string(), "dep2".to_string()]
            ),
            (
                "dep2_unittest".to_string(),
                vec!["core".to_string(), "dep1".to_string(), "dep2".to_string()]
            ),
            (
                "x".to_string(),
                vec!["core".to_string(), "dep2".to_string(), "x".to_string()]
            ),
            (
                "x_unittest".to_string(),
                vec![
                    "core".to_string(),
                    // With dev-deps propagation enabled, this would be included
                    // "dep1".to_string(),
                    "dep2".to_string(),
                    "x".to_string()
                ]
            ),
        ])
    );
}

#[test]
fn dev_dep_plugins_are_not_propagated_for_ws_members() {
    let t = TempDir::new().unwrap();

    let m = t.child("m");
    CairoPluginProjectBuilder::default().name("m").build(&m);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .dep_cairo_test()
        .dev_dep("m", &m)
        .build(&dep2);

    let pkg = t.child("pkg");
    ProjectBuilder::start()
        .name("x")
        .dep("dep2", &dep2)
        .build(&pkg);

    WorkspaceBuilder::start()
        .add_member("dep2")
        .add_member("pkg")
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        units_and_plugins(metadata.clone()),
        BTreeMap::from_iter(vec![
            ("dep2".to_string(), vec![]),
            (
                "dep2_unittest".to_string(),
                vec!["cairo_test".to_string(), "m".to_string()]
            ),
            ("m".to_string(), vec![]),
            ("x".to_string(), vec![]),
            ("x_unittest".to_string(), vec![]),
        ])
    );

    // Get the compilation unit of `x` unit tests.
    let cu = metadata
        .compilation_units
        .iter()
        .find(|unit| unit.target.name == "x_unittest")
        .unwrap();
    // Get dependencies of components in the compilation unit.
    let component_deps = BTreeMap::from_iter(cu.components.iter().map(|component| {
        (
            component.name.clone(),
            component
                .dependencies
                .as_ref()
                .map(|deps| {
                    deps.iter()
                        .map(|dep| {
                            metadata
                                .packages
                                .iter()
                                .find(|p| p.id.to_string() == dep.id.clone().to_string())
                                .map(|p| p.name.clone())
                                .unwrap()
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap(),
        )
    }));
    assert_eq!(
        component_deps,
        BTreeMap::from_iter(vec![
            ("core".to_string(), vec!["core".to_string()]),
            (
                "dep2".to_string(),
                vec![
                    "core".to_string(),
                    "dep2".to_string(),
                    // Note that `cairo_test` is indeed a dev dependency of `dep2`,
                    // but it's not propagated to the unit tests of `x`.
                    // Only dev dependencies of the main component should be enabled.
                    // "cairo_test".to_string(),
                    // "m".to_string()
                ]
            ),
            (
                "x".to_string(),
                vec!["core".to_string(), "dep2".to_string(), "x".to_string()]
            )
        ])
    );
}

#[test]
fn no_dep() {
    let t = TempDir::new().unwrap();
    create_local_dependencies_setup(&t);
    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(meta),
        BTreeMap::from_iter([(
            "x".to_string(),
            vec![
                "cairo_test".to_string(),
                "core".to_string(),
                "y".to_string()
            ]
        )])
    );
}

#[test]
fn manifest_targets_and_metadata() {
    let t = TempDir::new().unwrap();
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
            readme = "./README.md"

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
    t.child("README.md").touch().unwrap();
    t.child("license.md").touch().unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
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
            .license_file(
                fsx::canonicalize_utf8(t.join("license.md"))
                    .unwrap()
                    .into_string()
            )
            .readme(
                fsx::canonicalize_utf8(t.join("README.md"))
                    .unwrap()
                    .into_string()
            )
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
    let t = TempDir::new().unwrap();
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
        .arg("--json")
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
            ("cairo_test".to_string(), None),
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
    let t = TempDir::new().unwrap();
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
        .stdout_eq("{\"version\":1,[..]}\n");
}

#[test]
fn workspace_simple() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep_cairo_test()
        .manifest_extra("[[test]]")
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep_cairo_test()
        .manifest_extra("[[test]]")
        // Check paths are relative to manifest file.
        .dep("first", Dep.path("../first"))
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version=1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(metadata),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            ("cairo_test".to_string(), vec![]),
            (
                "first".to_string(),
                vec!["cairo_test".to_string(), "core".to_string(),]
            ),
            (
                "second".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "first".to_string(),
                ]
            ),
        ])
    )
}

#[test]
fn workspace_with_root() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep_cairo_test()
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep_cairo_test()
        .dep("first", Dep.path("../first"))
        .build(&pkg2);
    let root = ProjectBuilder::start()
        .name("some_root")
        .dep_cairo_test()
        .dep("first", Dep.path("./first"))
        .dep("second", Dep.path("./second"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(root)
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version=1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(metadata),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            (
                "some_root".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "first".to_string(),
                    "second".to_string(),
                ]
            ),
            (
                "first".to_string(),
                vec!["cairo_test".to_string(), "core".to_string(),]
            ),
            (
                "second".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "first".to_string(),
                ]
            ),
            ("cairo_test".to_string(), vec![]),
        ])
    )
}

#[test]
fn workspace_as_dep() {
    let t = TempDir::new().unwrap();
    let first_t = t.child("first_workspace");
    let pkg1 = first_t.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep_cairo_test()
        .manifest_extra("[[test]]")
        .build(&pkg1);
    let pkg2 = first_t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep_cairo_test()
        .manifest_extra("[[test]]")
        .dep("first", Dep.path("../first"))
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&first_t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version=1"])
        .current_dir(&first_t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(metadata),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            ("cairo_test".to_string(), vec![]),
            (
                "first".to_string(),
                vec!["cairo_test".to_string(), "core".to_string(),]
            ),
            (
                "second".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "first".to_string(),
                ]
            ),
        ])
    );

    let second_t = t.child("second_workspace");
    let pkg1 = second_t.child("third");
    ProjectBuilder::start()
        .name("third")
        .manifest_extra("[[test]]")
        .dep_cairo_test()
        .dep("first", Dep.path("../../first_workspace"))
        .dep("second", Dep.path("../../first_workspace"))
        .build(&pkg1);
    let pkg2 = second_t.child("fourth");
    ProjectBuilder::start()
        .name("fourth")
        .manifest_extra("[[test]]")
        .dep_cairo_test()
        .dep("third", Dep.path("../third"))
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("third")
        .add_member("fourth")
        .build(&second_t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version=1"])
        .current_dir(&second_t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(metadata),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            ("cairo_test".to_string(), vec![]),
            (
                "first".to_string(),
                vec!["cairo_test".to_string(), "core".to_string(),]
            ),
            (
                "second".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "first".to_string(),
                ]
            ),
            (
                "third".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "first".to_string(),
                    "second".to_string(),
                ]
            ),
            (
                "fourth".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "third".to_string()
                ]
            ),
        ])
    );
}

#[test]
fn workspace_package_key_inheritance() {
    let t = TempDir::new().unwrap();

    let some_dep = t.child("some_dep");
    ProjectBuilder::start()
        .name("some_dep")
        .dep_cairo_test()
        .manifest_extra("[[test]]")
        .version("0.1.0")
        .build(&some_dep);

    let some_workspace = t.child("some_workspace");
    let pkg1 = some_workspace.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep_cairo_test()
        .manifest_extra("[[test]]")
        .dep("some_dep", Dep.workspace())
        .build(&pkg1);
    let pkg2 = some_workspace.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep_cairo_test()
        .manifest_extra("[[test]]")
        .dep("first", Dep.path("../first"))
        .build(&pkg2);

    WorkspaceBuilder::start()
        .dep("some_dep", Dep.path("../some_dep"))
        .add_member("first")
        .add_member("second")
        .build(&some_workspace);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version=1"])
        .current_dir(&some_workspace)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_and_deps(metadata),
        BTreeMap::from_iter([
            ("core".to_string(), vec!["cairo_test".to_string()]),
            ("cairo_test".to_string(), vec![]),
            (
                "first".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "some_dep".to_string(),
                ]
            ),
            (
                "second".to_string(),
                vec![
                    "cairo_test".to_string(),
                    "core".to_string(),
                    "first".to_string(),
                ]
            ),
            (
                "some_dep".to_string(),
                vec!["cairo_test".to_string(), "core".to_string(),]
            )
        ])
    )
}

#[test]
fn infer_readme_simple() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_by_name(meta)
            .get("hello")
            .unwrap()
            .manifest_metadata
            .readme,
        None
    );

    t.child("README").touch().unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_by_name(meta)
            .get("hello")
            .unwrap()
            .manifest_metadata
            .readme,
        Some(
            fsx::canonicalize_utf8(t.join("README"))
                .unwrap()
                .into_string()
        )
    );

    t.child("README.txt").touch().unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_by_name(meta)
            .get("hello")
            .unwrap()
            .manifest_metadata
            .readme,
        Some(
            fsx::canonicalize_utf8(t.join("README.txt"))
                .unwrap()
                .into_string()
        )
    );

    t.child("README.md").touch().unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_by_name(meta)
            .get("hello")
            .unwrap()
            .manifest_metadata
            .readme,
        Some(
            fsx::canonicalize_utf8(t.join("README.md"))
                .unwrap()
                .into_string()
        )
    );

    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "1.0.0"
            readme = "a/b/c/MEREAD.md"
            "#,
        )
        .unwrap();
    t.child("a").child("b").child("c").create_dir_all().unwrap();
    t.child("a")
        .child("b")
        .child("c")
        .child("MEREAD.md")
        .touch()
        .unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_by_name(meta)
            .get("hello")
            .unwrap()
            .manifest_metadata
            .readme,
        Some(
            fsx::canonicalize_utf8(t.join("a/b/c/MEREAD.md"))
                .unwrap()
                .into_string()
        )
    );
}

#[test]
fn infer_readme_simple_bool() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "1.0.0"
            readme = false
            "#,
        )
        .unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_by_name(meta)
            .get("hello")
            .unwrap()
            .manifest_metadata
            .readme,
        None
    );

    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "1.0.0"
            readme = true
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            {"type":"error","message":"failed to parse manifest at: [..]/Scarb.toml[..]Caused by:[..]failed to find readme at [..]/README.md[..]"}
        "#});

    t.child("README.md").touch().unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(
        packages_by_name(meta)
            .get("hello")
            .unwrap()
            .manifest_metadata
            .readme,
        Some(
            fsx::canonicalize_utf8(t.join("README.md"))
                .unwrap()
                .into_string()
        )
    );
}

#[test]
fn infer_readme_workspace() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);
    let ws = ["t1", "t2", "t3", "t4", "t5", "t6"].iter().zip(
        [
            Some("MEREAD.md"),
            Some("README.md"),
            Some("README.txt"),
            Some("TEST.txt"),
            None,
            None,
        ]
        .iter(),
    );
    for (pack_name, readme_name) in ws {
        Scarb::quick_snapbox()
            .arg("new")
            .arg(pack_name)
            .current_dir(&t);
        if let Some(name) = readme_name {
            t.child(pack_name).child(name).touch().unwrap();
        }
    }
    t.child("MEREAD.md").touch().unwrap();
    t.child("tmp1").create_dir_all().unwrap();
    t.child("tmp1").child("tmp2").create_dir_all().unwrap();
    Scarb::quick_snapbox()
        .arg("new")
        .arg("t7")
        .current_dir(t.child("tmp1").child("tmp2"));
    t.child("tmp1")
        .child("tmp2")
        .child("t7")
        .child("MEREAD.md")
        .touch()
        .unwrap();
    t.child("tmp1")
        .child("tmp2")
        .child("t7")
        .child("Scarb.toml")
        .write_str(
            r#"
        [package]
        name = "t7"
        version.workspace = true
        edition = "2023_10"
        readme.workspace = true
    "#,
        )
        .unwrap();

    t.child("Scarb.toml")
        .write_str(
            r#"
            [workspace]
            members = [
                "t1",
                "t2",
                "t3",
                "t4",
                "t5",
                "t6",
                "tmp1/tmp2/t7",
            ]

            [workspace.package]
            version = "0.1.0"
            edition = "2023_10"
            readme = "MEREAD.md"

            [package]
            name = "hello"
            version.workspace = true
            readme.workspace = true
        "#,
        )
        .unwrap();

    t.child("t1")
        .child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "t1"
            version.workspace = true
            edition = "2023_10"
            readme.workspace = true
    "#,
        )
        .unwrap();
    t.child("t2")
        .child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "t2"
            version.workspace = true
            edition = "2023_10"
            readme = true
    "#,
        )
        .unwrap();
    t.child("t3")
        .child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "t3"
            version.workspace = true
            edition = "2023_10"
    "#,
        )
        .unwrap();
    t.child("t4")
        .child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "t4"
            version.workspace = true
            edition = "2023_10"
            readme = "TEST.txt"
    "#,
        )
        .unwrap();
    t.child("t5")
        .child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "t5"
            version.workspace = true
            edition = "2023_10"
            readme = false
    "#,
        )
        .unwrap();
    t.child("t6")
        .child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "t6"
            version.workspace = true
            edition = "2023_10"
    "#,
        )
        .unwrap();
    t.child("tmp1")
        .child("tmp2")
        .child("t7")
        .child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "t7"
            version.workspace = true
            edition = "2023_10"
            readme.workspace = true
    "#,
        )
        .unwrap();

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let packages = packages_by_name(meta);
    assert_eq!(
        packages.get("hello").unwrap().manifest_metadata.readme,
        Some(
            fsx::canonicalize_utf8(t.join("MEREAD.md"))
                .unwrap()
                .into_string()
        )
    );
    assert_eq!(
        packages.get("t7").unwrap().manifest_metadata.readme,
        Some(
            fsx::canonicalize_utf8(t.join("MEREAD.md"))
                .unwrap()
                .into_string()
        )
    );
    assert_eq!(
        packages.get("t1").unwrap().manifest_metadata.readme,
        Some(
            fsx::canonicalize_utf8(t.join("MEREAD.md"))
                .unwrap()
                .into_string()
        )
    );
    assert_eq!(
        packages.get("t2").unwrap().manifest_metadata.readme,
        Some(
            fsx::canonicalize_utf8(t.child("t2").join("README.md"))
                .unwrap()
                .into_string()
        )
    );
    assert_eq!(
        packages.get("t3").unwrap().manifest_metadata.readme,
        Some(
            fsx::canonicalize_utf8(t.child("t3").join("README.txt"))
                .unwrap()
                .into_string()
        )
    );
    assert_eq!(
        packages.get("t4").unwrap().manifest_metadata.readme,
        Some(
            fsx::canonicalize_utf8(t.child("t4").join("TEST.txt"))
                .unwrap()
                .into_string()
        )
    );
    assert_eq!(packages.get("t5").unwrap().manifest_metadata.readme, None);
    assert_eq!(packages.get("t6").unwrap().manifest_metadata.readme, None);
}

#[test]
fn includes_edition() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .edition("2023_10")
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    for package in metadata.packages {
        if package.name == "hello" {
            assert_eq!(package.edition, Some("2023_10".to_string()));
            return;
        }
    }
    panic!("Package not found in metadata!");
}

#[test]
fn includes_experimental_features() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_package_extra(
            r#"experimental-features = ["negative_impls", "associated_item_constraints"]"#,
        )
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let packages = packages_by_name(metadata);

    assert!(
        packages
            .get("hello")
            .unwrap()
            .clone()
            .experimental_features
            .contains(&String::from("negative_impls"))
    );

    assert!(
        packages
            .get("hello")
            .unwrap()
            .clone()
            .experimental_features
            .contains(&String::from("associated_item_constraints"))
    );
}

#[test]
fn prebuilt_plugins_disallowed_by_default() {
    let t = TempDir::new().unwrap();

    CairoPluginProjectBuilder::default()
        .name("q")
        .scarb_project(|builder| {
            builder
                .name("q")
                .version("1.0.0")
                .manifest_extra("[cairo-plugin]")
        })
        .build(&t.child("q"));
    ProjectBuilder::start()
        .name("y")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { z::f() }")
        .dep_cairo_test()
        .dep("q", Dep.path("../q"))
        .build(&t.child("y"));
    ProjectBuilder::start()
        .name("x")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { y::f() }")
        .dep_cairo_test()
        .dep("y", Dep.path("y"))
        .dep("q", Dep.path("q"))
        .build(&t);

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.name == "x")
        .unwrap();

    assert_eq!(cu.cairo_plugins.len(), 1);
    assert!(cu.cairo_plugins[0].package.repr.starts_with("q"));
    assert!(!cu.cairo_plugins[0].prebuilt_allowed.unwrap());
}

#[test]
fn can_allow_prebuilt_plugins_for_subtree() {
    let t = TempDir::new().unwrap();

    CairoPluginProjectBuilder::default()
        .name("q")
        .scarb_project(|builder| {
            builder
                .name("q")
                .version("1.0.0")
                .manifest_extra("[cairo-plugin]")
        })
        .build(&t.child("q"));

    ProjectBuilder::start()
        .name("z")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { q::f() }")
        .dep_cairo_test()
        .dep("q", Dep.path("../q"))
        .build(&t.child("z"));

    ProjectBuilder::start()
        .name("y")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { z::f() }")
        .dep_cairo_test()
        .dep("z", Dep.path("../z"))
        .build(&t.child("y"));

    ProjectBuilder::start()
        .name("x")
        .version("1.0.0")
        .lib_cairo(r"fn f() -> felt252 { y::f() }")
        .manifest_extra(indoc! {r#"
            [tool.scarb]
            allow-prebuilt-plugins = ["y"]
        "#})
        .dep_cairo_test()
        .dep("z", Dep.path("z"))
        .dep("y", Dep.path("y"))
        .build(&t);

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.name == "x")
        .unwrap();
    assert_eq!(cu.cairo_plugins.len(), 1);
    assert!(cu.cairo_plugins[0].package.repr.starts_with("q"));
    assert!(cu.cairo_plugins[0].prebuilt_allowed.unwrap());
}

#[test]
fn executable_target_can_allow_syscalls() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("executable_test")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            allow-syscalls = true
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);
    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let pkg = meta
        .packages
        .iter()
        .find(|p| p.name == "executable_test")
        .unwrap();
    let target = pkg.targets.first().unwrap();
    assert!(
        target
            .params
            .as_object()
            .unwrap()
            .get("allow-syscalls")
            .unwrap()
            .as_bool()
            .unwrap(),
        "syscalls not allowed"
    );
}

#[test]
fn executable_target_can_compile_to_sierra() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("executable_test")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            name = "first"
            sierra = true
            function = "executable_test::first"

            [[target.executable]]
            name = "second"
            sierra = true
            function = "executable_test::second"
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn first() -> felt252 {
                42
            }
            #[executable]
            fn second() -> felt252 {
                42
            }
        "#})
        .build(&t);
    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let pkg = meta
        .packages
        .iter()
        .find(|p| p.name == "executable_test")
        .unwrap();
    let targets = pkg
        .targets
        .iter()
        .filter(|t| t.kind == "executable")
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 2);
    for t in targets {
        assert!(
            t.params
                .as_object()
                .unwrap()
                .get("sierra")
                .unwrap()
                .as_bool()
                .unwrap(),
        );
    }
}

#[test]
fn cairo_plugins_added_as_component_dependencies() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("world")
        .manifest_extra(indoc! {r#"
            [cairo-plugin]
            # Stop Scarb from attempting to compile the plugin with Cargo
            builtin = true
        "#})
        .build(&t.child("world"));
    ProjectBuilder::start()
        .name("beautiful")
        .dep("world", t.child("world"))
        .build(&t.child("beautiful"));
    ProjectBuilder::start()
        .name("hello")
        .dep("beautiful", t.child("beautiful"))
        .build(&t.child("hello"));

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(t.child("hello"))
        .stdout_json::<Metadata>();
    let cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.name == "hello" && cu.target.kind == "lib")
        .unwrap();

    assert_eq!(
        cu.cairo_plugins.len(),
        1,
        "CU should contain exactly one plugin"
    );
    assert!(
        cu.cairo_plugins
            .first()
            .unwrap()
            .package
            .to_string()
            .starts_with("world 1.0.0"),
        "plugin world not found in cu plugins"
    );
    assert_eq!(
        cu.components.len(),
        3,
        "CU should contain exactly three components"
    );
    let beautiful_component = cu
        .components
        .iter()
        .find(|c| c.name == "beautiful")
        .unwrap();
    assert_eq!(
        beautiful_component.dependencies.clone().unwrap().len(),
        3,
        "beautiful component should have 3 dependencies"
    );
    let beautiful_deps = beautiful_component
        .dependencies
        .clone()
        .unwrap()
        .into_iter()
        .map(|dep| dep.id.to_string())
        .collect_vec();
    let beautiful_deps_packages = meta
        .packages
        .iter()
        .filter(|pkg| beautiful_deps.iter().contains(&pkg.id.to_string()))
        .map(|pkg| pkg.name.to_string())
        .sorted()
        .collect_vec();
    assert_eq!(
        beautiful_deps_packages.len(),
        beautiful_deps.len(),
        "all dependencies should be found in the packages list"
    );
    assert_eq!(
        beautiful_deps_packages,
        vec!["beautiful", "core", "world"],
        "beautiful component invalid dependencies"
    );
    let hello_component = cu.components.iter().find(|c| c.name == "hello").unwrap();
    assert_eq!(
        hello_component.dependencies.clone().unwrap().len(),
        3,
        "hello component should have 3 dependencies"
    );
    let hello_deps = hello_component
        .dependencies
        .clone()
        .unwrap()
        .into_iter()
        .map(|dep| dep.id.to_string())
        .collect_vec();
    let hello_deps_packages = meta
        .packages
        .iter()
        .filter(|pkg| hello_deps.iter().contains(&pkg.id.to_string()))
        .map(|pkg| pkg.name.to_string())
        .sorted()
        .collect_vec();
    assert_eq!(
        hello_deps_packages.len(),
        hello_deps.len(),
        "all dependencies should be found in the packages list"
    );
    // Note, hello does not depend on world.
    assert_eq!(
        hello_deps_packages,
        vec!["beautiful", "core", "hello"],
        "beautiful component invalid dependencies"
    );
}

#[test]
fn compiler_config_collected_properly() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
         [profile.dev.cairo]
         sierra-replace-ids = false

         [cairo]
         inlining-strategy = "avoid"
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let cu = metadata
        .compilation_units
        .iter()
        .find(|cu| &cu.target.kind == "lib")
        .unwrap();

    assert_eq!(
        cu.compiler_config,
        json!({
            "allow_warnings": true,
            "enable_gas": true,
            "inlining_strategy": "avoid",
            "sierra_replace_ids": false,
            "unstable_add_statements_code_locations_debug_info": false,
            "unstable_add_statements_functions_debug_info": false,
            "panic_backtrace": false,"unsafe_panic": false,
            "incremental": true
        })
    );
}

#[test]
fn compiler_config_collected_properly_in_workspace() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [cairo]
            sierra-replace-ids = false
        "#})
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .manifest_extra(indoc! {r#"
            [cairo]
            inlining-strategy = "default"
        "#})
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(indoc! {r#"
             [profile.dev.cairo]
             enable-gas = false

             [cairo]
             inlining-strategy = "avoid"
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version=1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let cu = metadata
        .compilation_units
        .iter()
        .find(|cu| &cu.target.kind == "lib" && cu.components[1].name == "first")
        .unwrap();

    assert_eq!(
        cu.compiler_config,
        json!({
            "allow_warnings": true,
            "enable_gas": false,
            "inlining_strategy": "avoid",
            "sierra_replace_ids": true,
            "unstable_add_statements_code_locations_debug_info": false,
            "unstable_add_statements_functions_debug_info": false,
            "panic_backtrace": false,"unsafe_panic": false,
            "incremental": true
        })
    );
}

#[test]
fn profile_can_override_cairo_section() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
         [profile.dev.cairo]
         inlining-strategy = "default"

         [cairo]
         inlining-strategy = "avoid"
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let cu = metadata
        .compilation_units
        .iter()
        .find(|cu| &cu.target.kind == "lib")
        .unwrap();

    assert_eq!(
        cu.compiler_config,
        json!({
            "allow_warnings": true,
            "enable_gas": true,
            "inlining_strategy": "default",
            "sierra_replace_ids": true,
            "unstable_add_statements_code_locations_debug_info": false,
            "unstable_add_statements_functions_debug_info": false,
            "panic_backtrace": false,"unsafe_panic": false,
            "incremental": true
        })
    );
}

#[test]
fn cairo_section_overrides_profile_defaults() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
         [profile.some]
         inherits = "release"

         [cairo]
         sierra-replace-ids = true
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let cu = metadata
        .compilation_units
        .iter()
        .find(|cu| &cu.target.kind == "lib")
        .unwrap();

    assert_eq!(
        cu.compiler_config,
        json!({
            "allow_warnings": true,
            "enable_gas": true,
            "inlining_strategy": "default",
            "sierra_replace_ids": true,
            "unstable_add_statements_code_locations_debug_info": false,
            "unstable_add_statements_functions_debug_info": false,
            "panic_backtrace": false,
            "unsafe_panic": false,
            "incremental": true
        })
    );
}

#[test]
fn can_specify_inlining_strategy_by_weight() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
         [cairo]
         inlining-strategy = 12
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let cu = metadata
        .compilation_units
        .iter()
        .find(|cu| &cu.target.kind == "lib")
        .unwrap();

    assert_eq!(
        cu.compiler_config,
        json!({
            "allow_warnings": true,
            "enable_gas": true,
            "inlining_strategy": 12,
            "sierra_replace_ids": true,
            "unstable_add_statements_code_locations_debug_info": false,
            "unstable_add_statements_functions_debug_info": false,
            "panic_backtrace": false,
            "unsafe_panic": false,
            "incremental": true
        })
    );
}

#[test]
fn cannot_specify_not_predefined_inlining_strategy() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
         [cairo]
         inlining-strategy = "super-cool"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: failed to parse manifest at: [..]Scarb.toml
            
            Caused by:
                TOML parse error at line 9, column 21
                  |
                9 | inlining-strategy = "super-cool"
                  |                     ^^^^^^^^^^^^
                unknown inlining strategy: `super-cool`
                use one of: `default`, `avoid` or a number
        "#});
}
