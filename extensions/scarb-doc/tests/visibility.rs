use assert_fs::TempDir;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod target;
use target::TargetChecker;

const EXPECTED_PRIVATE_ITEMS_INCLUDED_PATH: &str = "tests/data/private_items_included";
const EXPECTED_PRIVATE_ITEMS_EXCLUDED_PATH: &str = "tests/data/private_items_excluded";

const EDITION_INCLUDING_PRIVATE_ITEMS: &str = "2023_01";
const EDITION_IGNORING_PRIVATE_ITEMS: &str = "2023_11";

const VISIBILITY_EXAMPLE_CODE: &str = include_str!("code/code_1.cairo");

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
        .args(["--document-private-items"])
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
        .expected(EXPECTED_PRIVATE_ITEMS_INCLUDED_PATH)
        .assert_all_files_match();
}

#[test]
fn include_private_items_with_old_ediiton() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .edition(EDITION_INCLUDING_PRIVATE_ITEMS)
        .name("hello_world")
        .lib_cairo(VISIBILITY_EXAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
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
        .expected(EXPECTED_PRIVATE_ITEMS_INCLUDED_PATH)
        .assert_all_files_match();
}

#[test]
fn ignore_private_items_with_new_eiditon() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .edition(EDITION_IGNORING_PRIVATE_ITEMS)
        .name("hello_world")
        .lib_cairo(VISIBILITY_EXAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
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
        .expected(EXPECTED_PRIVATE_ITEMS_EXCLUDED_PATH)
        .assert_all_files_match();
}
