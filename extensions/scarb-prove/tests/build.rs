use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::fixtures::build_executable_project;

#[test]
#[cfg(not(windows))]
fn prove_fails_when_execution_output_not_found() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("prove")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Proving hello
            error: execution directory not found: [..]/target/execute/hello/execution1
            help: make sure to run `scarb execute` first
            and then run `scarb prove` with correct execution ID

        "#});
}

#[test]
#[cfg(not(windows))]
fn prove_fails_when_cairo_pie_output() {
    let t = build_executable_project();

    // First create a cairo pie output
    Scarb::quick_command()
        .arg("execute")
        .arg("--target=bootloader")
        .arg("--output=cairo-pie")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/execute/hello/execution1/cairo_pie.zip")
        .assert(predicates::path::exists());

    // Then try to prove it
    Scarb::quick_command()
        .arg("prove")
        .arg("--execution-id=1")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Proving hello
            error: proving cairo pie output is not supported: [..]/target/execute/hello/execution1/cairo_pie.zip
            help: run `scarb execute --output=standard` first
            and then run `scarb prove` with correct execution ID

        "#});
}

#[test]
#[cfg(windows)]
fn prove_fails_on_windows() {
    let t = build_executable_project();

    Scarb::quick_command()
        .arg("prove")
        .arg("--execute")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: `scarb prove` is not supported on Windows
            help: use WSL or a Linux/macOS machine instead

        "#});
}
