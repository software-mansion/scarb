use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::gitx;
use scarb_test_support::project_builder::ProjectBuilder;

/// https://github.com/software-mansion/scarb/issues/600
#[test]
fn issue_600_path() {
    let t = TempDir::new().unwrap();

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.7.0-rc.0")
        .lib_cairo("fn hello() -> felt252 { 42 }")
        .build(&dep1);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &dep1)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
}

/// https://github.com/software-mansion/scarb/issues/600
#[test]
fn issue_600_git() {
    let t = TempDir::new().unwrap();

    let dep1 = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .version("0.7.0-rc.0")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &dep1)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Updating git repository [..]
        "#});
}
