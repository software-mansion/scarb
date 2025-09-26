use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn assets_are_copied() {
    let t = TempDir::new().unwrap();

    t.child("data.txt")
        .write_str("“Marek, skup się na pracy.” ~ Marcin Skotniczny")
        .unwrap();

    ProjectBuilder::start()
        .name("foobar")
        .manifest_package_extra(r#"assets = ["data.txt"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target")
        .child("dev")
        .child("data.txt")
        .assert(predicates::path::is_file());
}

#[test]
fn asset_from_dependency_is_copied() {
    let t = TempDir::new().unwrap();

    // Dependency package with an asset.
    t.child("dep/assets").create_dir_all().unwrap();
    t.child("dep/assets/data.txt")
        .write_str("Hello from dependency!")
        .unwrap();
    ProjectBuilder::start()
        .name("dep")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["assets/data.txt"]"#)
        .build(&t.child("dep"));

    // Root package depending on `dep` and not declaring any assets itself.
    ProjectBuilder::start()
        .name("app")
        .version("0.1.0")
        .dep("dep", Dep.path("../dep"))
        .build(&t.child("app"));

    // Workspace with both members.
    WorkspaceBuilder::start()
        .add_member("dep")
        .add_member("app")
        .build(&t);

    // Build only `app` to ensure the asset is pulled via dependency graph.
    Scarb::quick_snapbox()
        .arg("build")
        .arg("-p")
        .arg("app")
        .current_dir(&t)
        .assert()
        .success();

    // The asset from the dependency should be copied to the workspace target dir of `app`.
    t.child("target")
        .child("dev")
        .child("data.txt")
        .assert(predicates::path::is_file());
}

#[test]
fn asset_directory_is_error() {
    let t = TempDir::new().unwrap();

    t.child("assets").create_dir_all().unwrap();

    ProjectBuilder::start()
        .name("badpkg")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["assets/"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            [..] Compiling badpkg v0.1.0 ([..])
            error: package `badpkg v0.1.0 ([..])` asset is not a file: [..]/assets
        "#});
}

#[test]
fn duplicate_asset_names_within_package_error() {
    let t = TempDir::new().unwrap();

    t.child("a").create_dir_all().unwrap();
    t.child("b").create_dir_all().unwrap();
    t.child("a/file.txt").write_str("A").unwrap();
    t.child("b/file.txt").write_str("B").unwrap();

    ProjectBuilder::start()
        .name("dupsame")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["a/file.txt", "b/file.txt"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            [..] Compiling dupsame v0.1.0 ([..])
            error: package `dupsame v0.1.0 ([..])` declares multiple assets with the same file name: file.txt
        "#});
}

#[test]
fn missing_asset() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("missing")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["data.txt"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            [..] Compiling missing v0.1.0 ([..])
            error: failed to find asset file at [..]/data.txt

            Caused by:
                0: failed to get absolute path of `[..]/data.txt`
                1: [..]
        "#});
}

#[test]
fn duplicate_asset_names_between_dependencies_error() {
    let t = TempDir::new().unwrap();

    // dep1 with an asset named `common.txt`.
    t.child("dep1/assets").create_dir_all().unwrap();
    t.child("dep1/assets/common.txt").write_str("A").unwrap();
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["assets/common.txt"]"#)
        .build(&t.child("dep1"));

    // dep2 with an asset named `common.txt` as well.
    t.child("dep2/assets").create_dir_all().unwrap();
    t.child("dep2/assets/common.txt").write_str("B").unwrap();
    ProjectBuilder::start()
        .name("dep2")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["assets/common.txt"]"#)
        .build(&t.child("dep2"));

    // Root package depending on both deps.
    ProjectBuilder::start()
        .name("app")
        .version("0.1.0")
        .dep("dep1", Dep.path("../dep1"))
        .dep("dep2", Dep.path("../dep2"))
        .build(&t.child("app"));

    // Create a workspace tying members together.
    WorkspaceBuilder::start()
        .add_member("dep1")
        .add_member("dep2")
        .add_member("app")
        .build(&t);

    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("build")
        .arg("-p")
        .arg("app")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..] Compiling app v0.1.0 ([..])
            error: multiple packages declare an asset with the same file name `common.txt`: dep2 [..], dep1 [..]
        "#});
}
