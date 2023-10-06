use std::path::Path;

use snapbox::cmd::{cargo_bin, Command};

use test_for_each_example::test_for_each_example;

#[test_for_each_example]
fn cairo_test(example: &Path) {
    Command::new(cargo_bin("scarb"))
        .arg("cairo-test")
        .arg("--workspace")
        .current_dir(example)
        .assert()
        .success();
}
