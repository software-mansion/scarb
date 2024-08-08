//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use expect_test::expect_file;
use indoc::indoc;
use std::fs;
use std::iter::zip;
use walkdir::WalkDir;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx;
use scarb_test_support::project_builder::ProjectBuilder;

const EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH: &str = "tests/data/hello_world_no_features";
const EXPECTED_ROOT_PACKAGE_WITH_FEATURES_PATH: &str = "tests/data/hello_world_with_features";
const EXPECTED_ROOT_PACKAGE_WITHOUT_FEATURES_PATH: &str = "tests/data/hello_world_without_features";
const EXPECTED_SUB_PACKAGE_NO_FEATURES_PATH: &str =
    "tests/data/hello_world_sub_package_no_features";
const EXPECTED_SUB_PACKAGE_WITH_FEATURES_PATH: &str =
    "tests/data/hello_world_sub_package_with_features";
const EXPECTED_SUB_PACKAGE_WITHOUT_FEATURES_PATH: &str =
    "tests/data/hello_world_sub_package_without_features";

const TARGET_ROOT_PACKAGE_PATH: &str = "target/doc/hello_world";
const TARGET_SUB_PACKAGE_PATH: &str = "target/doc/hello_world_sub_package";

const ROOT_PACKAGE_NAME: &str = "hello_world";
const SUB_PACKAGE_NAME: &str = "hello_world_sub_package";

const FEATURE_NAME: &str = "test_feature";

const FIBONACCI_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_1.cairo");
const FIBONACCI_CODE_WITH_FEATURE: &str = include_str!("code/code_4.cairo");
const COMMON_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_2.cairo");
const COMMON_CODE_WITH_FEATURE: &str = include_str!("code/code_3.cairo");

#[test]
fn json_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&t)
        .assert()
        .success();

    let serialized_crates = fs::read_to_string(t.path().join("target/doc/output.json"))
        .expect("Failed to read from file");
    let expected = expect_file!["./data/json_output_test_data.json"];
    expected.assert_eq(&serialized_crates);
}

