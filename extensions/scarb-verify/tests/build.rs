use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::{OutputAssertExt, Scarb};
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

// Disabled due to `scarb prove` not being supported on Windows
#[cfg(not(windows))]
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

// Disabled due to `scarb prove` not being supported on Windows
#[cfg(not(windows))]
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

    let proof_path = t.join("target/execute/hello/execution1/proof/proof.json");
    Scarb::quick_snapbox()
        .arg("verify")
        .arg("--proof-file")
        .arg(proof_path)
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

    Scarb::quick_snapbox()
        .arg("verify")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches_with_windows_exit_code_error(indoc! {r#"
            [..]Verifying hello
            error: execution directory does not exist at path: [..]/target/execute/hello/execution1
            help: make sure to run `scarb prove --execute` first
            and that the execution ID is correct

        "#});
}

#[test]
fn verify_fails_when_proof_file_not_found() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("verify")
        .arg("--proof-file=nonexistent.json")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches_with_windows_exit_code_error(indoc! {r#"
            [..]Verifying proof
            error: proof file does not exist at path: nonexistent.json
        "#});
}
