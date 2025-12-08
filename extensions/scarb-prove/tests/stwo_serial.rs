use assert_fs::TempDir;
use assert_fs::assert::PathAssert;
use assert_fs::prelude::PathChild;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

fn build_executable_project() -> TempDir {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
                [executable]

                [cairo]
                enable-gas = false
            "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);
    t
}

#[test]
#[cfg(not(windows))]
fn prove_with_execute() {
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
fn prove_from_execution_output() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("execute")
        .arg("--target=bootloader")
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
