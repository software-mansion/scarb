//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

mod markdown_target;
use markdown_target::MarkdownTargetChecker;

const TARGET_PACKAGE: &str = include_str!("code/code_9.cairo");
const FOREIGN_PACKAGE: &str = include_str!("code/code_10.cairo");

const EXPECTED_ROOT_TARGET_PACKAGE_PATH: &str = "tests/data/hello_world_workspaces_target_package";

#[test]
fn test_reeksports_in_multiple_workspaces() {
    let root_dir = TempDir::new().unwrap();

    let foreign_child_dir = root_dir.child("foreign_package");
    let target_child_dir = root_dir.child("target_package");

    ProjectBuilder::start()
        .name("foreign_package")
        .lib_cairo(FOREIGN_PACKAGE)
        .edition("2024_07")
        .build(&foreign_child_dir);

    ProjectBuilder::start()
        .name("target_package")
        .edition("2024_07")
        .dep("foreign_package", foreign_child_dir)
        .lib_cairo(TARGET_PACKAGE)
        .build(&target_child_dir);

    WorkspaceBuilder::start()
        .add_member("foreign_package")
        .add_member("target_package")
        .build(&root_dir);

    Scarb::quick_command()
        .arg("doc")
        .args(["-p", "target_package"])
        .current_dir(&root_dir)
        .assert()
        .success();

    MarkdownTargetChecker::default()
        .actual(
            root_dir
                .path()
                .join("target/doc/target_package")
                .to_str()
                .unwrap(),
        )
        .expected(EXPECTED_ROOT_TARGET_PACKAGE_PATH)
        .assert_all_files_match();
}
