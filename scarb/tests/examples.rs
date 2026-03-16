use std::path::Path;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use snapbox::cmd::Command;

use scarb_test_support::cargo::cargo_bin;
use scarb_test_support::command::Scarb;
use test_for_each_example::test_for_each_example;

#[test_for_each_example(ignore = "dependencies,procedural_macros")]
fn build(example: &Path) {
    Command::new(cargo_bin("scarb"))
        .arg("clean")
        .current_dir(example)
        .assert()
        .success();

    Command::new(cargo_bin("scarb"))
        .arg("build")
        .current_dir(example)
        .assert()
        .success();
}

#[test]
fn build_procedural_macro_examples() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let proc_macros_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("procedural_macros");

    for entry in proc_macros_dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            let macro_dir = entry.path().join("macro");
            if macro_dir.is_dir() {
                Command::new(&cargo)
                    .arg("clean")
                    .current_dir(&macro_dir)
                    .assert()
                    .success();

                Command::new(&cargo)
                    .arg("check")
                    .current_dir(&macro_dir)
                    .assert()
                    .success();
            }

            let cairo_repo = entry.path().join("cairo-repository");
            if cairo_repo.is_dir() {
                Scarb::quick_command()
                    .arg("build")
                    .current_dir(&cairo_repo)
                    .assert()
                    .success();
            }
        }
    }
}

#[test_for_each_example(ignore = "procedural_macros")]
fn readme(example: &Path) {
    let example_name = example.file_name().unwrap().to_str().unwrap();
    let readme = ChildPath::new(example.join("README.md"));

    readme
        .assert(predicates::path::exists())
        .assert(predicates::str::starts_with(format!("# `{example_name}`")));
}
