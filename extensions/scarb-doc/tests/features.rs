//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::{formatdoc, indoc};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

const EXPECTED_WORKSPACE_ROOT_WITH_SUB_PACKAGE_PATH: &str =
    "tests/data/hello_world_workspace_with_sub_package";
const EXPECTED_WORKSPACE_WITH_SUB_PACKAGE_WITH_FEATURES_PATH: &str =
    "tests/data/hello_world_workspace_with_sub_package_features";

mod markdown_target;
use markdown_target::MarkdownTargetChecker;

const FEATURE_NAME: &str = "test_feature";

const FIBONACCI_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_1.cairo");
const FIBONACCI_CODE_WITH_FEATURE: &str = include_str!("code/code_4.cairo");
const COMMON_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_2.cairo");
const COMMON_CODE_WITH_FEATURE: &str = include_str!("code/code_3.cairo");

#[test]
fn test_workspace_no_features() {
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
        .args(["--workspace", "--disable-remote-linking"])
        .current_dir(&root_dir)
        .assert()
        .success();

    MarkdownTargetChecker::default()
        .actual(root_dir.path().join("target/doc/").to_str().unwrap())
        .expected(EXPECTED_WORKSPACE_ROOT_WITH_SUB_PACKAGE_PATH)
        .assert_all_files_match();
}

#[test]
fn test_workspace_without_features_in_manifest() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("hello_world_sub_package");

    let root = ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .lib_cairo(COMMON_CODE_WITHOUT_FEATURE)
        .build(&child_dir);

    Scarb::quick_command()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--output-format",
            "markdown",
            "--workspace",
            "--disable-remote-linking",
            "--features",
            FEATURE_NAME,
        ])
        .current_dir(&root_dir)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: metadata command failed: `scarb metadata` exited with error

            stdout:
            error: none of the selected packages contains `test_feature` feature
            note: to use features, you need to define [features] section in Scarb.toml

            stderr:

        "#});
}

#[test]
fn test_workspace_with_working_feature_in_root_and_sub_package() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("hello_world_sub_package");

    let root = ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .manifest_extra(formatdoc! {r#"
            [features]
            {feature_name} = []
            "#,
            feature_name = FEATURE_NAME,
        })
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .edition("2023_01")
        .manifest_extra(formatdoc! {r#"
            [features]
            {feature_name} = []
            "#,
            feature_name = FEATURE_NAME
        })
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_command()
        .arg("doc")
        .args([
            "--workspace",
            "--disable-remote-linking",
            "--features",
            FEATURE_NAME,
            "--output-format",
            "markdown",
        ])
        .current_dir(&root_dir)
        .assert()
        .success();

    MarkdownTargetChecker::default()
        .actual(root_dir.path().join("target/doc/").to_str().unwrap())
        .expected(EXPECTED_WORKSPACE_WITH_SUB_PACKAGE_WITH_FEATURES_PATH)
        .assert_all_files_match();
}

#[test]
fn test_workspace_with_working_feature_in_root_only() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("hello_world_sub_package");

    let root = ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .manifest_extra(formatdoc! {r#"
            [features]
            {feature_name} = []
            "#,
            feature_name = FEATURE_NAME
        })
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_command()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--workspace",
            "--disable-remote-linking",
            "--features",
            FEATURE_NAME,
            "--output-format",
            "markdown",
        ])
        .current_dir(&root_dir)
        .assert()
        .success();
}

#[test]
fn test_workspace_with_working_feature_in_sub_package_only() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("hello_world_sub_package");

    let root = ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .manifest_extra(formatdoc! {r#"
            [features]
            {feature_name} = []
            "#,
            feature_name = FEATURE_NAME
        })
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_command()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--workspace",
            "--disable-remote-linking",
            "--features",
            FEATURE_NAME,
            "--output-format",
            "markdown",
        ])
        .current_dir(&root_dir)
        .assert()
        .success();
}

#[test]
fn test_workspace_without_features_in_manifest_and_present_in_sub_package_code() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("hello_world_sub_package");

    let root = ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_command()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--output-format",
            "markdown",
            "--disable-remote-linking",
            "--workspace",
            "--features",
            FEATURE_NAME,
        ])
        .current_dir(&root_dir)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: metadata command failed: `scarb metadata` exited with error

            stdout:
            error: none of the selected packages contains `test_feature` feature
            note: to use features, you need to define [features] section in Scarb.toml

            stderr:

        "#});
}

#[test]
fn test_workspace_without_features_in_manifest_and_present_in_root_package_code() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("hello_world_sub_package");

    let root = ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_command()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--output-format",
            "markdown",
            "--disable-remote-linking",
            "--workspace",
            "--features",
            FEATURE_NAME,
        ])
        .current_dir(&root_dir)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: metadata command failed: `scarb metadata` exited with error
    
            stdout:
            error: none of the selected packages contains `test_feature` feature
            note: to use features, you need to define [features] section in Scarb.toml
    
            stderr:

        "#});
}
