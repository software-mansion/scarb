//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod markdown_target;
use markdown_target::MarkdownTargetChecker;

mod json_target;
use json_target::JsonTargetChecker;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

const EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH: &str = "tests/data/hello_world_no_features";
const EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH_MDX: &str = "tests/data/hello_world_no_features_mdx";
const FIBONACCI_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_1.cairo");
const COMMON_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_2.cairo");

#[test]
fn json_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&t)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&t.path().join("target/doc/output.json"))
        .expected("./data/json_output_test_data.json")
        .assert_files_match();
}

#[test]
fn markdown_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .current_dir(&t)
        .assert()
        .success();

    MarkdownTargetChecker::default()
        .actual(t.path().join("target/doc/hello_world").to_str().unwrap())
        .expected(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH)
        .assert_all_files_match();
}

#[test]
fn mdx_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "mdx"])
        .current_dir(&t)
        .assert()
        .success();

    MarkdownTargetChecker::default()
        .actual(t.path().join("target/doc/hello_world").to_str().unwrap())
        .expected(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH_MDX)
        .assert_all_files_match();
}

#[test]
fn test_workspace_json() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("hello_world_sub_package");

    let root = ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .edition("2023_01")
        .lib_cairo(COMMON_CODE_WITHOUT_FEATURE)
        .build(&child_dir);

    Scarb::quick_command()
        .arg("doc")
        .args(["--workspace", "--output-format", "json"])
        .current_dir(&root_dir)
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/json_workspace_with_sub_package.json")
        .assert_files_match();
}
