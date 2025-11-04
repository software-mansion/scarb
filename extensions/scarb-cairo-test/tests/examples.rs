use std::path::Path;

use scarb_test_support::cargo::cargo_bin;
use snapbox::cmd::Command;

use test_for_each_example::test_for_each_example;

// TODO(maciektr): Revert ignoring the dependencies test case.
#[test_for_each_example(ignore = "dependencies")]
fn cairo_test(example: &Path) {
    Command::new(cargo_bin("scarb"))
        .arg("cairo-test")
        .arg("--workspace")
        .current_dir(example)
        .assert()
        .success();
}
