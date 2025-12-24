use assert_fs::TempDir;
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
