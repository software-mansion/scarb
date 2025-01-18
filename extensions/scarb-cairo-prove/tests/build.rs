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
        .arg("cairo-execute")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_snapbox()
        .arg("cairo-prove")
        .arg("--execution=1")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Proving hello
        Saving proof to: target/scarb-execute/hello/execution1/proof/proof.json
        "#});

    t.child("target/scarb-execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

#[test]
fn prove_from_paths() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("cairo-execute")
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_snapbox()
        .arg("cairo-prove")
        .arg("--pub-input-file=target/scarb-execute/hello/execution1/air_public_input.json")
        .arg("--priv-input-file=target/scarb-execute/hello/execution1/air_private_input.json")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Proving Cairo program
        Saving proof to: proof.json
        "#});

    t.child("proof.json").assert(predicates::path::exists());
}

#[test]
fn prove_with_track_relations() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("cairo-execute")
        .current_dir(&t)
        .assert()
        .success();

    let cmd = Scarb::quick_snapbox()
        .arg("cairo-prove")
        .arg("--execution=1")
        .arg("--track-relations")
        .current_dir(&t)
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(stdout.contains("Proving hello"));
    assert!(stdout.contains("Relations summary:"));
    assert!(
        stdout.contains("Saving proof to: target/scarb-execute/hello/execution1/proof/proof.json")
    );

    t.child("target/scarb-execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

#[test]
fn prove_with_display_components() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("cairo-execute")
        .current_dir(&t)
        .assert()
        .success();

    let cmd = Scarb::quick_snapbox()
        .arg("cairo-prove")
        .arg("--execution=1")
        .arg("--display-components")
        .current_dir(&t)
        .assert()
        .success();

    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(stdout.contains("Proving hello"));
    assert!(stdout.contains("CairoComponents"));
    assert!(
        stdout.contains("Saving proof to: target/scarb-execute/hello/execution1/proof/proof.json")
    );

    t.child("target/scarb-execute/hello/execution1/proof/proof.json")
        .assert(predicates::path::exists());
}

#[test]
fn prove_fails_when_execution_output_not_found() {
    let t = build_executable_project();

    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-prove")
            .arg("--execution=1")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Proving hello
        error: execution directory not found: [..]/target/scarb-execute/hello/execution1
        help: make sure to run `scarb cairo-execute` first
        and that the execution ID is correct

        "#},
    )
}

#[test]
fn prove_fails_when_input_files_not_found() {
    let t = build_executable_project();

    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-prove")
            .arg("--pub-input-file=nonexistent.json")
            .arg("--priv-input-file=nonexistent.json")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Proving Cairo program
        error: public input file does not exist at path: nonexistent.json
        "#},
    )
}

#[test]
fn prove_with_execute() {
    let t = build_executable_project();

    Scarb::quick_snapbox()
        .arg("cairo-prove")
        .arg("--execute")
        .arg("--target=standalone")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..])
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/scarb-execute/hello/execution1
        [..]Proving hello
        Saving proof to: target/scarb-execute/hello/execution1/proof/proof.json
        "#});

    t.child("target/scarb-execute/hello/execution1/proof/proof.json")
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
