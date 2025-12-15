use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;
use snapbox::Data;

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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..] Updating git repository [..]
        "#});
}

#[test]
fn no_core_package_with_yes_core_deps() {
    let t = TempDir::new().unwrap();

    let dep1 = t.child("dep1");
    ProjectBuilder::start().name("dep1").build(&dep1);

    ProjectBuilder::start()
        .name("core")
        .version("1.0.0")
        .no_core()
        .dep("dep1", &dep1)
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: found dependencies on the same package `core` coming from incompatible sources:
            source 1: [..]Scarb.toml
            source 2: std
        "#});
}
