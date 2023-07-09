use assert_fs::prelude::*;
use assert_fs::TempDir;
use itertools::Itertools;
use scarb_metadata::Metadata;

use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn read_no_root() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start().name("second").build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["metadata", "--format-version=1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let packages = metadata
        .packages
        .iter()
        .map(|p| p.name.clone())
        .map(String::from)
        .sorted()
        .collect::<Vec<_>>();

    assert_eq!(packages[0], "core");
    assert_eq!(packages[1], "first");
    assert_eq!(packages[2], "second");
    assert_eq!(packages.len(), 3);
}

#[test]
fn read_with_root() {
    let t = TempDir::new().unwrap().child("test_root");
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start().name("second").build(&pkg2);
    let root = ProjectBuilder::start().name("some_root");
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(root)
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["metadata", "--format-version=1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let packages = metadata
        .packages
        .iter()
        .map(|p| p.name.clone())
        .map(String::from)
        .sorted()
        .collect::<Vec<_>>();

    assert_eq!(packages[0], "core");
    assert_eq!(packages[1], "first");
    assert_eq!(packages[2], "second");
    assert_eq!(packages[3], "some_root");
    assert_eq!(packages.len(), 4);
}

#[test]
fn build_with_root() {
    let t = TempDir::new().unwrap().child("test_root");
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start().name("second").build(&pkg2);
    let root = ProjectBuilder::start().name("some_root");
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(root)
        .build(&t);

    Scarb::quick_snapbox()
        .args(["build"])
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec!["first.sierra", "second.sierra", "some_root.sierra"]
    );
}
