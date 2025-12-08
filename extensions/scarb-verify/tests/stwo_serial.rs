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

// Disabled due to `scarb prove` not being supported on Windows
#[cfg(not(windows))]
#[test]
fn verify_from_execution_output() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("prove")
        .arg("--execute")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_command()
        .arg("verify")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Verifying hello
        [..]Verified proof successfully
        "#});
}

// Disabled due to `scarb prove` not being supported on Windows
#[cfg(not(windows))]
#[test]
fn verify_from_path() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("prove")
        .arg("--execute")
        .current_dir(&t)
        .assert()
        .success();

    let proof_path = t.join("target/execute/hello/execution1/proof/proof.json");
    Scarb::quick_command()
        .arg("verify")
        .arg("--proof-file")
        .arg(proof_path)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Verifying proof
        [..]Verified proof successfully
        "#});
}
