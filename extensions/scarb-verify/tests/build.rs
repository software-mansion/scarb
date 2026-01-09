use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::fixtures::build_executable_project;

#[test]
fn verify_fails_when_execution_output_not_found() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("verify")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Verifying hello
            error: execution directory does not exist at path: [..]/target/execute/hello/execution1
            help: make sure to run `scarb prove --execute` first
            and that the execution ID is correct

        "#});
}

#[test]
fn verify_fails_when_proof_file_not_found() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("verify")
        .arg("--proof-file=nonexistent.json")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Verifying proof
            error: proof file does not exist at path: nonexistent.json
        "#});
}
