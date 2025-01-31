use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
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
fn prove_from_execution_output() {
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
        .success()
        .stdout_matches(indoc! {r#"
        [..]soundness of proof is not yet guaranteed by Stwo, use at your own risk
        [..]Proving hello
        Saving proof to: target/execute/hello/execution1/proof/proof.json
        "#});

    t.child("target/execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

#[test]
fn prove_with_track_relations() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success();

    let cmd = Scarb::quick_snapbox()
        .arg("prove")
        .arg("--execution-id=1")
        .arg("--track-relations")
        .current_dir(&t)
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(stdout.contains("Proving hello"));
    assert!(stdout.contains("Relations summary:"));
    assert!(stdout.contains("Saving proof to: target/execute/hello/execution1/proof/proof.json"));

    t.child("target/execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

#[test]
fn prove_with_display_components() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success();

    let cmd = Scarb::quick_snapbox()
        .arg("prove")
        .arg("--execution-id=1")
        .arg("--display-components")
        .current_dir(&t)
        .assert()
        .success();

    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(stdout.contains("Proving hello"));
    assert!(stdout.contains("CairoComponents"));
    assert!(stdout.contains("Saving proof to: target/execute/hello/execution1/proof/proof.json"));

    t.child("target/execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

#[test]
fn prove_fails_when_execution_output_not_found() {
    let t = build_executable_project();

    output_assert(
        Scarb::quick_snapbox()
            .arg("prove")
            .arg("--execution-id=1")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]soundness of proof is not yet guaranteed by Stwo, use at your own risk
        [..]Proving hello
        error: execution directory not found: [..]/target/execute/hello/execution1
        help: make sure to run `scarb execute` first
        and then run `scarb prove` with correct execution ID

        "#},
    )
}

#[test]
fn prove_fails_when_cairo_pie_output() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--target=bootloader")
        .arg("--output=cairo-pie")
        .current_dir(&t)
        .assert()
        .success();

    output_assert(
        Scarb::quick_snapbox()
            .arg("prove")
            .arg("--execution-id=1")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]soundness of proof is not yet guaranteed by Stwo, use at your own risk
        [..]Proving hello
        error: proving cairo pie output is not supported: [..]/target/execute/hello/execution1.zip
        help: run `scarb execute --output=standard` first
        and then run `scarb prove` with correct execution ID

        "#},
    )
}

#[test]
fn prove_with_execute() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("prove")
        .arg("--execute")
        .arg("--target=standalone")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]soundness of proof is not yet guaranteed by Stwo, use at your own risk
        [..]Compiling hello v0.1.0 ([..])
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello/execution1
        [..]Proving hello
        Saving proof to: target/execute/hello/execution1/proof/proof.json
        "#});

    t.child("target/execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

fn output_assert(output: OutputAssert, expected: &str) {
    #[cfg(windows)]
    output.stdout_matches(format!(
        "{expected}error: process did not exit successfully: exit code: 1\n"
    ));
    #[cfg(not(windows))]
    output.stdout_matches(expected);
}
