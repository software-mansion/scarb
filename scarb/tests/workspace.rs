use assert_fs::fixture::{PathChild, PathCreateDir};
use assert_fs::TempDir;
use indoc::indoc;

use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn warn_on_member_without_manifest() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    t.child("second").create_dir_all().unwrap();
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(
            "warn: workspace members definition matched path `[..]`, \
        which misses a manifest file\n",
        );
}

#[test]
fn error_on_virtual_manifest_with_dependencies() {
    let t = TempDir::new().unwrap();
    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
            [dependencies]
            foo = "1.0.0"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]

            Caused by:
                this virtual manifest specifies a [dependencies] section, which is not allowed
                help: use [workspace.dependencies] instead
        "#});
}

#[test]
fn unify_target_dir() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    WorkspaceBuilder::start().add_member("first").build(&t);

    // Make sure target dir is created.
    Scarb::quick_snapbox()
        .args(["build"])
        .current_dir(&pkg1)
        .assert()
        .success();

    let root_metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let pkg_metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&pkg1)
        .stdout_json::<Metadata>();

    assert_eq!(root_metadata.target_dir, pkg_metadata.target_dir);
    assert_eq!(
        fsx::canonicalize(
            root_metadata
                .target_dir
                .unwrap()
                .to_owned()
                .into_std_path_buf()
        )
        .unwrap(),
        fsx::canonicalize(t.child("target")).unwrap()
    );
}

#[test]
fn target_name_duplicate() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
        [[target.starknet-contract]]
        name = "hello"
        "#})
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .manifest_extra(indoc! {r#"
        [[target.starknet-contract]]
        name = "hello"
        "#})
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: workspace contains duplicate target definitions `starknet-contract (hello)`
            help: use different target names to resolve the conflict
        "#});
}

#[test]
fn target_group_id_duplicates_target_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
        [[target.starknet-contract]]
        name = "hello"
        group-id = "hello"
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
           error: the group id `hello` of target `starknet-contract` duplicates target name
           help: use different group name to resolve the conflict
        "#});
}
