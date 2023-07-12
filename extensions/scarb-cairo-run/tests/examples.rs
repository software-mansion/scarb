use indoc::indoc;
use snapbox::cmd::{cargo_bin, Command};

use scarb_test_support::cargo::manifest_dir;

#[test]
fn cairo_test_success() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("cairo_run_example");
    Command::new(cargo_bin("scarb"))
        .arg("build")
        .current_dir(example.clone())
        .assert()
        .success();
    Command::new(cargo_bin("scarb"))
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("2000000")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            running cairo_run_example ...
            Run completed successfully, returning [2]
            Remaining gas: 1971340
        "#});
}

#[test]
fn cairo_test_package_not_built() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("cairo_run_example");
    Command::new(cargo_bin("scarb"))
        .arg("clean")
        .current_dir(example.clone())
        .assert()
        .success();
    Command::new(cargo_bin("scarb"))
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("2000000")
        .current_dir(example)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            Error: package has not been compiled, file does not exist: cairo_run_example.sierra
            help: run `scarb build` to compile the package

        "#});
}
