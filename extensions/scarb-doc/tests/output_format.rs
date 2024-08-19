//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use std::fs;

use assert_fs::TempDir;
use expect_test::expect_file;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod target;
use target::TargetChecker;

const EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH: &str = "tests/data/hello_world_no_features";

const FIBONACCI_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_1.cairo");

#[test]
fn json_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
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
        .name("hello_world")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .current_dir(&t)
        .assert()
        .success();

    TargetChecker::default()
        .actual(t.path().join("target/doc/hello_world").to_str().unwrap())
        .expected(EXPECTED_ROOT_PACKAGE_NO_FEATURES_PATH)
        .assert_all_files_match();
}
