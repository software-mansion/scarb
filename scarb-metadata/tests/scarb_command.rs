use assert_fs::TempDir;
use scarb_metadata::ScarbCommand;
use scarb_test_support::cargo::cargo_bin;
use snapbox::cmd::Command;
use std::env;
use std::path::PathBuf;

fn scarb_bin() -> PathBuf {
    env::var_os("SCARB_TEST_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| cargo_bin("scarb"))
}

#[test]
fn empty_project() {
    let t = TempDir::new().unwrap();

    let result = ScarbCommand::new()
        .scarb_path(scarb_bin())
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
        .scarb_path(scarb_bin())
        .current_dir(t.path())
        .arg("fetch")
        .run();

    result.unwrap();
}

fn init_project(t: &TempDir) {
    Command::new(scarb_bin())
        .args(["init", "--name", "hello"])
        .env("SCARB_INIT_TEST_RUNNER", "none")
        .current_dir(t)
        .assert()
        .success();
}
