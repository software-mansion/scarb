use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn simple() {
    let t = TempDir::new().unwrap();
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

#[test]
fn requires_workspace() {
    let t = TempDir::new().unwrap();
    t.child("target/dev/some.sierra.json")
        .write_str("Lorem ipsum.")
        .unwrap();

    Scarb::quick_snapbox()
        .arg("clean")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to read manifest at: [..]/Scarb.toml

            Caused by:
                No such file or directory (os error 2)
        "#});

    t.child("target/dev/some.sierra.json")
        .assert(predicates::path::is_file());
}
