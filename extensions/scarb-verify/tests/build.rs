use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::cmd::OutputAssert;

fn build_executable_project() -> TempDir {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
                [executable]
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
fn verify_from_execution_output() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_snapbox()
        .arg("prove")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_snapbox()
        .arg("verify")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Verifying hello
        [..]Verified proof successfully
        "#});
}

#[test]
fn verify_from_path() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_snapbox()
        .arg("prove")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_snapbox()
        .arg("verify")
        .arg("--proof-file=target/execute/hello/execution1/proof/proof.json")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Verifying proof
        [..]Verified proof successfully
        "#});
}

#[test]
fn verify_fails_when_execution_output_not_found() {
    let t = build_executable_project();

    output_assert(
        Scarb::quick_snapbox()
            .arg("verify")
            .arg("--execution-id=1")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Verifying hello
        error: execution directory does not exist at path: [..]/target/execute/hello/execution1
        help: make sure to run `scarb prove --execute` first
        and that the execution ID is correct

        "#},
    )
}

#[test]
fn verify_fails_when_proof_file_not_found() {
    let t = build_executable_project();

    output_assert(
        Scarb::quick_snapbox()
            .arg("verify")
            .arg("--proof-file=nonexistent.json")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Verifying proof
        error: proof file does not exist: nonexistent.json
        "#},
    )
}

fn output_assert(output: OutputAssert, expected: &str) {
    #[cfg(windows)]
    output.stdout_matches(format!(
        "{expected}error: process did not exit successfully: exit code: 1\n"
    ));
    #[cfg(not(windows))]
    output.stdout_matches(expected);
}
