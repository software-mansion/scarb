use std::path::Path;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use snapbox::cmd::{cargo_bin, Command};

use test_for_each_example::test_for_each_example;

#[test_for_each_example]
fn build(example: &Path) {
    Command::new(cargo_bin!("scarb"))
        .arg("clean")
        .current_dir(example)
        .assert()
        .success();

    Command::new(cargo_bin!("scarb"))
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
