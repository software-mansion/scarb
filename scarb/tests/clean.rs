use assert_fs::prelude::*;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn simple() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    t.child("target").assert(predicates::path::is_dir());

    Scarb::quick_snapbox()
        .arg("clean")
        .current_dir(&t)
        .assert()
        .success();
    t.child("target").assert(predicates::path::missing());
}
