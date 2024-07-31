use std::path::Path;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use snapbox::cmd::Command;

use scarb_test_support::cargo::cargo_bin;
use test_for_each_example::test_for_each_example;

#[test_for_each_example(ignore = "dependencies")]
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

#[test_for_each_example]
fn readme(example: &Path) {
    let example_name = example.file_name().unwrap().to_str().unwrap();
    let readme = ChildPath::new(example.join("README.md"));

    readme
        .assert(predicates::path::exists())
        .assert(predicates::str::starts_with(format!("# `{example_name}`")));
}
