use assert_fs::TempDir;
use snapbox::cmd::{cargo_bin, Command};

use scarb_metadata::MetadataCommand;

#[test]
fn empty_project() {
    let t = TempDir::new().unwrap();

    let result = MetadataCommand::new()
        .scarb_path(cargo_bin("scarb"))
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
        .scarb_path(cargo_bin("scarb"))
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
        .scarb_path(cargo_bin("scarb"))
        .no_deps()
        .current_dir(t.path())
        .inherit_stderr()
        .exec()
        .unwrap();
}

fn init_project(t: &TempDir) {
    Command::new(cargo_bin("scarb"))
        .args(["init", "--name", "hello"])
        .current_dir(t)
        .assert()
        .success();
}
