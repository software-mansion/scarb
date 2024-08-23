use assert_fs::TempDir;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod json_target;
use json_target::JsonTargetChecker;

const EDITION_INCLUDING_PRIVATE_ITEMS: &str = "2023_01";
const EDITION_IGNORING_PRIVATE_ITEMS: &str = "2023_11";

const VISIBILITY_EXAMPLE_CODE: &str = include_str!("code/code_5.cairo");

#[test]
fn document_private_items_flag() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .edition(EDITION_IGNORING_PRIVATE_ITEMS)
        .name("hello_world")
        .lib_cairo(VISIBILITY_EXAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items", "--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/json_private_items_included.json")
        .assert_files_match();
}

#[test]
fn include_private_items_with_old_edition() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .edition(EDITION_INCLUDING_PRIVATE_ITEMS)
        .name("hello_world")
        .lib_cairo(VISIBILITY_EXAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/json_private_items_included.json")
        .assert_files_match();
}

#[test]
fn ignore_private_items_with_new_edition() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .edition(EDITION_IGNORING_PRIVATE_ITEMS)
        .name("hello_world")
        .lib_cairo(VISIBILITY_EXAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/json_private_items_excluded.json")
        .assert_files_match();
}
