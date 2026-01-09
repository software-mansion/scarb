use assert_fs::assert::PathAssert;
use assert_fs::prelude::PathChild;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::fixtures::build_executable_project;

#[test]
#[cfg(not(windows))]
#[cfg_attr(
    not(feature = "heavy-tests"),
    ignore = "heavy tests must be run with feature flag"
)]
fn sequential_prove_with_execute() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("prove")
        .arg("--execute")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..])
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello/execution1
        [..]Proving hello
        warn: soundness of proof is not yet guaranteed by Stwo, use at your own risk
        Saving proof to: target/execute/hello/execution1/proof/proof.json
        "#});

    t.child("target/execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

#[test]
#[cfg(not(windows))]
#[cfg_attr(
    not(feature = "heavy-tests"),
    ignore = "heavy tests must be run with feature flag"
)]
fn sequential_prove_from_execution_output() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("execute")
        .arg("--target=bootloader")
        .arg("--output=standard")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_command()
        .arg("prove")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Proving hello
        warn: soundness of proof is not yet guaranteed by Stwo, use at your own risk
        Saving proof to: target/execute/hello/execution1/proof/proof.json
        "#});

    t.child("target/execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}
