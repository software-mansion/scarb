use assert_fs::TempDir;
use indoc::indoc;
use snapbox::cmd::{cargo_bin, Command};

use scarb_test_support::cargo::manifest_dir;

#[test]
fn hello_world() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("hello_world");

    let t = TempDir::new().unwrap();

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("build")
        .current_dir(example.clone())
        .assert()
        .success();
    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("2000000")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
            Remaining gas: 1953640
        "#});
}

#[test]
fn package_not_built() {
    let example = manifest_dir()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join("hello_world");

    let t = TempDir::new().unwrap();

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("2000000")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
            Remaining gas: 1953640
        "#});
}
