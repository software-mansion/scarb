//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

const EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH: &str = "tests/data/hello_world_no_features";
const EXPECTED_ROOT_PACKAGE_WITH_FEATURES_PATH: &str = "tests/data/hello_world_with_features";
const EXPECTED_SUB_PACKAGE_NO_FEATURES_PATH: &str =
    "tests/data/hello_world_sub_package_no_features";
const EXPECTED_SUB_PACKAGE_WITH_FEATURES_PATH: &str =
    "tests/data/hello_world_sub_package_with_features";

mod target;
use target::TargetChecker;

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
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE);

    WorkspaceBuilder::start()
        .add_member("hello_world_sub_package")
        .package(root)
        .build(&root_dir);

    ProjectBuilder::start()
        .name("hello_world_sub_package")
        .lib_cairo(COMMON_CODE_WITHOUT_FEATURE)
        .build(&child_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "markdown", "--workspace"])
        .current_dir(&root_dir)
        .assert()
        .success();

    TargetChecker::default()
        .actual(
            root_dir
                .path()
                .join("target/doc/hello_world")
                .to_str()
                .unwrap(),
        )
        .expected(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH)
        .assert_all_files_match();

    TargetChecker::default()
        .actual(
            root_dir
                .path()
                .join("target/doc/hello_world_sub_package")
                .to_str()
                .unwrap(),
        )
        .expected(EXPECTED_SUB_PACKAGE_NO_FEATURES_PATH)
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

    let snapbox = Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--output-format",
            "markdown",
            "--workspace",
            "--features",
            FEATURE_NAME,
        ])
        .current_dir(&root_dir)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
        note: to use features, you need to define [features] section in Scarb.toml

        stderr:
        
        error: process did not exit successfully: exit code: 1
        "#});

    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
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
        .manifest_extra(formatdoc! {r#"
            [features]
            {feature_name} = []
            "#,
            feature_name = FEATURE_NAME
        })
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args([
            "--workspace",
            "--features",
            FEATURE_NAME,
            "--output-format",
            "markdown",
        ])
        .current_dir(&root_dir)
        .assert()
        .success();

    TargetChecker::default()
        .actual(
            root_dir
                .path()
                .join("target/doc/hello_world")
                .to_str()
                .unwrap(),
        )
        .expected(EXPECTED_ROOT_PACKAGE_WITH_FEATURES_PATH)
        .assert_all_files_match();

    TargetChecker::default()
        .actual(
            root_dir
                .path()
                .join("target/doc/hello_world_sub_package")
                .to_str()
                .unwrap(),
        )
        .expected(EXPECTED_SUB_PACKAGE_WITH_FEATURES_PATH)
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

    let snapbox = Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--workspace",
            "--features",
            FEATURE_NAME,
            "--output-format",
            "markdown",
        ])
        .current_dir(&root_dir)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
        note: to use features, you need to define [features] section in Scarb.toml

        stderr:
        
        error: process did not exit successfully: exit code: 1
        "#});

    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
        note: to use features, you need to define [features] section in Scarb.toml

        stderr:

        "#});
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

    let snapbox = Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--workspace",
            "--features",
            FEATURE_NAME,
            "--output-format",
            "markdown",
        ])
        .current_dir(&root_dir)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
        note: to use features, you need to define [features] section in Scarb.toml

        stderr:
        
        error: process did not exit successfully: exit code: 1
        "#});

    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
        note: to use features, you need to define [features] section in Scarb.toml
        
        stderr:

        "#});
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

    let snapbox = Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--output-format",
            "markdown",
            "--workspace",
            "--features",
            FEATURE_NAME,
        ])
        .current_dir(&root_dir)
        .assert()
        .failure();
    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
            error: metadata command failed: `scarb metadata` exited with error
    
            stdout:
            error: no features in manifest
            note: to use features, you need to define [features] section in Scarb.toml
    
            stderr:
            
            error: process did not exit successfully: exit code: 1
            "#});

    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
            error: metadata command failed: `scarb metadata` exited with error
    
            stdout:
            error: no features in manifest
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

    let snapbox = Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("doc")
        .args([
            "--output-format",
            "markdown",
            "--workspace",
            "--features",
            FEATURE_NAME,
        ])
        .current_dir(&root_dir)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
        note: to use features, you need to define [features] section in Scarb.toml

        stderr:
        
        error: process did not exit successfully: exit code: 1
        "#});

    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
        error: metadata command failed: `scarb metadata` exited with error

        stdout:
        error: no features in manifest
        note: to use features, you need to define [features] section in Scarb.toml

        stderr:

        "#});
}
