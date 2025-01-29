use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use predicates::prelude::*;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::predicates::is_file_empty;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::cmd::OutputAssert;

fn executable_project_builder() -> ProjectBuilder {
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
}

fn build_executable_project() -> TempDir {
    let t = TempDir::new().unwrap();
    executable_project_builder().build(&t);
    t
}

#[test]
fn can_execute_default_main_function_from_executable() {
    let t = build_executable_project();
    Scarb::quick_snapbox()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello
        "#});

    t.child("target/execute/hello/air_private_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/air_public_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/memory.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
    t.child("target/execute/hello/trace.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
}

#[test]
fn can_execute_prebuilt_executable() {
    let t = build_executable_project();
    Scarb::quick_snapbox().arg("build").current_dir(&t).assert();
    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Executing hello
        Saving output to: target/execute/hello
        "#});

    t.child("target/execute/hello/air_private_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/air_public_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/memory.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
    t.child("target/execute/hello/trace.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
}

#[test]
fn can_execute_bootloader_target() {
    let t = build_executable_project();
    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--target=bootloader")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello
        "#});

    t.child("target/execute/hello/air_private_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/air_public_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/memory.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
    t.child("target/execute/hello/trace.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
}

#[test]
fn can_produce_cairo_pie_output() {
    let t = build_executable_project();
    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--target=bootloader")
        .arg("--output=cairo-pie")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello.zip
        "#});

    t.child("target/execute/hello.zip")
        .assert(predicates::path::exists());
}

#[test]
fn fails_when_attr_missing() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
                [executable]
            "#})
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);

    output_assert(
        Scarb::quick_snapbox()
            .arg("execute")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        error: Requested `#[executable]` not found.
        error: could not compile `hello` due to previous error
        error: `scarb metadata` exited with error
        "#},
    );

    output_assert(
        Scarb::quick_snapbox()
            .arg("execute")
            .arg("--no-build")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Executing hello
        error: package has not been compiled, file does not exist: `hello.executable.json`
        help: run `scarb build` to compile the package

        "#},
    );
}

#[test]
fn can_print_panic_reason() {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                panic!("abcd");
                42
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--print-program-output")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Program output:
        1
        Panicked with "abcd".
        Saving output to: target/execute/hello
        "#});
    t.child("target/execute/hello/air_private_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/air_public_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/memory.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
    t.child("target/execute/hello/trace.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
}

fn output_assert(output: OutputAssert, expected: &str) {
    #[cfg(windows)]
    output.stdout_matches(format!(
        "{expected}error: process did not exit successfully: exit code: 1\n"
    ));
    #[cfg(not(windows))]
    output.stdout_matches(expected);
}
