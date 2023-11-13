use std::collections::BTreeMap;

use assert_fs::prelude::*;
use indoc::indoc;
use serde_json::json;

use scarb_metadata::{Cfg, ManifestMetadataBuilder, Metadata, PackageMetadata};
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

#[test]
fn simple() {
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
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    assert_eq!(
        packages_and_deps(meta),
        BTreeMap::from_iter([
            ("core".to_string(), vec![]),
            ("test_plugin".to_string(), vec![]),
            (
                "q".to_string(),
                vec!["core".to_string(), "test_plugin".to_string()]
            ),
            (
                "x".to_string(),
                vec![
                    "core".to_string(),
                    "test_plugin".to_string(),
                    "y".to_string()
                ]
            ),
            (
                "y".to_string(),
                vec![
                    "core".to_string(),
                    "q".to_string(),
                    "test_plugin".to_string(),
                    "z".to_string()
                ]
            ),
            (
                "z".to_string(),
                vec![
                    "core".to_string(),
                    "q".to_string(),
                    "test_plugin".to_string()
                ]
            ),
        ])
    )
}

#[test]
fn no_dep() {
    let t = assert_fs::TempDir::new().unwrap();
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
                "core".to_string(),
                "test_plugin".to_string(),
                "y".to_string()
            ]
        )])
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
            ("test_plugin".to_string(), None),
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

#[test]
fn workspace_simple() {
    let t = assert_fs::TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra("[[test]]")
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
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
            ("core".to_string(), vec![]),
            ("test_plugin".to_string(), vec![]),
            (
                "first".to_string(),
                vec!["core".to_string(), "test_plugin".to_string()]
            ),
            (
                "second".to_string(),
                vec![
                    "core".to_string(),
                    "first".to_string(),
                    "test_plugin".to_string()
                ]
            ),
        ])
    )
}

#[test]
fn workspace_with_root() {
    let t = assert_fs::TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep("first", Dep.path("../first"))
        .build(&pkg2);
    let root = ProjectBuilder::start()
        .name("some_root")
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
            ("core".to_string(), vec![]),
            (
                "some_root".to_string(),
                vec![
                    "core".to_string(),
                    "first".to_string(),
                    "second".to_string(),
                    "test_plugin".to_string()
                ]
            ),
            (
                "first".to_string(),
                vec!["core".to_string(), "test_plugin".to_string()]
            ),
            (
                "second".to_string(),
                vec![
                    "core".to_string(),
                    "first".to_string(),
                    "test_plugin".to_string(),
                ]
            ),
            ("test_plugin".to_string(), vec![]),
        ])
    )
}

#[test]
fn workspace_as_dep() {
    let t = assert_fs::TempDir::new().unwrap();
    let first_t = t.child("first_workspace");
    let pkg1 = first_t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra("[[test]]")
        .build(&pkg1);
    let pkg2 = first_t.child("second");
    ProjectBuilder::start()
        .name("second")
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
            ("test_plugin".to_string(), vec![]),
            ("core".to_string(), vec![]),
            (
                "first".to_string(),
                vec!["core".to_string(), "test_plugin".to_string()]
            ),
            (
                "second".to_string(),
                vec![
                    "core".to_string(),
                    "first".to_string(),
                    "test_plugin".to_string()
                ]
            ),
        ])
    );

    let second_t = t.child("second_workspace");
    let pkg1 = second_t.child("third");
    ProjectBuilder::start()
        .name("third")
        .manifest_extra("[[test]]")
        .dep("first", Dep.path("../../first_workspace"))
        .dep("second", Dep.path("../../first_workspace"))
        .build(&pkg1);
    let pkg2 = second_t.child("fourth");
    ProjectBuilder::start()
        .name("fourth")
        .manifest_extra("[[test]]")
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
            ("core".to_string(), vec![]),
            ("test_plugin".to_string(), vec![]),
            (
                "first".to_string(),
                vec!["core".to_string(), "test_plugin".to_string()]
            ),
            (
                "second".to_string(),
                vec![
                    "core".to_string(),
                    "first".to_string(),
                    "test_plugin".to_string()
                ]
            ),
            (
                "third".to_string(),
                vec![
                    "core".to_string(),
                    "first".to_string(),
                    "second".to_string(),
                    "test_plugin".to_string()
                ]
            ),
            (
                "fourth".to_string(),
                vec![
                    "core".to_string(),
                    "test_plugin".to_string(),
                    "third".to_string()
                ]
            ),
        ])
    );
}

#[test]
fn workspace_package_key_inheritance() {
    let t = assert_fs::TempDir::new().unwrap();

    let some_dep = t.child("some_dep");
    ProjectBuilder::start()
        .name("some_dep")
        .manifest_extra("[[test]]")
        .version("0.1.0")
        .build(&some_dep);

    let some_workspace = t.child("some_workspace");
    let pkg1 = some_workspace.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra("[[test]]")
        .dep("some_dep", Dep.workspace())
        .build(&pkg1);
    let pkg2 = some_workspace.child("second");
    ProjectBuilder::start()
        .name("second")
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
            ("core".to_string(), vec![]),
            ("test_plugin".to_string(), vec![]),
            (
                "first".to_string(),
                vec![
                    "core".to_string(),
                    "some_dep".to_string(),
                    "test_plugin".to_string()
                ]
            ),
            (
                "second".to_string(),
                vec![
                    "core".to_string(),
                    "first".to_string(),
                    "test_plugin".to_string()
                ]
            ),
            (
                "some_dep".to_string(),
                vec!["core".to_string(), "test_plugin".to_string()]
            )
        ])
    )
}

#[test]
fn infer_readme_simple() {
    let t = assert_fs::TempDir::new().unwrap();
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
    let t = assert_fs::TempDir::new().unwrap();
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
        .stdout_matches(indoc! {r#"
            {"type":"error","message":"failed to parse manifest at: [..]/Scarb.toml[..]Caused by:[..]failed to find the file at [..]/README.md[..]"}
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
    let t = assert_fs::TempDir::new().unwrap();
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
    let t = assert_fs::TempDir::new().unwrap();
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
