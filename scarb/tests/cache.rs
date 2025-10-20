use assert_fs::{TempDir, prelude::*};
use snapbox::Data;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn simple_clean() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);
    let cache_dir = TempDir::new().unwrap();

    Scarb::new()
        .cache(cache_dir.path())
        .snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
    cache_dir.assert(predicates::path::is_dir());

    Scarb::new()
        .cache(cache_dir.path())
        .snapbox()
        .arg("cache")
        .arg("clean")
        .current_dir(&t)
        .assert()
        .success();
    cache_dir.assert(predicates::path::missing());
}

#[test]
fn path_print() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);
    let cache_dir = TempDir::new().unwrap();

    Scarb::new()
        .cache(cache_dir.path())
        .snapbox()
        .arg("cache")
        .arg("path")
        .current_dir(&t)
        .assert()
        .stdout_eq(Data::from(format!("{}\n", cache_dir.path().display())).raw())
        .success();
    cache_dir.assert(predicates::path::is_dir());
}
