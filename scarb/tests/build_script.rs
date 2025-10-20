use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn build_script_runs_before_compilation() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [scripts]
            build = "echo 'Prebuild script executed'"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Running build script for hello v1.0.0 ([..]Scarb.toml)
            Prebuild script executed
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn ignore_dependency_build_script() {
    let t = TempDir::new().unwrap();

    let dep = t.child("dep");
    ProjectBuilder::start()
        .name("dep")
        .manifest_extra(indoc! {r#"
            [scripts]
            build = "echo 'THIS SHOULD NOT BE PRINTED'"
        "#})
        .build(&dep);

    let main = t.child("main");
    ProjectBuilder::start()
        .name("main")
        .dep("dep", &dep)
        .build(&main);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&main)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling main v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
}