#[test]
fn markdown_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .current_dir(&t)
        .assert()
        .success();

    for (dir_entry_1, dir_entry_2) in zip(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(t.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let dir_entry_1 = dir_entry_1.unwrap();
        let dir_entry_2 = dir_entry_2.unwrap();

        if dir_entry_1.file_type().is_file() {
            assert!(dir_entry_2.file_type().is_file());

            let content = fs::read_to_string(dir_entry_2.path()).unwrap();

            let expect_file = expect_file![fsx::canonicalize(dir_entry_1.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}

#[test]
fn test_workspace_no_features() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child(SUB_PACKAGE_NAME);
    fs::create_dir(child_dir.path()).expect("Couldn't create a sub package directory.");

    ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [workspace]
            members = ["{}"]
            "#},
            SUB_PACKAGE_NAME
        ))
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&root_dir);

    ProjectBuilder::start()
        .name(SUB_PACKAGE_NAME)
        .lib_cairo(COMMON_CODE_WITHOUT_FEATURE)
        .build(&child_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "markdown", "--workspace"])
        .current_dir(&root_dir)
        .assert()
        .success();

    for (dir_entry_1, dir_entry_2, dir_entry_3, dir_entry_4) in multizip::zip4(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
        WalkDir::new(EXPECTED_SUB_PACKAGE_NO_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_SUB_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let root_dir_entry_expected = dir_entry_1.unwrap();
        let root_dir_entry = dir_entry_2.unwrap();
        let sub_package_dir_entry_expected = dir_entry_3.unwrap();
        let sub_package_dir = dir_entry_4.unwrap();

        if root_dir_entry_expected.file_type().is_file() {
            assert!(root_dir_entry.file_type().is_file());

            let content = fs::read_to_string(root_dir_entry.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(root_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }

        if sub_package_dir_entry_expected.file_type().is_file() {
            assert!(sub_package_dir.file_type().is_file());

            let content = fs::read_to_string(sub_package_dir.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(sub_package_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}

#[test]
fn test_workspace_without_features_in_manifest() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child(SUB_PACKAGE_NAME);
    fs::create_dir(child_dir.path()).expect("Couldn't create a sub package directory.");

    ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [workspace]
            members = ["{}"]
            "#},
            SUB_PACKAGE_NAME
        ))
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&root_dir);

    ProjectBuilder::start()
        .name(SUB_PACKAGE_NAME)
        .lib_cairo(COMMON_CODE_WITHOUT_FEATURE)
        .build(&child_dir);

    Scarb::quick_snapbox()
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
        .success();

    for (dir_entry_1, dir_entry_2, dir_entry_3, dir_entry_4) in multizip::zip4(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
        WalkDir::new(EXPECTED_SUB_PACKAGE_NO_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_SUB_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let root_dir_entry_expected = dir_entry_1.unwrap();
        let root_dir_entry = dir_entry_2.unwrap();
        let sub_package_dir_entry_expected = dir_entry_3.unwrap();
        let sub_package_dir = dir_entry_4.unwrap();

        if root_dir_entry_expected.file_type().is_file() {
            assert!(root_dir_entry.file_type().is_file());

            let content = fs::read_to_string(root_dir_entry.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(root_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }

        if sub_package_dir_entry_expected.file_type().is_file() {
            assert!(sub_package_dir.file_type().is_file());

            let content = fs::read_to_string(sub_package_dir.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(sub_package_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}

#[test]
fn test_workspace_with_working_feature_in_root_and_sub_package() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child(SUB_PACKAGE_NAME);
    fs::create_dir(child_dir.path()).expect("Couldn't create a sub package directory.");

    let root_project = ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [features]
            {} = []

            [workspace]
            members = ["{}"]
            "#},
            FEATURE_NAME, SUB_PACKAGE_NAME
        ))
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE);

    root_project.build(&root_dir);

    ProjectBuilder::start()
        .name(SUB_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [features]
            {} = []
            "#},
            FEATURE_NAME
        ))
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

    for (dir_entry_1, dir_entry_2, dir_entry_3, dir_entry_4) in multizip::zip4(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_WITH_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
        WalkDir::new(EXPECTED_SUB_PACKAGE_WITH_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_SUB_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let root_dir_entry_expected = dir_entry_1.unwrap();
        let root_dir_entry = dir_entry_2.unwrap();
        let sub_package_dir_entry_expected = dir_entry_3.unwrap();
        let sub_package_dir = dir_entry_4.unwrap();

        if root_dir_entry_expected.file_type().is_file() {
            assert!(root_dir_entry.file_type().is_file());

            let content = fs::read_to_string(root_dir_entry.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(root_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }

        if sub_package_dir_entry_expected.file_type().is_file() {
            assert!(sub_package_dir.file_type().is_file());

            let content = fs::read_to_string(sub_package_dir.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(sub_package_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}

#[test]
fn test_workspace_with_working_feature_in_root_only() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child(SUB_PACKAGE_NAME);
    fs::create_dir(child_dir.path()).expect("Couldn't create a sub package directory.");

    let root_project = ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [features]
            {} = []

            [workspace]
            members = ["{}"]
            "#},
            FEATURE_NAME, SUB_PACKAGE_NAME
        ))
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE);

    root_project.build(&root_dir);

    ProjectBuilder::start()
        .name(SUB_PACKAGE_NAME)
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

    for (dir_entry_1, dir_entry_2, dir_entry_3, dir_entry_4) in multizip::zip4(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_WITH_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
        WalkDir::new(EXPECTED_SUB_PACKAGE_WITHOUT_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_SUB_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let root_dir_entry_expected = dir_entry_1.unwrap();
        let root_dir_entry = dir_entry_2.unwrap();
        let sub_package_dir_entry_expected = dir_entry_3.unwrap();
        let sub_package_dir = dir_entry_4.unwrap();

        if root_dir_entry_expected.file_type().is_file() {
            assert!(root_dir_entry.file_type().is_file());

            let content = fs::read_to_string(root_dir_entry.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(root_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }

        if sub_package_dir_entry_expected.file_type().is_file() {
            assert!(sub_package_dir.file_type().is_file());

            let content = fs::read_to_string(sub_package_dir.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(sub_package_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}

#[test]
fn test_workspace_with_working_feature_in_sub_package_only() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child(SUB_PACKAGE_NAME);
    fs::create_dir(child_dir.path()).expect("Couldn't create a sub package directory.");

    let root_project = ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [workspace]
            members = ["{}"]
            "#},
            SUB_PACKAGE_NAME
        ))
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE);

    root_project.build(&root_dir);

    ProjectBuilder::start()
        .name(SUB_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [features]
            {} = []
            "#},
            FEATURE_NAME
        ))
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

    for (dir_entry_1, dir_entry_2, dir_entry_3, dir_entry_4) in multizip::zip4(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_WITHOUT_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
        WalkDir::new(EXPECTED_SUB_PACKAGE_WITH_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_SUB_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let root_dir_entry_expected = dir_entry_1.unwrap();
        let root_dir_entry = dir_entry_2.unwrap();
        let sub_package_dir_entry_expected = dir_entry_3.unwrap();
        let sub_package_dir = dir_entry_4.unwrap();

        if root_dir_entry_expected.file_type().is_file() {
            assert!(root_dir_entry.file_type().is_file());

            let content = fs::read_to_string(root_dir_entry.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(root_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }

        if sub_package_dir_entry_expected.file_type().is_file() {
            assert!(sub_package_dir.file_type().is_file());

            let content = fs::read_to_string(sub_package_dir.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(sub_package_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}

#[test]
fn test_workspace_without_features_in_manifest_and_present_in_sub_package_code() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child(SUB_PACKAGE_NAME);
    fs::create_dir(child_dir.path()).expect("Couldn't create a sub package directory.");

    ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [workspace]
            members = ["{}"]
            "#},
            SUB_PACKAGE_NAME
        ))
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&root_dir);

    ProjectBuilder::start()
        .name(SUB_PACKAGE_NAME)
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_snapbox()
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
        .success();

    for (dir_entry_1, dir_entry_2, dir_entry_3, dir_entry_4) in multizip::zip4(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
        WalkDir::new(EXPECTED_SUB_PACKAGE_WITHOUT_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_SUB_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let root_dir_entry_expected = dir_entry_1.unwrap();
        let root_dir_entry = dir_entry_2.unwrap();
        let sub_package_dir_entry_expected = dir_entry_3.unwrap();
        let sub_package_dir = dir_entry_4.unwrap();

        if root_dir_entry_expected.file_type().is_file() {
            assert!(root_dir_entry.file_type().is_file());

            let content = fs::read_to_string(root_dir_entry.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(root_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }

        if sub_package_dir_entry_expected.file_type().is_file() {
            assert!(sub_package_dir.file_type().is_file());

            let content = fs::read_to_string(sub_package_dir.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(sub_package_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}

#[test]
fn test_workspace_without_features_in_manifest_and_present_in_root_package_code() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child(SUB_PACKAGE_NAME);
    fs::create_dir(child_dir.path()).expect("Couldn't create a sub package directory.");

    ProjectBuilder::start()
        .name(ROOT_PACKAGE_NAME)
        .manifest_extra(format!(
            indoc! {r#"
            [workspace]
            members = ["{}"]
            "#},
            SUB_PACKAGE_NAME
        ))
        .lib_cairo(FIBONACCI_CODE_WITH_FEATURE)
        .build(&root_dir);

    ProjectBuilder::start()
        .name(SUB_PACKAGE_NAME)
        .lib_cairo(COMMON_CODE_WITH_FEATURE)
        .build(&child_dir);

    Scarb::quick_snapbox()
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
        .success();

    for (dir_entry_1, dir_entry_2, dir_entry_3, dir_entry_4) in multizip::zip4(
        WalkDir::new(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_ROOT_PACKAGE_PATH)).sort_by_file_name(),
        WalkDir::new(EXPECTED_SUB_PACKAGE_WITHOUT_FEATURES_PATH).sort_by_file_name(),
        WalkDir::new(root_dir.path().join(TARGET_SUB_PACKAGE_PATH)).sort_by_file_name(),
    ) {
        let root_dir_entry_expected = dir_entry_1.unwrap();
        let root_dir_entry = dir_entry_2.unwrap();
        let sub_package_dir_entry_expected = dir_entry_3.unwrap();
        let sub_package_dir = dir_entry_4.unwrap();

        if root_dir_entry_expected.file_type().is_file() {
            assert!(root_dir_entry.file_type().is_file());

            let content = fs::read_to_string(root_dir_entry.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(root_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }

        if sub_package_dir_entry_expected.file_type().is_file() {
            assert!(sub_package_dir.file_type().is_file());

            let content = fs::read_to_string(sub_package_dir.path()).unwrap();

            let expect_file =
                expect_file![fsx::canonicalize(sub_package_dir_entry_expected.path()).unwrap()];
            expect_file.assert_eq(&content);
        }
    }
}
