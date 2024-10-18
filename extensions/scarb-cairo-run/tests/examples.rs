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

    // Command::new and .env("SCARB_TARGET_DIR", t.path()) are used here, because this test is run
    // on a project from examples directory. In that case, the target dir (examples/hello_world/target)
    // is shared by all the tests, hence no need to create it multiple times.
    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished `dev` profile target(s) in [..]
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

    // Command::new and .env("SCARB_TARGET_DIR", t.path()) are used here, because this test is run
    // on a project from examples directory. In that case, the target dir (examples/hello_world/target)
    // is shared by all the tests, hence no need to create it multiple times.
    let snapbox = Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--no-build")
        .current_dir(example)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_eq(indoc! {r#"
            error: package has not been compiled, file does not exist: hello_world.sierra.json
            help: run `scarb build` to compile the package
            error: process did not exit successfully: exit code: 1
        "#});
    #[cfg(not(windows))]
    snapbox.stdout_eq(indoc! {r#"
            error: package has not been compiled, file does not exist: hello_world.sierra.json
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

    // Command::new and .env("SCARB_TARGET_DIR", t.path()) are used here, because this test is run
    // on a project from examples directory. In that case, the target dir (examples/hello_world/target)
    // is shared by all the tests, hence no need to create it multiple times.
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
                Finished `dev` profile target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
            Remaining gas: 59760
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

    // Command::new and .env("SCARB_TARGET_DIR", t.path()) are used here, because this test is run
    // on a project from examples directory. In that case, the target dir (examples/hello_world/target)
    // is shared by all the tests, hence no need to create it multiple times.
    let snapbox = Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("0")
        .current_dir(example)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello_world
            error: program requires gas counter, please provide `--available-gas` argument
            error: process did not exit successfully: exit code: 1
        "#});
    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello_world v0.1.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello_world
            error: program requires gas counter, please provide `--available-gas` argument
        "#});
}
