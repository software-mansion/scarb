use assert_fs::TempDir;
use indoc::indoc;
use snapbox::cmd::{Command, cargo_bin};

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
            warn: `scarb cairo-run` will be deprecated soon
            help: use `scarb execute` instead
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
    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--no-build")
        .current_dir(example)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            warn: `scarb cairo-run` will be deprecated soon
            help: use `scarb execute` instead
            error: package has not been compiled, file does not exist: `hello_world.sierra.json`
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
        .arg("100000000")
        .current_dir(example)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            warn: `scarb cairo-run` will be deprecated soon
            help: use `scarb execute` instead
               Compiling hello_world v0.1.0 ([..]/Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello_world
            Run completed successfully, returning [987]
            Remaining gas: 99804810
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
    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("0")
        .current_dir(example)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            warn: `scarb cairo-run` will be deprecated soon
            help: use `scarb execute` instead
               Compiling hello_world v0.1.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello_world
            error: program requires gas counter, please provide `--available-gas` argument
        "#});
}
