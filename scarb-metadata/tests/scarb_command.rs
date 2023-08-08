use assert_fs::TempDir;
use snapbox::cmd::{cargo_bin, Command};

use scarb_metadata::ScarbCommand;

#[test]
fn empty_project() {
    let t = TempDir::new().unwrap();

    let result = ScarbCommand::new()
        .scarb_path(cargo_bin("scarb"))
        .current_dir(t.path())
        .arg("fetch")
        .run();

    result.unwrap_err();
}

#[test]
fn sample_project() {
    let t = TempDir::new().unwrap();
    init_project(&t);

    let result = ScarbCommand::new()
        .scarb_path(cargo_bin("scarb"))
        .current_dir(t.path())
        .arg("fetch")
        .run();

    result.unwrap();
}

fn init_project(t: &TempDir) {
    Command::new(cargo_bin("scarb"))
        .args(["init", "--name", "hello"])
        .current_dir(t)
        .assert()
        .success();
}
