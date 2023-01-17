use assert_fs::prelude::*;
use snapbox::cmd::{cargo_bin, Command};

#[test]
fn simple() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r"fn main() -> felt { 42 }")
        .unwrap();

    Command::new(cargo_bin!("scarb"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    t.child("target").assert(predicates::path::is_dir());

    Command::new(cargo_bin!("scarb"))
        .arg("clean")
        .current_dir(&t)
        .assert()
        .success();
    t.child("target").assert(predicates::path::missing());
}
