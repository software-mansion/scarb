use assert_fs::TempDir;
use snapbox::cmd::{cargo_bin, Command};
use std::env;
use std::path::PathBuf;

use scarb_metadata::MetadataCommand;

fn scarb_bin() -> PathBuf {
    env::var_os("SCARB_BIN_PATH")
        .map(PathBuf::from)
        .unwrap_or(cargo_bin("scarb"))
}

#[test]
fn empty_project() {
    let t = TempDir::new().unwrap();

    let result = MetadataCommand::new()
        .scarb_path(scarb_bin())
        .current_dir(t.path())
        .inherit_stderr()
        .exec();

    assert!(result
        .unwrap_err()
        .to_string()
        .contains("failed to read manifest"));
}

#[test]
fn sample_project() {
    let t = TempDir::new().unwrap();
    init_project(&t);

    MetadataCommand::new()
        .scarb_path(scarb_bin())
        .current_dir(t.path())
        .inherit_stderr()
        .exec()
        .unwrap();
}

#[test]
fn no_deps() {
    let t = TempDir::new().unwrap();
    init_project(&t);

    MetadataCommand::new()
        .scarb_path(scarb_bin())
        .no_deps()
        .current_dir(t.path())
        .inherit_stderr()
        .exec()
        .unwrap();
}

#[test]
fn manifest_path() {
    let t = TempDir::new().unwrap();
    init_project(&t);

    MetadataCommand::new()
        .scarb_path(scarb_bin())
        .manifest_path(t.join("Scarb.toml").as_path())
        .inherit_stderr()
        .exec()
        .unwrap();
}

fn init_project(t: &TempDir) {
    Command::new(scarb_bin())
        .args(["init", "--name", "hello"])
        .current_dir(t)
        .assert()
        .success();
}
