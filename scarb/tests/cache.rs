use assert_fs::{prelude::*, TempDir};

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn simple_clean() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);
    let cache_dir = TempDir::new().unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success();
    cache_dir.assert(predicates::path::is_dir());

    Scarb::quick_snapbox()
        .arg("cache")
        .arg("clean")
        .env("SCARB_CACHE", cache_dir.path())
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

    Scarb::quick_snapbox()
        .arg("cache")
        .arg("path")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .stdout_eq(format!("{}\n", cache_dir.path().display()))
        .success();
    cache_dir.assert(predicates::path::is_dir());
}
