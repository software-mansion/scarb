use assert_fs::TempDir;
use indoc::indoc;
use snapbox::cmd::{cargo_bin, Command};

use scarb_test_support::cargo::manifest_dir;

#[test]
fn scarb_build_is_called() {
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
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
        "#});
}

#[test]
fn build_can_be_skipped() {
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
        .arg("--no-build")
        .current_dir(example)
        .assert()
        .failure()
        .stderr_eq(indoc! {r#"
            Error: package has not been compiled, file does not exist: hello_world.sierra.json
            help: run `scarb build` to compile the package

        "#});
}

#[test]
fn can_limit_gas() {
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
        .arg("100000")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
            Remaining gas: 53640
        "#});
}

#[test]
fn can_disable_gas() {
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
        .arg("0")
        .current_dir(example)
        .assert()
        .failure()
        .stderr_eq(indoc! {r#"
            Error: program requires gas counter, please provide `--available-gas` argument
        "#});
}
