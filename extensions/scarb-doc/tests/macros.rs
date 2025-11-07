//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use assert_fs::TempDir;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod markdown_target;
use markdown_target::MarkdownTargetChecker;

mod json_target;
use json_target::JsonTargetChecker;

const EXPECTED_ROOT_PACKAGE: &str = "tests/data/expose_macros";

const MACROS_CODE: &str = include_str!("code/code_11.cairo");

#[test]
fn json_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_11")
        .lib_cairo(MACROS_CODE)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&t)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&t.path().join("target/doc/output.json"))
        .expected("./data/json_macro_expose.json")
        .assert_files_match();
}

#[test]
fn markdown_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_11")
        .lib_cairo(MACROS_CODE)
        .manifest_package_extra(r#"experimental-features = ["user_defined_inline_macros"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();

    MarkdownTargetChecker::default()
        .actual(t.path().join("target/doc/hello_world").to_str().unwrap())
        .expected(EXPECTED_ROOT_PACKAGE)
        .assert_all_files_match();
}
